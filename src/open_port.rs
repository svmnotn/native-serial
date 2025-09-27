use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam::channel::{after, unbounded, Receiver, Sender};

use crate::types::{Command, PortSettings, SharedTsfn};

#[napi]
pub struct OpenPort {
  // worker thread that owns the port and performs both reads and writes
  worker_thread: Option<thread::JoinHandle<()>>,
  // TSFN shared between JS setter and worker
  tsfn: SharedTsfn,
  // separate TSFN for write errors
  write_error_tsfn: SharedTsfn,
  // sender for control / write commands
  cmd_tx: Sender<Command>,
}

#[napi]
impl OpenPort {
  #[napi]
  pub fn write(&self, data: &[u8]) -> napi::Result<()> {
    self
      .cmd_tx
      .send(Command::Write(data.to_vec()))
      .map_err(|e| napi::Error::from_reason(format!("failed to send write command: {}", e)))?;
    Ok(())
  }

  // settable property: open.onDataReceived = (err, data) => { ... }
  #[napi(js_name = "onDataReceived")]
  pub fn set_on_data_received(
    &self,
    callback: Option<ThreadsafeFunction<Buffer, ()>>,
  ) -> napi::Result<()> {
    // Drop existing TSFN
    {
      let mut guard = self.tsfn.lock().unwrap();
      *guard = None;
    }

    if let Some(tsfn) = callback {
      let mut guard = self.tsfn.lock().unwrap();
      *guard = Some(tsfn);
    }

    Ok(())
  }

  // settable property: open.onWriteError = (err) => { ... }
  #[napi(js_name = "onWriteError")]
  pub fn set_on_write_error(
    &self,
    callback: Option<ThreadsafeFunction<Buffer, ()>>,
  ) -> napi::Result<()> {
    // Drop existing TSFN
    {
      let mut guard = self.write_error_tsfn.lock().unwrap();
      *guard = None;
    }

    if let Some(tsfn) = callback {
      let mut guard = self.write_error_tsfn.lock().unwrap();
      *guard = Some(tsfn);
    }

    Ok(())
  }

  #[napi]
  pub fn close(&mut self) -> napi::Result<()> {
    // Drop TSFN so worker won't call into JS.
    {
      let mut ts = self.tsfn.lock().unwrap();
      *ts = None;
    }

    // Send shutdown command
    let _ = self.cmd_tx.send(Command::Shutdown);

    // Join worker thread
    if let Some(handle) = self.worker_thread.take() {
      let _ = handle.join();
    }

    Ok(())
  }
}

fn apply_builder_settings(
  mut builder: serialport::SerialPortBuilder,
  settings: &PortSettings,
) -> serialport::SerialPortBuilder {
  // data bits
  if let Some(db) = &settings.data_bits {
    let db_enum = match db {
      crate::types::DataBits::Five => serialport::DataBits::Five,
      crate::types::DataBits::Six => serialport::DataBits::Six,
      crate::types::DataBits::Seven => serialport::DataBits::Seven,
      crate::types::DataBits::Eight => serialport::DataBits::Eight,
    };
    builder = builder.data_bits(db_enum);
  }

  // parity
  if let Some(p) = &settings.parity {
    let p_enum = match p {
      crate::types::Parity::None => serialport::Parity::None,
      crate::types::Parity::Odd => serialport::Parity::Odd,
      crate::types::Parity::Even => serialport::Parity::Even,
    };
    builder = builder.parity(p_enum);
  }

  // stop bits
  if let Some(sb) = &settings.stop_bits {
    let sb_enum = match sb {
      crate::types::StopBits::One => serialport::StopBits::One,
      crate::types::StopBits::Two => serialport::StopBits::Two,
    };
    builder = builder.stop_bits(sb_enum);
  }

  // flow control
  if let Some(fc) = &settings.flow_control {
    let fc_enum = match fc {
      crate::types::FlowControl::None => serialport::FlowControl::None,
      crate::types::FlowControl::Software => serialport::FlowControl::Software,
      crate::types::FlowControl::Hardware => serialport::FlowControl::Hardware,
    };
    builder = builder.flow_control(fc_enum);
  }

  builder
}

pub fn open_port(port_path: String, settings: Option<PortSettings>) -> napi::Result<OpenPort> {
  let settings = settings.unwrap_or(PortSettings {
    baud_rate: Some(115_200),
    timeout_ms: Some(10),
    data_bits: Some(crate::types::DataBits::Eight),
    parity: Some(crate::types::Parity::None),
    stop_bits: Some(crate::types::StopBits::One),
    flow_control: Some(crate::types::FlowControl::None),
  });

  let baud = settings.baud_rate.unwrap_or(115_200);
  let timeout_ms = settings.timeout_ms.unwrap_or(10);
  let timeout = Duration::from_millis(timeout_ms as u64);

  let builder = serialport::new(port_path.clone(), baud);
  let builder = apply_builder_settings(builder, &settings).timeout(timeout);

  let sp = builder
    .open()
    .map_err(|e| napi::Error::from_reason(format!("failed to open {}: {}", port_path, e)))?;

  // TSFN holder shared with API setter and worker
  let tsfn_holder: SharedTsfn = Arc::new(Mutex::new(None));
  let write_error_holder: SharedTsfn = Arc::new(Mutex::new(None));

  // command channel for write/shutdown etc.
  let (cmd_tx, cmd_rx): (Sender<Command>, Receiver<Command>) = unbounded();

  // worker thread owns the serial port (no other locks for reads/writes)
  let tsfn_for_thread = tsfn_holder.clone();
  let write_error_for_thread = write_error_holder.clone();

  let handle = thread::spawn(move || {
    let mut port = sp; // owns the serial port
    let mut buf = [0u8; 1024];

    loop {
      // Use crossbeam select with a timeout tick to alternate between handling commands
      // and attempting reads from the port.
      crossbeam::select! {
        recv(cmd_rx) -> msg => {
          match msg {
            Ok(Command::Write(data)) => {
                  if let Err(e) = port.write_all(&data) {
                    // write error: attempt to surface this to JS via the onWriteError TSFN if present
                    if let Some(tsfn) = write_error_for_thread.lock().unwrap().as_ref() {
                      let _ = tsfn.call(
                        Err(napi::Error::from_reason(format!("write failed: {}", e))),
                        ThreadsafeFunctionCallMode::NonBlocking,
                      );
                    }
                // keep the worker alive and continue listening for commands
                continue;
              }
              let _ = port.flush();
            }
            Ok(Command::Shutdown) | Err(_) => {
              // Shutdown requested or sender dropped
              let _ = port.flush();
              break;
            }
          }
        }
        recv(after(timeout)) -> _ => {
          // Time to attempt a read; port is configured with the same timeout but this pattern keeps
          // us responsive to commands.
          match port.read(&mut buf) {
            Ok(n) if n > 0 => {
              let v = &buf[..n];
              if let Some(tsfn) = tsfn_for_thread.lock().unwrap().as_ref() {
                let _ = tsfn.call(Ok(Buffer::from(v)), ThreadsafeFunctionCallMode::NonBlocking);
              }
            }
            Ok(_) => {
              // zero bytes, continue
              continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
              // normal: no data this iteration
              continue;
            }
            Err(_) => {
              // unrecoverable error or port closed -> exit
              break;
            }
          }
        }
      }
    }
  });

  Ok(OpenPort {
    worker_thread: Some(handle),
    tsfn: tsfn_holder,
    write_error_tsfn: write_error_holder,
    cmd_tx,
  })
}

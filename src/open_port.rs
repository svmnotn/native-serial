use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;

use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam::channel::{bounded, unbounded, Receiver, RecvError, Sender};

use crate::types::PortSettings;

pub type OnDataReceivedCallback = ThreadsafeFunction<Buffer, (), Buffer, napi::Status, false>;
pub type OnErrorCallback = ThreadsafeFunction<(), ()>;

#[napi]
pub struct OpenPort {
  // thread handles, wrapped in an option so we can join them without having to take self without reference
  read_thread: Option<thread::JoinHandle<()>>,
  write_thread: Option<thread::JoinHandle<()>>,
  // sender for writes
  write_tx: Sender<Buffer>,
  // sender is wrapped in an option so we can drop it without having to take self without reference
  kill_tx: Option<Sender<()>>,
}

#[napi]
impl OpenPort {
  #[napi]
  pub fn write(&self, data: Buffer) -> napi::Result<()> {
    self
      .write_tx
      .send(data)
      .map_err(|e| napi::Error::from_reason(format!("failed to send write to thread: {e}")))
  }

  #[napi]
  pub fn close(&mut self) -> napi::Result<()> {
    // Close the send side of the write channel to signal the threads to exit
    drop(self.kill_tx.take());

    // Join worker thread
    if let Some(handle) = self.write_thread.take() {
      let _ = handle.join();
    }

    if let Some(handle) = self.read_thread.take() {
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

// Make the opened port non-exclusive on platforms that support it.
// On Unix-like platforms the underlying TTY port supports `set_exclusive`.
// On Windows this is a no-op because the COM port implementation doesn't expose it.
#[cfg(unix)]
fn make_port_nonexclusive(port: &mut serialport::TTYPort, path: &str) -> napi::Result<()> {
  port.set_exclusive(false).map_err(|e| {
    napi::Error::from_reason(format!("failed to make the port {path} not exclusive: {e}"))
  })
}

// No-op on Windows
#[cfg(windows)]
fn make_port_nonexclusive(_: &mut serialport::COMPort, _: &str) -> napi::Result<()> {
  Ok(())
}

pub fn open_port(
  path: &str,
  on_data_received: ThreadsafeFunction<Buffer, (), Buffer, napi::Status, false>,
  on_error: ThreadsafeFunction<(), ()>,
  settings: Option<PortSettings>,
) -> napi::Result<OpenPort> {
  let settings = settings.unwrap_or(PortSettings {
    baud_rate: Some(115_200),
    timeout_ms: Some(10),
    data_bits: Some(crate::types::DataBits::Eight),
    parity: Some(crate::types::Parity::None),
    stop_bits: Some(crate::types::StopBits::One),
    flow_control: Some(crate::types::FlowControl::None),
  });

  let baud = settings.baud_rate.unwrap_or(115_200);
  let timeout = Duration::from_millis(settings.timeout_ms.unwrap_or(10) as u64);

  let builder = serialport::new(path, baud);
  let builder = apply_builder_settings(builder, &settings).timeout(timeout);

  let mut read_port = builder
    .open_native()
    .map_err(|e| napi::Error::from_reason(format!("failed to open: {e}")))?;

  make_port_nonexclusive(&mut read_port, path)?;

  let mut write_port = read_port
    .try_clone_native()
    .map_err(|e| napi::Error::from_reason(format!("failed to clone port: {e}")))?;

  // command channel for write/shutdown etc.
  let (kill_tx, kill_rx_read): (Sender<()>, Receiver<()>) = bounded(0);
  let kill_rx_write = kill_rx_read.clone();

  let (write_tx, write_rx): (Sender<Buffer>, Receiver<Buffer>) = unbounded();

  let on_error = Arc::new(on_error);
  let read_on_error = on_error.clone();
  let write_on_error = on_error;

  let read_handle = thread::spawn(move || {
    loop {
      crossbeam::select! {
        // Shutdown requested
        recv(kill_rx_read) -> _ => break,
        default() => {
          let mut buf = [0u8; 1024];
          match read_port.read(&mut buf) {
            Ok(n) if n > 0 => {
              let _ = on_data_received.call(Buffer::from(&buf[..n]), ThreadsafeFunctionCallMode::Blocking);
            }
            // zero bytes, continue
            Ok(_) => continue,
            // normal: no data this iteration
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            // unrecoverable error or port closed -> exit
            Err(e) => {
              let _ = read_on_error.call(Err(napi::Error::from_reason(format!("read thread died due to {e}"))), ThreadsafeFunctionCallMode::NonBlocking);
              break;
            }
          }
        }
      }
    }
  });

  let write_handle = thread::spawn(move || {
    loop {
      crossbeam::select! {
        // Shutdown requested
        recv(kill_rx_write) -> _ => break,
        // Write data
        recv(write_rx) -> msg => {
          match msg {
            Ok(data) => {
              if let Err(e) = write_port.write_all(&data) {
                let _ = write_on_error.call(Err(napi::Error::from_reason(format!("failed to write: {e}"))), ThreadsafeFunctionCallMode::NonBlocking);
                continue;
              }
            }
            // channel closed, exit
            Err(RecvError) => {
              let _ = write_on_error.call(Err(napi::Error::from_reason(format!("write channel closed?!"))), ThreadsafeFunctionCallMode::NonBlocking);
              break;
            }
          }
        }
      }
    }
  });

  Ok(OpenPort {
    kill_tx: Some(kill_tx),
    read_thread: Some(read_handle),
    write_thread: Some(write_handle),
    write_tx,
  })
}

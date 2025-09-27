use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;

use std::sync::{Arc, Mutex};

#[napi]
pub enum DataBits {
  Five,
  Six,
  Seven,
  Eight,
}

#[napi]
pub enum Parity {
  None,
  Odd,
  Even,
}

#[napi]
pub enum StopBits {
  One,
  Two,
}

#[napi]
pub enum FlowControl {
  None,
  Software,
  Hardware,
}

#[napi(object)]
pub struct PortSettings {
  pub baud_rate: Option<u32>,
  /// read timeout in ms
  pub timeout_ms: Option<u32>,
  pub data_bits: Option<DataBits>,
  pub parity: Option<Parity>,
  pub stop_bits: Option<StopBits>,
  pub flow_control: Option<FlowControl>,
}

// Commands sent to the single-threaded worker that owns the serial port
pub enum Command {
  Write(Vec<u8>),
  Shutdown,
}

pub type SharedTsfn = Arc<Mutex<Option<ThreadsafeFunction<Buffer, ()>>>>;

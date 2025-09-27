use napi::bindgen_prelude::ToNapiValue;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;

use std::sync::{Arc, Mutex};

#[napi(string_enum)]
pub enum DataBits {
  Five,
  Six,
  Seven,
  Eight,
}

#[napi(string_enum)]
pub enum Parity {
  None,
  Odd,
  Even,
}

#[napi(string_enum)]
pub enum StopBits {
  One,
  Two,
}

#[napi(string_enum)]
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

// A small struct to surface USB-specific fields from SerialPortType::UsbPort
#[derive(Clone)]
#[napi(object)]
pub struct UsbInfo {
  #[napi(readonly)]
  pub vid: u16,
  #[napi(readonly)]
  pub pid: u16,
  #[napi(readonly)]
  pub serial: Option<String>,
  #[napi(readonly)]
  pub manufacturer: Option<String>,
  #[napi(readonly)]
  pub product: Option<String>,
}

impl ToNapiValue for &mut UsbInfo {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    let env_wrapper = napi::Env::from(env);
    let mut obj = napi::bindgen_prelude::Object::new(&env_wrapper)?;
    let UsbInfo {
      vid,
      pid,
      serial,
      manufacturer,
      product,
    } = val;
    obj.set("vid", vid)?;
    obj.set("pid", pid)?;
    if let Some(s) = serial {
      obj.set("serial", s)?;
    }
    if let Some(s) = manufacturer {
      obj.set("manufacturer", s)?;
    }
    if let Some(s) = product {
      obj.set("product", s)?;
    }
    napi::bindgen_prelude::Object::to_napi_value(env, obj)
  }
}

pub type SharedTsfn<T> = Arc<Mutex<Option<ThreadsafeFunction<T, ()>>>>;

use crate::open_port::open_port;
use crate::types::PortSettings;
use napi_derive::napi;
use serialport::{SerialPortInfo, SerialPortType};

// A small struct to surface USB-specific fields from SerialPortType::UsbPort
#[derive(Clone)]
#[napi]
pub struct UsbInfo {
  vid: u16,
  pid: u16,
  serial: Option<String>,
  manufacturer: Option<String>,
  product: Option<String>,
}

#[napi]
impl UsbInfo {
  #[napi(getter)]
  pub fn vid(&self) -> u16 {
    self.vid
  }

  #[napi(getter)]
  pub fn pid(&self) -> u16 {
    self.pid
  }

  #[napi(getter)]
  pub fn serial(&self) -> Option<String> {
    self.serial.clone()
  }

  #[napi(getter)]
  pub fn manufacturer(&self) -> Option<String> {
    self.manufacturer.clone()
  }

  #[napi(getter)]
  pub fn product(&self) -> Option<String> {
    self.product.clone()
  }
}

#[napi]
pub struct Port {
  path: String,
  port_type: String,
  usb_info: Option<UsbInfo>,
}

#[napi]
impl Port {
  // keep compatibility with existing `path()` accessor
  #[napi(getter)]
  pub fn path(&self) -> String {
    self.path.clone()
  }

  #[napi(getter, js_name = "type")]
  pub fn port_type(&self) -> String {
    self.port_type.clone()
  }

  #[napi(getter, js_name = "usbInfo")]
  pub fn usb_info(&self) -> Option<UsbInfo> {
    self.usb_info.clone()
  }

  // Instance method: open the port and return an OpenPort (the worker)
  #[napi]
  pub fn open(&self, settings: Option<PortSettings>) -> napi::Result<crate::open_port::OpenPort> {
    open_port(self.path.clone(), settings)
  }
}

#[napi]
pub fn list_ports() -> napi::Result<Vec<Port>> {
  let ports = serialport::available_ports()
    .map_err(|e| napi::Error::from_reason(format!("list_ports failed: {}", e)))?;

  let result = ports.into_iter().map(serial_info_to_port).collect();

  Ok(result)
}

fn serial_info_to_port(p: SerialPortInfo) -> Port {
  let (port_type, usb) = match p.port_type {
    SerialPortType::UsbPort(ref info) => {
      let usb_info = UsbInfo {
        vid: info.vid,
        pid: info.pid,
        serial: info.serial_number.clone(),
        manufacturer: info.manufacturer.clone(),
        product: info.product.clone(),
      };
      ("Usb".to_string(), Some(usb_info))
    }
    SerialPortType::BluetoothPort => ("Bluetooth".to_string(), None),
    SerialPortType::PciPort => ("Pci".to_string(), None),
    SerialPortType::Unknown => ("Unknown".to_string(), None),
  };

  Port {
    path: p.port_name,
    port_type,
    usb_info: usb,
  }
}

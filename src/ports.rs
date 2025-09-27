use crate::open_port::open_port;
use crate::types::{PortSettings, UsbInfo};
use napi_derive::napi;
use serialport::{SerialPortInfo, SerialPortType};

#[napi]
pub struct AvailablePort {
  #[napi(readonly)]
  pub path: String,
  #[napi(readonly, js_name = "type")]
  pub port_type: String,
  #[napi(readonly, js_name = "usb")]
  pub usb_info: Option<UsbInfo>,
}

#[napi]
impl AvailablePort {
  #[napi]
  pub fn open(&self, settings: Option<PortSettings>) -> napi::Result<crate::open_port::OpenPort> {
    open_port(&self.path, settings)
  }
}

#[napi]
pub fn list_ports() -> napi::Result<Vec<AvailablePort>> {
  let ports = serialport::available_ports()
    .map_err(|e| napi::Error::from_reason(format!("list_ports failed: {}", e)))?;

  Ok(ports.into_iter().map(serial_info_to_port).collect())
}

fn serial_info_to_port(p: SerialPortInfo) -> AvailablePort {
  let (port_type, usb_info) = match p.port_type {
    SerialPortType::UsbPort(info) => {
      let usb_info = UsbInfo {
        vid: info.vid,
        pid: info.pid,
        serial: info.serial_number,
        manufacturer: info.manufacturer,
        product: info.product,
      };
      ("Usb".to_string(), Some(usb_info))
    }
    SerialPortType::BluetoothPort => ("Bluetooth".to_string(), None),
    SerialPortType::PciPort => ("Pci".to_string(), None),
    SerialPortType::Unknown => ("Unknown".to_string(), None),
  };

  AvailablePort {
    path: p.port_name,
    port_type,
    usb_info,
  }
}

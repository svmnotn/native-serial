// Library entry: re-export modules and public items

pub mod open_port;
pub mod ports;
pub mod types;

pub use open_port::OpenPort;
pub use ports::list_ports;
pub use ports::Port;
pub use types::{DataBits, FlowControl, Parity, PortSettings, StopBits};

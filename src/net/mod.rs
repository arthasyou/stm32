pub mod codec;
pub mod connection;
pub mod packet;
pub mod router;
pub mod tcp_server;
pub mod serial_transport;

// 重新导出常用类型
pub use codec::{CodecError, DecodedPacket, PacketCodec};
pub use connection::TcpError;
pub use packet::{Packet, PacketError, PacketHeader, PacketType};
pub use router::{example_handler, Router};
pub use tcp_server::{TcpServer, TcpServerConfig};
pub use serial_transport::{SerialTransport, SerialTransportConfig};

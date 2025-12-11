pub mod codec;
pub mod connection;
pub mod events;
pub mod manager;
pub mod packet;
pub mod router;
pub mod tcp_server;

// 重新导出常用类型
pub use codec::{CodecError, DecodedPacket, PacketCodec};
pub use connection::{Connection, ConnectionId, TcpError};
pub use events::TcpEvent;
pub use manager::{start_manager_loop, ConnectionManager};
pub use packet::{Packet, PacketError, PacketHeader, PacketType};
pub use router::Router;
pub use tcp_server::{TcpEventChannel, TcpServer, TcpServerConfig};

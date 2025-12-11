// TCP 事件定义
use super::connection::{Connection, ConnectionId};
use defmt::Format;
use heapless::Vec;

/// TCP 事件最大载荷大小
pub const MAX_EVENT_PAYLOAD: usize = 512;

/// TCP 事件
#[derive(Format)]
pub enum TcpEvent {
    /// 握手事件（连接ID，连接对象）
    Handshake(ConnectionId, Connection),

    /// 断开连接事件
    Disconnect(ConnectionId),

    /// 广播消息事件
    Broadcast {
        /// 错误码
        error_code: u16,
        /// 命令
        cmd: u16,
        /// 消息载荷
        message: Option<Vec<u8, MAX_EVENT_PAYLOAD>>,
        /// 目标连接ID列表（None表示广播给所有连接）
        connection_ids: Option<Vec<u32, 16>>,
    },
}

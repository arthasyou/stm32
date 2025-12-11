// 连接管理器
use super::{
    connection::{Connection, ConnectionId},
    events::TcpEvent,
    tcp_server::TcpEventChannel,
};
use defmt::{info, warn};
use heapless::{FnvIndexMap, Vec};

/// 最大连接数
const MAX_CONNECTIONS: usize = 8;

/// 错误码
pub const SUCCESS: u16 = 0;
pub const SYSTEM_ERROR: u16 = 1;

/// 连接管理器
pub struct ConnectionManager {
    connections: FnvIndexMap<u32, Connection, MAX_CONNECTIONS>,
    max_clients: usize,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub const fn new() -> Self {
        Self {
            connections: FnvIndexMap::new(),
            max_clients: MAX_CONNECTIONS,
        }
    }

    /// 添加连接
    pub fn add_connection(&mut self, conn: Connection) -> Result<(), ()> {
        if self.connections.len() >= self.max_clients {
            warn!("Too many connections");
            return Err(());
        }

        info!("Adding connection {}", conn.id.0);
        self.connections.insert(conn.id.0, conn).ok();
        info!("Total connections: {}", self.connections.len());

        Ok(())
    }

    /// 移除连接
    pub fn remove_connection(&mut self, id: ConnectionId) {
        if self.connections.remove(&id.0).is_some() {
            info!("Connection {} removed", id.0);
        } else {
            warn!("Connection {} not found", id.0);
        }
        info!("Total connections: {}", self.connections.len());
    }

    /// 获取连接数量
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// 检查连接是否存在
    pub fn has_connection(&self, id: ConnectionId) -> bool {
        self.connections.contains_key(&id.0)
    }
}

/// 管理器事件循环
pub async fn start_manager_loop(event_channel: &'static TcpEventChannel) {
    let mut manager = ConnectionManager::new();

    info!("Connection manager started");

    loop {
        let event = event_channel.receive().await;

        match event {
            TcpEvent::Handshake(conn_id, conn) => {
                info!("Handshake from connection {}", conn_id.0);
                if manager.add_connection(conn).is_err() {
                    warn!("Failed to add connection {}", conn_id.0);
                }
            }

            TcpEvent::Disconnect(conn_id) => {
                info!("Disconnect from connection {}", conn_id.0);
                manager.remove_connection(conn_id);
            }

            TcpEvent::Broadcast {
                error_code,
                cmd,
                message,
                connection_ids,
            } => {
                info!(
                    "Broadcast: error_code={}, cmd={}, targets={:?}",
                    error_code, cmd, connection_ids
                );

                // 注意：在当前架构中，broadcast 需要通过其他机制实现
                // 因为我们没有保存每个连接的发送通道
                // 这里仅作为事件记录
                warn!("Broadcast not fully implemented in this architecture");
            }
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

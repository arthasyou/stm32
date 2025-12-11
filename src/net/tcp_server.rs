// TCP 服务器主体
use super::{
    connection::{handle_connection, ConnectionId},
    events::TcpEvent,
    manager,
    router::Router,
};
use defmt::{error, info, warn};
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

/// TCP 事件通道容量
const EVENT_CHANNEL_SIZE: usize = 16;

/// TCP 缓冲区大小
pub const RX_BUFFER_SIZE: usize = 2048;
pub const TX_BUFFER_SIZE: usize = 2048;

/// TCP 事件通道（用于连接与管理器之间通信）
pub type TcpEventChannel = Channel<CriticalSectionRawMutex, TcpEvent, EVENT_CHANNEL_SIZE>;

/// TCP 服务器配置
#[derive(Clone, Copy)]
pub struct TcpServerConfig {
    /// 监听端口
    pub port: u16,
    /// 接收超时
    pub recv_timeout: Duration,
}

impl Default for TcpServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            recv_timeout: Duration::from_secs(30),
        }
    }
}

/// TCP 服务器
pub struct TcpServer {
    config: TcpServerConfig,
}

impl TcpServer {
    /// 创建新的 TCP 服务器
    pub const fn new(config: TcpServerConfig) -> Self {
        Self { config }
    }

    /// 启动 TCP 服务器
    pub async fn start<'d>(
        &self,
        stack: &'static Stack<'d>,
        event_channel: &'static TcpEventChannel,
        router: &'static Router,
    ) -> ! {
        info!("Starting TCP server on port {}", self.config.port);

        let mut next_conn_id: u32 = 1;

        loop {
            // 等待网络就绪
            while !stack.is_link_up() {
                warn!("Network link down, waiting...");
                Timer::after(Duration::from_secs(1)).await;
            }

            // 显示本地 IP
            if let Some(config) = stack.config_v4() {
                info!("Network ready: IP={:?}", config.address);
            }

            // 使用 StaticCell 管理缓冲区（Rust 2024 安全方式）
            static RX_BUF: StaticCell<[u8; RX_BUFFER_SIZE]> = StaticCell::new();
            static TX_BUF: StaticCell<[u8; TX_BUFFER_SIZE]> = StaticCell::new();

            let rx_buf = RX_BUF.init([0; RX_BUFFER_SIZE]);
            let tx_buf = TX_BUF.init([0; TX_BUFFER_SIZE]);

            let mut socket = TcpSocket::new(*stack, rx_buf, tx_buf);
            socket.set_timeout(Some(self.config.recv_timeout));

            info!("Listening on port {}", self.config.port);
            if let Err(e) = socket.accept(self.config.port).await {
                error!("Accept error: {:?}", e);
                Timer::after(Duration::from_secs(1)).await;
                continue;
            }

            let remote = socket.remote_endpoint();
            info!("New client connected: {:?}", remote);

            let conn_id = ConnectionId(next_conn_id);
            next_conn_id = next_conn_id.wrapping_add(1);

            // 处理连接（这里需要生成新任务，但 embassy 的任务池有限）
            // 简化版本：同步处理连接（一次只处理一个连接）
            if let Err(e) = handle_connection(socket, conn_id, event_channel, router).await {
                warn!("Connection {} error: {:?}", conn_id.0, e);
            }

            info!("Client {} disconnected", conn_id.0);

            // 短暂延迟后继续接受新连接
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}

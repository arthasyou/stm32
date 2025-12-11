// TCP 服务器 - 只接受单个客户端连接
use super::{connection::handle_connection, router::Router};
use defmt::{error, info, warn};
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

/// TCP 缓冲区大小
pub const RX_BUFFER_SIZE: usize = 4096;
pub const TX_BUFFER_SIZE: usize = 4096;

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

/// TCP 服务器（只处理单个连接）
pub struct TcpServer {
    config: TcpServerConfig,
}

impl TcpServer {
    /// 创建新的 TCP 服务器
    pub const fn new(config: TcpServerConfig) -> Self {
        Self { config }
    }

    /// 启动 TCP 服务器（只接受一个连接）
    pub async fn start<'d>(
        &self,
        stack: &'static Stack<'d>,
        router: &'static Router,
    ) -> ! {
        info!("Starting TCP server on port {} (single connection mode)", self.config.port);

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

            // 使用 StaticCell 管理缓冲区
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
            info!("Client connected: {:?}", remote);

            // 处理连接（阻塞直到断开）
            if let Err(e) = handle_connection(socket, router).await {
                warn!("Connection error: {:?}", e);
            }

            info!("Client disconnected, waiting for new connection...");

            // 短暂延迟后继续接受新连接
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}

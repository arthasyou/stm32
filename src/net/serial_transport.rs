// Serial Transport - Event Producer（与 TCP Server 并列的传输层）
//
// 职责：
// 1. 从串口读取已完整的应用层字节流（硬件已完成 TCP 重组、校验）
// 2. 使用 PacketCodec 解码应用层协议包
// 3. 将解码结果封装为 Event::NetworkIncoming
// 4. 通过 Event Channel 注入事件系统
//
// ⚠️  不做：CRC/校验、丢包处理、重传、确认、窗口控制（硬件已完成）

use super::codec::PacketCodec;
use super::packet::PacketType;
use crate::event::Event;
use alloc::vec::Vec;
use byteorder::{BigEndian, ByteOrder};
use defmt::{debug, error, info, warn};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};

/// Serial Transport 配置
#[derive(Clone, Copy)]
pub struct SerialTransportConfig {
    /// 读取超时
    pub read_timeout: Duration,
    /// 是否启用 mock 模式（用于 Demo）
    pub mock_mode: bool,
}

impl Default for SerialTransportConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(30),
            mock_mode: true, // Demo 模式默认开启
        }
    }
}

/// Serial Transport（串口传输层）
pub struct SerialTransport {
    config: SerialTransportConfig,
}

impl SerialTransport {
    /// 创建新的 Serial Transport
    pub const fn new(config: SerialTransportConfig) -> Self {
        Self { config }
    }

    /// 启动 Serial Transport（Event Producer）
    ///
    /// # 参数
    /// - `event_tx`: Event Channel 的发送端，用于注入 NetworkIncoming 事件
    ///
    /// # 架构说明
    /// 这是一个 Event Producer，与 tcp_server 并列：
    /// - tcp_server: 从 TcpSocket 读取 → 解码 → 产生 Event::NetworkIncoming
    /// - serial_transport: 从串口读取 → 解码 → 产生 Event::NetworkIncoming
    ///
    /// 上层系统（dispatch_task + router + handlers）对传输方式完全无感
    pub async fn start(
        &self,
        event_tx: Sender<'static, CriticalSectionRawMutex, Event, 32>,
    ) -> ! {
        info!("Starting Serial Transport (Event Producer mode)");

        if self.config.mock_mode {
            info!("⚠️  Running in MOCK mode (for Demo)");
        }

        let mut codec = PacketCodec::new();
        let mut decode_buffer = [0u8; 1024];

        loop {
            // ========== 第一步：从串口读取字节流 ==========
            //
            // 📌 硬件语义：
            // - USB 转网口芯片已完成 TCP/IP 协议栈
            // - 串口收到的数据 = TCP socket 中的应用层 payload
            // - 已保证：顺序、完整性、可靠性
            //
            // 🔧 接入真实硬件时，替换以下两个函数：
            // - mock_serial_read() → 真实的 uart.read()
            // - mock_serial_available() → 真实的 uart.poll() / interrupt

            let rx_data = if self.config.mock_mode {
                // Demo: 模拟串口读取
                match self.mock_serial_read().await {
                    Some(data) => data,
                    None => {
                        Timer::after(Duration::from_millis(100)).await;
                        continue;
                    }
                }
            } else {
                // TODO: 接入真实串口驱动
                // 示例：
                // let mut rx_buffer = [0u8; 512];
                // match uart.read(&mut rx_buffer).await {
                //     Ok(n) if n > 0 => &rx_buffer[..n],
                //     _ => continue,
                // }
                error!("Real UART not implemented yet");
                Timer::after(Duration::from_secs(1)).await;
                continue;
            };

            debug!("Serial received {} bytes", rx_data.len());

            // ========== 第二步：使用已有 codec 解码 ==========
            // 与 tcp_server/connection.rs 中的逻辑完全一致

            if let Err(e) = codec.feed(rx_data) {
                warn!("Codec feed error: {:?}", e);
                continue;
            }

            // 尝试解码数据包
            while let Ok(Some(packet)) = codec.decode(&mut decode_buffer) {
                debug!(
                    "Decoded packet: type={:?}, seq={}, len={}",
                    packet.packet_type,
                    packet.seq,
                    packet.payload.len()
                );

                // 跳过 Ping（由 tcp_server 处理，这里暂不响应）
                if packet.packet_type == PacketType::Ping {
                    debug!("Received Ping (ignored in serial mode)");
                    continue;
                }

                // ========== 第三步：提取 cmd + payload ==========
                // 协议格式：[cmd: 2 bytes][payload: variable]

                if packet.payload.len() < 2 {
                    warn!("Packet payload too short");
                    continue;
                }

                let cmd = BigEndian::read_u16(&packet.payload[0..2]);
                let payload_data = &packet.payload[2..];

                // 转换为 alloc::vec::Vec（Event 需要）
                let mut payload_vec = Vec::new();
                payload_vec.extend_from_slice(payload_data);

                // ========== 第四步：注入 Event::NetworkIncoming ==========
                // 🎯 关键点：与 tcp_server 产生相同的 Event 类型
                // 上层系统（dispatch → router → handlers）完全无感

                let event = Event::NetworkIncoming {
                    cmd,
                    payload: payload_vec,
                };

                debug!("Injecting NetworkIncoming event: cmd={:04X}", cmd);

                event_tx.send(event).await;
            }
        }
    }

    // ========== Mock 实现（Demo 用） ==========
    // 🔧 接入真实硬件时，删除以下函数，使用真实的 UART API

    /// Mock: 模拟串口读取
    ///
    /// 真实替换路径：
    /// ```rust
    /// // 使用 embassy_stm32::usart::Uart
    /// let mut uart = Uart::new(/* ... */);
    /// let mut rx_buffer = [0u8; 512];
    /// match uart.read(&mut rx_buffer).await {
    ///     Ok(n) => &rx_buffer[..n],
    ///     Err(_) => continue,
    /// }
    /// ```
    async fn mock_serial_read(&self) -> Option<&'static [u8]> {
        // Demo: 模拟接收一个 Command 包
        // 实际数据格式：
        // - PacketHeader (8 bytes): magic + type + seq + len + checksum
        // - Payload: [cmd: 0x2001][data: ...]

        Timer::after(Duration::from_secs(5)).await;

        // 构造一个示例 Command 包（cmd=0x2001, 请求状态）
        // 这里直接返回预编码的字节流
        static MOCK_DATA: &[u8] = &[
            // PacketHeader (8 bytes)
            0xAA, 0x55,       // magic
            0x20,             // PacketType::Command
            0x01,             // seq
            0x00, 0x02,       // payload_len = 2
            0x00, 0x00,       // checksum (需要计算)
            // Payload (2 bytes)
            0x20, 0x01,       // cmd = 0x2001 (Request Status)
        ];

        // 计算并修正 checksum（这里简化，直接使用示例数据）
        // 真实环境中，数据来自硬件，已经包含正确的 checksum

        info!("Mock: Simulating serial data reception");
        Some(MOCK_DATA)
    }
}

// ========== 架构说明文档（代码内嵌） ==========

// # Serial Transport vs TCP Server 职责对照
//
// ## TCP Server (src/net/tcp_server.rs)
// ```text
// TcpSocket::read()
//   ↓
// PacketCodec::feed() + decode()
//   ↓
// 提取 cmd + payload
//   ↓
// 直接调用 router.handle_message()  ← 不走 Event Channel
// ```
//
// ## Serial Transport (src/net/serial_transport.rs)
// ```text
// uart.read() / mock_serial_read()
//   ↓
// PacketCodec::feed() + decode()  ← 完全相同
//   ↓
// 提取 cmd + payload              ← 完全相同
//   ↓
// event_tx.send(NetworkIncoming)  ← 标准 Event Producer
//   ↓
// dispatch_task → route_event → handlers::network
// ```
//
// ## 关键差异
// - **数据源**：TcpSocket vs UART（但语义等价：都是应用层 payload）
// - **事件注入**：tcp_server 直接调用 handler，serial_transport 走 Event Channel
// - **协议处理**：完全相同（PacketCodec + cmd 解析）
//
// ## 未来真实硬件接入
//
// 只需修改 `start()` 函数中的串口读取部分：
//
// ```rust
// // 1. 初始化 UART（main.rs 中）
// let uart = Uart::new(
//     p.USART1,
//     p.PA10,  // RX
//     p.PA9,   // TX
//     Irqs,
//     p.DMA1_CH4,
//     p.DMA1_CH5,
//     uart_config,
// );
//
// // 2. 替换 mock_serial_read()
// let mut rx_buffer = [0u8; 512];
// let n = uart.read(&mut rx_buffer).await?;
// let rx_data = &rx_buffer[..n];
//
// // 后续流程保持不变（codec、event 注入）
// ```
//
// ## 系统启动配置
//
// 在 `main.rs` 中，选择一种传输方式启动：
//
// ```rust
// // 选项 1: TCP 模式
// spawner.spawn(tcp_server_task(stack, router)).unwrap();
//
// // 选项 2: Serial 模式
// let serial_transport = SerialTransport::new(Default::default());
// spawner.spawn(serial_transport_task(serial_transport, event_tx)).unwrap();
// ```

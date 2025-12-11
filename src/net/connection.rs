// TCP 连接处理
use super::{
    codec::{CodecError, PacketCodec},
    events::TcpEvent,
    packet::PacketType,
    router::Router,
    tcp_server::TcpEventChannel,
};
use byteorder::{BigEndian, ByteOrder};
use defmt::{debug, error, info, warn, Format};
use embassy_net::tcp::TcpSocket;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use heapless::Vec;

/// 消息通道容量
const MSG_CHANNEL_SIZE: usize = 4;

/// 消息类型：(error_code, cmd, payload)
type Msg = (u16, u16, Option<Vec<u8, 512>>);

/// 消息通道
pub type MsgChannel = Channel<CriticalSectionRawMutex, Msg, MSG_CHANNEL_SIZE>;

/// 连接 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub struct ConnectionId(pub u32);

/// 连接对象
#[derive(Format)]
pub struct Connection {
    pub id: ConnectionId,
    // 注意：由于 no_std 限制，我们不能像 tokio 那样使用 Sender
    // 这里简化为只存储 ID，通过管理器来发送消息
}

impl Connection {
    pub fn new(id: ConnectionId) -> Self {
        Self { id }
    }
}

/// TCP 错误
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum TcpError {
    Disconnected,
    SendFailed,
    RecvTimeout,
    CodecError(CodecError),
    ChannelFull,
    Other,
}

impl From<CodecError> for TcpError {
    fn from(e: CodecError) -> Self {
        TcpError::CodecError(e)
    }
}

/// 处理 TCP 连接
pub async fn handle_connection<'a>(
    mut socket: TcpSocket<'a>,
    conn_id: ConnectionId,
    event_channel: &'static TcpEventChannel,
    router: &'static Router,
) -> Result<(), TcpError> {
    info!("Handling connection {}", conn_id.0);

    // 创建消息通道（用于发送响应）
    static MSG_CHAN: MsgChannel = Channel::new();

    // 创建连接对象
    let connection = Connection::new(conn_id);

    // 发送握手事件到管理器
    event_channel
        .send(TcpEvent::Handshake(conn_id, connection))
        .await;

    info!("Connection {} handshake sent", conn_id.0);

    // 处理消息
    if let Err(e) = handle_messages(&mut socket, conn_id, &MSG_CHAN, router, event_channel).await {
        warn!("Connection {} error: {:?}", conn_id.0, e);
    }

    // 发送断开连接事件
    event_channel.send(TcpEvent::Disconnect(conn_id)).await;

    info!("Connection {} closed", conn_id.0);

    Ok(())
}

/// 处理接收到的消息
async fn handle_messages<'a>(
    socket: &mut TcpSocket<'a>,
    conn_id: ConnectionId,
    msg_channel: &'static MsgChannel,
    router: &'static Router,
    event_channel: &'static TcpEventChannel,
) -> Result<(), TcpError> {
    let mut codec = PacketCodec::new();
    let mut rx_buffer = [0u8; 512];
    let mut decode_buffer = [0u8; 1024];

    loop {
        // 从 socket 读取数据
        let n = match socket.read(&mut rx_buffer).await {
            Ok(0) => {
                info!("Connection {} closed by peer", conn_id.0);
                return Err(TcpError::Disconnected);
            }
            Ok(n) => n,
            Err(e) => {
                error!("Socket read error: {:?}", e);
                return Err(TcpError::Other);
            }
        };

        debug!("Connection {} received {} bytes", conn_id.0, n);

        // 喂给编解码器
        if let Err(e) = codec.feed(&rx_buffer[..n]) {
            warn!("Codec feed error: {:?}", e);
            continue;
        }

        // 尝试解码数据包
        while let Ok(Some(packet)) = codec.decode(&mut decode_buffer) {
            info!(
                "Connection {} decoded packet: type={:?}, seq={}, len={}",
                conn_id.0,
                packet.packet_type,
                packet.seq,
                packet.payload.len()
            );

            // 处理 Ping（自动响应 Pong）
            if packet.packet_type == PacketType::Ping {
                debug!("Connection {} received Ping, sending Pong", conn_id.0);
                if let Err(e) = send_pong(socket).await {
                    warn!("Failed to send Pong: {:?}", e);
                }
                continue;
            }

            // 解析命令（使用简单的 cmd 格式：2字节cmd + payload）
            if packet.payload.len() >= 2 {
                let cmd = BigEndian::read_u16(&packet.payload[0..2]);
                let payload_data = &packet.payload[2..];

                // 创建 payload Vec
                let mut payload_vec = Vec::new();
                if payload_vec.extend_from_slice(payload_data).is_err() {
                    warn!("Payload too large");
                    continue;
                }

                debug!("Connection {} processing cmd={}", conn_id.0, cmd);

                // 路由处理消息
                match router.handle_message(cmd, payload_vec, event_channel).await {
                    Ok(response_data) => {
                        // 发送响应
                        if let Err(e) = send_response(socket, 0, cmd, Some(response_data)).await {
                            warn!("Failed to send response: {:?}", e);
                        }
                    }
                    Err(_) => {
                        // 发送错误响应
                        if let Err(e) = send_response(socket, 1, cmd, None).await {
                            warn!("Failed to send error response: {:?}", e);
                        }
                    }
                }
            } else {
                warn!("Packet payload too short");
            }
        }
    }
}

/// 发送 Pong 响应
async fn send_pong(socket: &mut TcpSocket<'_>) -> Result<(), TcpError> {
    let mut tx_buffer = [0u8; 8]; // 只需要头部
    let len = PacketCodec::encode_simple(PacketType::Pong, 0, &mut tx_buffer)
        .map_err(TcpError::from)?;

    match socket.write(&tx_buffer[..len]).await {
        Ok(n) if n == len => {}
        _ => return Err(TcpError::SendFailed),
    }

    Ok(())
}

/// 发送响应
async fn send_response(
    socket: &mut TcpSocket<'_>,
    error_code: u16,
    cmd: u16,
    payload: Option<Vec<u8, 512>>,
) -> Result<(), TcpError> {
    // 构建响应数据：error_code(2) + cmd(2) + payload
    let mut response = Vec::<u8, 1024>::new();

    // 添加 error_code 和 cmd
    let mut header = [0u8; 4];
    BigEndian::write_u16(&mut header[0..2], error_code);
    BigEndian::write_u16(&mut header[2..4], cmd);

    if response.extend_from_slice(&header).is_err() {
        return Err(TcpError::Other);
    }

    // 添加 payload
    if let Some(data) = payload {
        if response.extend_from_slice(&data).is_err() {
            return Err(TcpError::Other);
        }
    }

    // 使用 Command 类型的数据包发送
    let mut tx_buffer = [0u8; 1024 + 8];
    let len = PacketCodec::encode(PacketType::Response, 0, &response, &mut tx_buffer)
        .map_err(TcpError::from)?;

    match socket.write(&tx_buffer[..len]).await {
        Ok(n) if n == len => {}
        _ => return Err(TcpError::SendFailed),
    }

    info!("Response sent: error_code={}, cmd={}", error_code, cmd);

    Ok(())
}

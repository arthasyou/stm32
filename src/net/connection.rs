// TCP 连接处理（单个连接，简化版）
use super::{
    codec::{CodecError, PacketCodec},
    packet::PacketType,
    router::Router,
};
use byteorder::{BigEndian, ByteOrder};
use defmt::{debug, error, info, warn, Format};
use embassy_net::tcp::TcpSocket;
use heapless::Vec;

/// TCP 错误
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum TcpError {
    Disconnected,
    SendFailed,
    CodecError(CodecError),
    Other,
}

impl From<CodecError> for TcpError {
    fn from(e: CodecError) -> Self {
        TcpError::CodecError(e)
    }
}

/// 处理 TCP 连接（直接在这里处理消息）
pub async fn handle_connection<'a>(
    mut socket: TcpSocket<'a>,
    router: &'static Router,
) -> Result<(), TcpError> {
    info!("Handling connection");

    let mut codec = PacketCodec::new();
    let mut rx_buffer = [0u8; 512];
    let mut decode_buffer = [0u8; 1024];

    loop {
        // 从 socket 读取数据
        let n = match socket.read(&mut rx_buffer).await {
            Ok(0) => {
                info!("Connection closed by peer");
                return Err(TcpError::Disconnected);
            }
            Ok(n) => n,
            Err(e) => {
                error!("Socket read error: {:?}", e);
                return Err(TcpError::Other);
            }
        };

        debug!("Received {} bytes", n);

        // 喂给编解码器
        if let Err(e) = codec.feed(&rx_buffer[..n]) {
            warn!("Codec feed error: {:?}", e);
            continue;
        }

        // 尝试解码数据包
        while let Ok(Some(packet)) = codec.decode(&mut decode_buffer) {
            info!(
                "Decoded packet: type={:?}, seq={}, len={}",
                packet.packet_type,
                packet.seq,
                packet.payload.len()
            );

            // 处理 Ping（自动响应 Pong）
            if packet.packet_type == PacketType::Ping {
                debug!("Received Ping, sending Pong");
                if let Err(e) = send_pong(&mut socket).await {
                    warn!("Failed to send Pong: {:?}", e);
                }
                continue;
            }

            // 解析命令（cmd 格式：2字节cmd + payload）
            if packet.payload.len() >= 2 {
                let cmd = BigEndian::read_u16(&packet.payload[0..2]);
                let payload_data = &packet.payload[2..];

                // 创建 payload Vec
                let mut payload_vec = Vec::new();
                if payload_vec.extend_from_slice(payload_data).is_err() {
                    warn!("Payload too large");
                    continue;
                }

                debug!("Processing cmd={}", cmd);

                // 路由处理消息
                match router.handle_message(cmd, payload_vec) {
                    Ok(response_data) => {
                        // 发送响应
                        if let Err(e) = send_response(&mut socket, 0, cmd, Some(response_data)).await {
                            warn!("Failed to send response: {:?}", e);
                        }
                    }
                    Err(_) => {
                        // 发送错误响应
                        if let Err(e) = send_response(&mut socket, 1, cmd, None).await {
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
        Ok(n) if n == len => Ok(()),
        _ => Err(TcpError::SendFailed),
    }
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

    // 使用 Response 类型的数据包发送
    let mut tx_buffer = [0u8; 1024 + 8];
    let len = PacketCodec::encode(PacketType::Response, 0, &response, &mut tx_buffer)
        .map_err(TcpError::from)?;

    match socket.write(&tx_buffer[..len]).await {
        Ok(n) if n == len => {
            info!("Response sent: error_code={}, cmd={}", error_code, cmd);
            Ok(())
        }
        _ => Err(TcpError::SendFailed),
    }
}

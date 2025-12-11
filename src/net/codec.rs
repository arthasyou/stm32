// 编解码（以后加 protobuf 放这里）
use super::packet::{Packet, PacketError, PacketHeader, PacketType, HEADER_LEN, MAX_PAYLOAD_LEN};
use defmt::{debug, warn, Format};
use heapless::Vec;

/// 编解码器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
enum CodecState {
    /// 等待头部
    WaitingHeader,
    /// 等待载荷
    WaitingPayload { header: PacketHeader },
}

/// 数据包编解码器
pub struct PacketCodec {
    state: CodecState,
    buffer: Vec<u8, { HEADER_LEN + MAX_PAYLOAD_LEN }>,
}

impl PacketCodec {
    /// 创建新的编解码器
    pub fn new() -> Self {
        Self {
            state: CodecState::WaitingHeader,
            buffer: Vec::new(),
        }
    }

    /// 重置编解码器
    pub fn reset(&mut self) {
        self.state = CodecState::WaitingHeader;
        self.buffer.clear();
    }

    /// 向缓冲区添加数据
    pub fn feed(&mut self, data: &[u8]) -> Result<(), CodecError> {
        for &byte in data {
            if self.buffer.push(byte).is_err() {
                warn!("Codec buffer overflow, resetting");
                self.reset();
                return Err(CodecError::BufferOverflow);
            }
        }
        Ok(())
    }

    /// 尝试解码一个完整的数据包
    pub fn decode<'a>(
        &mut self,
        output_buf: &'a mut [u8],
    ) -> Result<Option<DecodedPacket<'a>>, CodecError> {
        loop {
            match self.state {
                CodecState::WaitingHeader => {
                    // 需要至少 HEADER_LEN 字节才能解析头部
                    if self.buffer.len() < HEADER_LEN {
                        return Ok(None);
                    }

                    // 解析头部
                    match PacketHeader::from_bytes(&self.buffer[..HEADER_LEN]) {
                        Ok(header) => {
                            // 检查载荷长度是否合理
                            if header.payload_len as usize > MAX_PAYLOAD_LEN {
                                warn!("Payload too large: {}", header.payload_len);
                                self.reset();
                                return Err(CodecError::PayloadTooLarge);
                            }

                            debug!("Header decoded: type={:?}, seq={}, len={}",
                                   header.packet_type, header.seq, header.payload_len);

                            // 移除头部数据
                            self.buffer.as_mut_slice().copy_within(HEADER_LEN.., 0);
                            self.buffer.truncate(self.buffer.len() - HEADER_LEN);

                            // 转换状态
                            self.state = CodecState::WaitingPayload { header };
                        }
                        Err(e) => {
                            warn!("Invalid header: {:?}", e);
                            // 丢弃第一个字节，继续寻找有效头部
                            if self.buffer.len() > 1 {
                                self.buffer.as_mut_slice().copy_within(1.., 0);
                                self.buffer.truncate(self.buffer.len() - 1);
                            } else {
                                self.buffer.clear();
                            }
                            return Err(CodecError::InvalidHeader(e));
                        }
                    }
                }

                CodecState::WaitingPayload { header } => {
                    let payload_len = header.payload_len as usize;

                    // 检查是否收到完整的载荷
                    if self.buffer.len() < payload_len {
                        return Ok(None);
                    }

                    // 检查输出缓冲区大小
                    if output_buf.len() < payload_len {
                        self.reset();
                        return Err(CodecError::OutputBufferTooSmall);
                    }

                    // 复制载荷到输出缓冲区
                    output_buf[..payload_len].copy_from_slice(&self.buffer[..payload_len]);

                    // 创建数据包并验证
                    let packet = Packet {
                        header,
                        payload: &output_buf[..payload_len],
                    };

                    if let Err(e) = packet.verify() {
                        warn!("Packet verification failed: {:?}", e);
                        self.reset();
                        return Err(CodecError::InvalidPacket(e));
                    }

                    debug!("Packet decoded successfully: type={:?}, seq={}",
                           header.packet_type, header.seq);

                    // 移除载荷数据
                    if self.buffer.len() > payload_len {
                        self.buffer.as_mut_slice().copy_within(payload_len.., 0);
                        self.buffer.truncate(self.buffer.len() - payload_len);
                    } else {
                        self.buffer.clear();
                    }

                    // 重置状态
                    self.state = CodecState::WaitingHeader;

                    // 返回解码的数据包信息
                    return Ok(Some(DecodedPacket {
                        packet_type: header.packet_type,
                        seq: header.seq,
                        payload: &output_buf[..payload_len],
                    }));
                }
            }
        }
    }

    /// 编码数据包到缓冲区
    pub fn encode(
        packet_type: PacketType,
        seq: u8,
        payload: &[u8],
        output: &mut [u8],
    ) -> Result<usize, CodecError> {
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(CodecError::PayloadTooLarge);
        }

        let total_len = HEADER_LEN + payload.len();
        if output.len() < total_len {
            return Err(CodecError::OutputBufferTooSmall);
        }

        // 创建数据包
        let packet = Packet::new(packet_type, seq, payload);

        // 写入头部
        let header_bytes = packet.header.to_bytes();
        output[..HEADER_LEN].copy_from_slice(&header_bytes);

        // 写入载荷
        output[HEADER_LEN..total_len].copy_from_slice(payload);

        debug!("Packet encoded: type={:?}, seq={}, len={}", packet_type, seq, payload.len());

        Ok(total_len)
    }

    /// 编码简单响应（无载荷）
    pub fn encode_simple(
        packet_type: PacketType,
        seq: u8,
        output: &mut [u8],
    ) -> Result<usize, CodecError> {
        Self::encode(packet_type, seq, &[], output)
    }
}

/// 解码后的数据包
#[derive(Debug, Format)]
pub struct DecodedPacket<'a> {
    pub packet_type: PacketType,
    pub seq: u8,
    pub payload: &'a [u8],
}

/// 编解码错误
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum CodecError {
    /// 缓冲区溢出
    BufferOverflow,
    /// 载荷太大
    PayloadTooLarge,
    /// 输出缓冲区太小
    OutputBufferTooSmall,
    /// 无效的头部
    InvalidHeader(PacketError),
    /// 无效的数据包
    InvalidPacket(PacketError),
}

impl Default for PacketCodec {
    fn default() -> Self {
        Self::new()
    }
}

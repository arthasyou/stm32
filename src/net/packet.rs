// 协议包格式（先用简单头部）
use defmt::Format;

/// 数据包头部长度（字节）
pub const HEADER_LEN: usize = 8;

/// 数据包最大载荷长度
pub const MAX_PAYLOAD_LEN: usize = 1024;

/// 数据包类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
#[repr(u8)]
pub enum PacketType {
    /// 心跳/Ping
    Ping = 0x01,
    /// Pong响应
    Pong = 0x02,
    /// 按键事件
    Button = 0x10,
    /// 通用命令
    Command = 0x20,
    /// 响应
    Response = 0x21,
    /// 错误
    Error = 0xFF,
}

impl PacketType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Ping),
            0x02 => Some(Self::Pong),
            0x10 => Some(Self::Button),
            0x20 => Some(Self::Command),
            0x21 => Some(Self::Response),
            0xFF => Some(Self::Error),
            _ => None,
        }
    }
}

/// 数据包头部
#[derive(Debug, Clone, Copy, Format, PartialEq, Eq)]
pub struct PacketHeader {
    /// 魔数 0xAA55
    pub magic: u16,
    /// 包类型
    pub packet_type: PacketType,
    /// 序列号
    pub seq: u8,
    /// 载荷长度
    pub payload_len: u16,
    /// 校验和（简单累加）
    pub checksum: u16,
}

impl PacketHeader {
    pub const MAGIC: u16 = 0xAA55;

    /// 创建新的数据包头部
    pub fn new(packet_type: PacketType, seq: u8, payload_len: u16) -> Self {
        Self {
            magic: Self::MAGIC,
            packet_type,
            seq,
            payload_len,
            checksum: 0,
        }
    }

    /// 从字节数组解析头部
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PacketError> {
        if bytes.len() < HEADER_LEN {
            return Err(PacketError::InvalidLength);
        }

        let magic = u16::from_be_bytes([bytes[0], bytes[1]]);
        if magic != Self::MAGIC {
            return Err(PacketError::InvalidMagic);
        }

        let packet_type = PacketType::from_u8(bytes[2])
            .ok_or(PacketError::InvalidType)?;

        let seq = bytes[3];
        let payload_len = u16::from_be_bytes([bytes[4], bytes[5]]);
        let checksum = u16::from_be_bytes([bytes[6], bytes[7]]);

        Ok(Self {
            magic,
            packet_type,
            seq,
            payload_len,
            checksum,
        })
    }

    /// 将头部序列化为字节数组
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[0..2].copy_from_slice(&self.magic.to_be_bytes());
        bytes[2] = self.packet_type as u8;
        bytes[3] = self.seq;
        bytes[4..6].copy_from_slice(&self.payload_len.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    /// 计算校验和
    pub fn calculate_checksum(&self, payload: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        // 头部字段（不包括checksum）
        sum += self.magic as u32;
        sum += self.packet_type as u32;
        sum += self.seq as u32;
        sum += self.payload_len as u32;

        // 载荷数据
        for &byte in payload {
            sum += byte as u32;
        }

        // 折叠为16位
        while sum > 0xFFFF {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        sum as u16
    }

    /// 验证校验和
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        self.checksum == self.calculate_checksum(payload)
    }
}

/// 完整的数据包
#[derive(Debug, Clone, Format)]
pub struct Packet<'a> {
    pub header: PacketHeader,
    pub payload: &'a [u8],
}

impl<'a> Packet<'a> {
    /// 创建新数据包
    pub fn new(packet_type: PacketType, seq: u8, payload: &'a [u8]) -> Self {
        let mut header = PacketHeader::new(packet_type, seq, payload.len() as u16);
        header.checksum = header.calculate_checksum(payload);

        Self { header, payload }
    }

    /// 验证数据包
    pub fn verify(&self) -> Result<(), PacketError> {
        if self.header.magic != PacketHeader::MAGIC {
            return Err(PacketError::InvalidMagic);
        }

        if self.payload.len() != self.header.payload_len as usize {
            return Err(PacketError::InvalidLength);
        }

        if !self.header.verify_checksum(self.payload) {
            return Err(PacketError::InvalidChecksum);
        }

        Ok(())
    }
}

/// 数据包错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum PacketError {
    /// 无效的魔数
    InvalidMagic,
    /// 无效的包类型
    InvalidType,
    /// 无效的长度
    InvalidLength,
    /// 校验和错误
    InvalidChecksum,
    /// 缓冲区太小
    BufferTooSmall,
}

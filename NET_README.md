# Embassy STM32 TCP 服务器实现

## 概述

这个项目为 STM32F407ZG 实现了一个基于 embassy-net 的 TCP 服务器，包含完整的数据包协议、编解码和连接管理功能。

## 目录结构

```
src/net/
├── mod.rs          # 模块导出
├── packet.rs       # 数据包格式定义
├── codec.rs        # 编解码器
└── tcp.rs          # TCP 服务器/客户端
```

## 功能特性

### 1. 数据包协议 (packet.rs)

- **固定头部格式**（8字节）:
  ```
  | Magic (2B) | Type (1B) | Seq (1B) | Length (2B) | Checksum (2B) |
  ```
- **魔数**: 0xAA55
- **数据包类型**:
  - `Ping (0x01)`: 心跳请求
  - `Pong (0x02)`: 心跳响应
  - `Button (0x10)`: 按键事件
  - `Command (0x20)`: 通用命令
  - `Response (0x21)`: 响应
  - `Error (0xFF)`: 错误
- **校验和**: 头部+载荷的累加和验证
- **最大载荷**: 1024 字节

### 2. 编解码器 (codec.rs)

- **状态机解码**: 自动处理数据包边界
- **错误恢复**: 遇到无效数据自动重新同步
- **零拷贝**: 使用 heapless Vec 避免动态分配
- **缓冲管理**: 自动处理不完整数据包

主要 API:
```rust
let mut codec = PacketCodec::new();

// 接收数据
codec.feed(data)?;

// 解码数据包
if let Some(packet) = codec.decode(&mut output_buf)? {
    // 处理数据包
}

// 编码数据包
let len = PacketCodec::encode(PacketType::Button, seq, payload, &mut buf)?;
```

### 3. TCP 服务器/客户端 (tcp.rs)

#### TCP 服务器特性:
- **自动接受连接**: 循环接受新连接
- **自动重连**: 连接断开后自动等待新连接
- **超时管理**: 可配置接收超时
- **Ping/Pong 自动处理**: 自动响应心跳包
- **错误恢复**: 网络断开时自动等待恢复

#### TCP 客户端特性:
- **自动重连**: 连接断开后自动重连
- **可配置重连延迟**: 默认 5 秒
- **网络状态检测**: 等待网络就绪后再连接

## 使用方法

### 服务器端示例

```rust
#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{Stack, Config, StackResources};
use embassy_stm32::eth::{Ethernet, PacketQueue};
use embassy_time::Duration;
use static_cell::StaticCell;

mod net;
use net::{TcpServer, TcpServerConfig, PacketType, TcpError};

// 网络栈资源
static STACK: StaticCell<Stack<Ethernet<'static, ETH, GenericSMI>>> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<Ethernet<'static, ETH, GenericSMI>>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn tcp_server_task(stack: &'static Stack<Ethernet<'static, ETH, GenericSMI>>) -> ! {
    static mut RX_BUF: [u8; 2048] = [0; 2048];
    static mut TX_BUF: [u8; 2048] = [0; 2048];

    let config = TcpServerConfig {
        port: 8080,
        recv_timeout: Duration::from_secs(30),
    };

    let server = TcpServer::new(config);

    server.run(
        stack,
        unsafe { &mut RX_BUF },
        unsafe { &mut TX_BUF },
        |packet, conn| {
            match packet.packet_type {
                PacketType::Button => {
                    info!("Button pressed: {:?}", packet.payload);
                    // 发送响应
                    let response = b"Button received";
                    conn.send_packet(PacketType::Response, response).await?;
                }
                PacketType::Command => {
                    info!("Command received: {:?}", packet.payload);
                    // 处理命令...
                    conn.send_simple(PacketType::Response).await?;
                }
                _ => {
                    info!("Unknown packet type: {:?}", packet.packet_type);
                }
            }
            Ok(())
        },
    ).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());

    // 初始化以太网（需要根据你的硬件配置）
    // let eth = Ethernet::new(...);

    // 配置网络栈
    let config = Config::dhcpv4(Default::default());
    // 或使用静态IP:
    // let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
    //     address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 1, 100), 24),
    //     gateway: Some(Ipv4Address::new(192, 168, 1, 1)),
    //     dns_servers: Default::default(),
    // });

    let stack = &*STACK.init(Stack::new(
        eth,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    ));

    // 启动网络任务
    spawner.spawn(net_task(stack)).unwrap();

    // 启动 TCP 服务器
    spawner.spawn(tcp_server_task(stack)).unwrap();

    loop {
        info!("Main loop running...");
        Timer::after(Duration::from_secs(10)).await;
    }
}
```

### 客户端示例

```rust
use net::{TcpClient, PacketType};
use embassy_net::{IpEndpoint, Ipv4Address};

#[embassy_executor::task]
async fn tcp_client_task(stack: &'static Stack</* Driver */>) -> ! {
    static mut RX_BUF: [u8; 2048] = [0; 2048];
    static mut TX_BUF: [u8; 2048] = [0; 2048];

    let server_addr = IpEndpoint::new(
        Ipv4Address::new(192, 168, 1, 100).into(),
        8080,
    );

    let mut client = TcpClient::new(server_addr);
    client.set_reconnect_delay(Duration::from_secs(3));

    client.run(
        stack,
        unsafe { &mut RX_BUF },
        unsafe { &mut TX_BUF },
        |conn| {
            info!("Connected to server");

            // 发送按键事件
            conn.send_packet(PacketType::Button, &[0x01]).await?;

            // 接收响应
            let packet = conn.recv_packet().await?;
            info!("Response: {:?}", packet.payload);

            Ok(())
        },
    ).await
}
```

## 依赖项

```toml
[dependencies]
embassy-executor = { version = "0.9.1", features = ["executor-thread", "arch-cortex-m", "defmt"] }
embassy-stm32 = { version = "0.4.0", features = ["memory-x", "time-driver-any", "unstable-pac", "exti", "stm32f407zg"] }
embassy-time = { version = "0.5", features = ["tick-hz-32_768", "defmt"] }
embassy-net = { version = "0.6", features = ["defmt", "tcp", "dhcpv4", "medium-ethernet"] }
embassy-futures = { version = "0.1" }
cortex-m = { version = "0.7.6", features = ["critical-section-single-core"] }
cortex-m-rt = { version = "0.7.0" }
panic-probe = { version = "1.0", features = ["print-defmt"] }
defmt = { version = "1.0" }
defmt-rtt = { version = "1.0" }
heapless = { version = "0.8" }
static_cell = { version = "2.1" }
```

## 网络硬件配置

### STM32F407ZG 以太网配置

STM32F407ZG 内置以太网 MAC，需要外部 PHY 芯片（如 LAN8720、DP83848 等）:

```rust
use embassy_stm32::eth::{Ethernet, GenericSMI, PacketQueue};
use embassy_stm32::peripherals::ETH;

// 以太网 DMA 描述符和缓冲区
static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();

// 初始化以太网
let eth_int = interrupt::take!(ETH);
let mac_addr = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];

let device = Ethernet::new(
    PACKETS.init(PacketQueue::<4, 4>::new()),
    p.ETH,
    eth_int,
    p.PA1,  // REF_CLK
    p.PA2,  // MDIO
    p.PC1,  // MDC
    p.PA7,  // CRS_DV
    p.PC4,  // RXD0
    p.PC5,  // RXD1
    p.PB11, // TX_EN
    p.PB12, // TXD0
    p.PB13, // TXD1
    GenericSMI::new(0),  // PHY 地址
    mac_addr,
);
```

## 测试工具

### Python 客户端测试脚本

```python
import socket
import struct

def create_packet(packet_type, seq, payload):
    magic = 0xAA55
    payload_len = len(payload)

    # 计算校验和
    checksum = magic + packet_type + seq + payload_len
    for byte in payload:
        checksum += byte
    checksum &= 0xFFFF

    # 构建数据包
    header = struct.pack('>HBBHH', magic, packet_type, seq, payload_len, checksum)
    return header + payload

def parse_packet(data):
    if len(data) < 8:
        return None

    magic, ptype, seq, plen, checksum = struct.unpack('>HBBHH', data[:8])
    if magic != 0xAA55:
        return None

    payload = data[8:8+plen]
    return {'type': ptype, 'seq': seq, 'payload': payload}

# 连接服务器
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(('192.168.1.100', 8080))

# 发送按键事件
packet = create_packet(0x10, 1, b'\x01')  # Button type, seq=1, button_id=1
sock.send(packet)

# 接收响应
response = sock.recv(1024)
parsed = parse_packet(response)
print(f"Response: {parsed}")

sock.close()
```

## 性能优化建议

1. **调整缓冲区大小**: 根据实际数据量调整 RX_BUFFER_SIZE 和 TX_BUFFER_SIZE
2. **使用静态内存**: 避免使用 static mut，改用 StaticCell
3. **并发连接**: 如需支持多个并发连接，可以创建多个 TCP 任务
4. **DMA 优化**: 确保以太网使用 DMA 传输以降低 CPU 负载

## 扩展功能

### 添加 Protobuf 支持

在 `codec.rs` 中添加:

```rust
#[cfg(feature = "protobuf")]
pub fn decode_protobuf<M: prost::Message + Default>(
    payload: &[u8]
) -> Result<M, CodecError> {
    M::decode(payload).map_err(|_| CodecError::InvalidPacket(PacketError::InvalidLength))
}

#[cfg(feature = "protobuf")]
pub fn encode_protobuf<M: prost::Message>(
    msg: &M,
    buf: &mut [u8]
) -> Result<usize, CodecError> {
    let len = msg.encoded_len();
    if buf.len() < len {
        return Err(CodecError::OutputBufferTooSmall);
    }
    msg.encode(buf).map_err(|_| CodecError::PayloadTooLarge)?;
    Ok(len)
}
```

## 故障排查

1. **编译错误**: 确保所有依赖版本正确
2. **连接失败**: 检查网络配置和 PHY 初始化
3. **数据包校验失败**: 验证数据包格式是否正确
4. **内存不足**: 减小缓冲区大小或优化内存使用

## 许可证

根据项目需要添加许可证信息。

# TCP 服务器使用示例

## 基本架构

net 模块提供了完整的 TCP 服务器/客户端实现，包括：
- `packet.rs`: 定义数据包格式（魔数 0xAA55, 类型, 序列号, 载荷, 校验和）
- `codec.rs`: 自动编解码数据包
- `tcp.rs`: TCP 服务器和客户端，支持自动重连

## 使用 TCP 服务器

```rust
use embassy_executor::Spawner;
use embassy_net::{Stack, StackResources};
use embassy_time::Duration;
use static_cell::StaticCell;

mod net;
use net::{TcpServer, TcpServerConfig, PacketType};

#[embassy_executor::task]
async fn net_task(stack: &'static Stack</* 你的驱动 */>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn tcp_server_task(stack: &'static Stack</* 你的驱动 */>) -> ! {
    // TCP 缓冲区
    static mut RX_BUF: [u8; 2048] = [0; 2048];
    static mut TX_BUF: [u8; 2048] = [0; 2048];

    // 服务器配置
    let config = TcpServerConfig {
        port: 8080,
        recv_timeout: Duration::from_secs(30),
    };

    let server = TcpServer::new(config);

    // 运行服务器
    server.run(
        stack,
        unsafe { &mut RX_BUF },
        unsafe { &mut TX_BUF },
        |packet, conn| {
            // 处理接收到的数据包
            match packet.packet_type {
                PacketType::Button => {
                    info!("Button event: {:?}", packet.payload);
                    // 发送响应
                    conn.send_simple(PacketType::Response).await?;
                }
                PacketType::Command => {
                    info!("Command: {:?}", packet.payload);
                    // 处理命令...
                }
                _ => {}
            }
            Ok(())
        },
    ).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // 初始化硬件
    let p = embassy_stm32::init(Default::default());

    // 初始化网络栈（需要根据你的硬件配置）
    // ...

    // 启动网络任务
    spawner.spawn(net_task(stack)).unwrap();

    // 启动 TCP 服务器
    spawner.spawn(tcp_server_task(stack)).unwrap();
}
```

## 使用 TCP 客户端

```rust
use net::{TcpClient, PacketType};
use embassy_net::IpEndpoint;

#[embassy_executor::task]
async fn tcp_client_task(stack: &'static Stack</* 你的驱动 */>) -> ! {
    static mut RX_BUF: [u8; 2048] = [0; 2048];
    static mut TX_BUF: [u8; 2048] = [0; 2048];

    // 服务器地址
    let server_addr = IpEndpoint::new(
        embassy_net::Ipv4Address::new(192, 168, 1, 100).into(),
        8080,
    );

    let client = TcpClient::new(server_addr);

    client.run(
        stack,
        unsafe { &mut RX_BUF },
        unsafe { &mut TX_BUF },
        |conn| {
            // 连接成功后的处理

            // 发送按键事件
            let data = [0x01]; // 按键 ID
            conn.send_packet(PacketType::Button, &data).await?;

            // 接收响应
            let packet = conn.recv_packet().await?;
            info!("Response: {:?}", packet);

            Ok(())
        },
    ).await
}
```

## 数据包格式

```
+--------+--------+--------+--------+--------+--------+--------+--------+
| Magic  | Magic  |  Type  |  Seq   | Length | Length |Checksum|Checksum|
| (0xAA) | (0x55) | (u8)   | (u8)   | (u16)  | (u16)  | (u16)  | (u16)  |
+--------+--------+--------+--------+--------+--------+--------+--------+
|                         Payload (可变长度)                            |
+-----------------------------------------------------------------------+
```

- Magic: 0xAA55 (固定魔数)
- Type: 包类型 (Ping=0x01, Pong=0x02, Button=0x10, Command=0x20, etc.)
- Seq: 序列号 (自动递增)
- Length: 载荷长度
- Checksum: 校验和 (头部+载荷的累加和)
- Payload: 实际数据

## 注意事项

1. **网络栈初始化**: 需要根据你的硬件配置网络驱动（Ethernet, WiFi 等）
2. **静态内存**: TCP 缓冲区使用 static mut，在生产代码中考虑使用 StaticCell
3. **错误处理**: handler 函数返回 Err 时会断开连接
4. **自动 Ping/Pong**: 服务器会自动响应 Ping 包，无需手动处理
5. **重连机制**: 客户端支持自动重连，可配置重连延迟

## 扩展 Protobuf

未来可以在 `codec.rs` 中添加 Protobuf 支持：

```rust
// 在 codec.rs 中添加
pub fn decode_protobuf<M: prost::Message + Default>(
    payload: &[u8]
) -> Result<M, CodecError> {
    M::decode(payload).map_err(|_| CodecError::InvalidPacket)
}
```

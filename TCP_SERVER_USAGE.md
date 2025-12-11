# Embassy STM32 TCP 服务器使用指南

## 项目架构

参考你的 websocket 服务器架构，TCP 服务器实现了以下模块：

```
src/net/
├── tcp.rs          # TCP 服务器主体（类似 server.rs）
├── connection.rs   # 连接处理
├── manager.rs      # 连接管理器
├── router.rs       # 命令路由
├── events.rs       # 事件定义
├── packet.rs       # 数据包格式
├── codec.rs        # 编解码器
└── mod.rs          # 模块导出
```

## 消息格式

### 外层数据包格式（packet.rs）
```
+--------+--------+--------+--------+--------+--------+--------+--------+
| Magic  | Magic  |  Type  |  Seq   | Length | Length |Checksum|Checksum|
| (0xAA) | (0x55) | (u8)   | (u8)   | (u16)  | (u16)  | (u16)  | (u16)  |
+--------+--------+--------+--------+--------+--------+--------+--------+
|                         Payload (可变长度)                            |
+-----------------------------------------------------------------------+
```

### 内层消息格式（在 Payload 中）
```
+--------+--------+---------------------------------------+
|   CMD  |   CMD  |         Business Data                |
| (u16)  | (u16)  |       (可变长度)                       |
+--------+--------+---------------------------------------+
```

完整示例：
1. 外层：PacketType::Command 的数据包
2. 内层Payload：[cmd(2字节) + 业务数据]

## 使用示例

### 1. 创建路由和处理器

```rust
use crate::net::{Router, TcpEventChannel};
use crate::error::Result;
use heapless::Vec;
use defmt::info;

// 定义命令常量
const CMD_PING: u16 = 0x0001;
const CMD_BUTTON: u16 = 0x0010;
const CMD_GET_STATUS: u16 = 0x0020;

// 处理器函数
fn handle_ping(
    _data: Vec<u8, 512>,
    _event_channel: &'static TcpEventChannel,
) -> Result<Vec<u8, 512>> {
    info!("Ping command received");
    let mut response = Vec::new();
    response.push(b'P').ok();
    response.push(b'O').ok();
    response.push(b'N').ok();
    response.push(b'G').ok();
    Ok(response)
}

fn handle_button(
    data: Vec<u8, 512>,
    _event_channel: &'static TcpEventChannel,
) -> Result<Vec<u8, 512>> {
    if let Some(&button_id) = data.first() {
        info!("Button {} pressed", button_id);
    }
    // 返回空响应
    Ok(Vec::new())
}

fn handle_get_status(
    _data: Vec<u8, 512>,
    _event_channel: &'static TcpEventChannel,
) -> Result<Vec<u8, 512>> {
    info!("Get status command");
    let mut response = Vec::new();

    // 构建状态响应
    response.extend_from_slice(&[
        0x01, // 状态码: 运行中
        0x64, // 电量: 100
    ]).ok();

    Ok(response)
}

// 创建路由器
pub fn create_router() -> Router {
    let mut router = Router::new();

    router.add_route(CMD_PING, handle_ping);
    router.add_route(CMD_BUTTON, handle_button);
    router.add_route(CMD_GET_STATUS, handle_get_status);

    router
}
```

### 2. 主程序设置

```rust
#![no_std]
#![no_main]

mod error;
mod net;

use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_stm32::Config as StmConfig;
use embassy_time::Duration;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use net::{
    start_manager_loop, Router, TcpEventChannel, TcpServer, TcpServerConfig,
};

// 静态资源
static STACK: StaticCell<Stack<'static>> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
static EVENT_CHANNEL: TcpEventChannel = TcpEventChannel::new();
static ROUTER: StaticCell<Router> = StaticCell::new();

/// 网络任务
#[embassy_executor::task]
async fn net_task(stack: &'static Stack<'static>) -> ! {
    stack.run().await
}

/// 连接管理器任务
#[embassy_executor::task]
async fn manager_task() -> ! {
    start_manager_loop(&EVENT_CHANNEL).await;
    unreachable!()
}

/// TCP 服务器任务
#[embassy_executor::task]
async fn tcp_server_task(
    stack: &'static Stack<'static>,
    router: &'static Router,
) -> ! {
    let config = TcpServerConfig {
        port: 8080,
        recv_timeout: Duration::from_secs(30),
    };

    let server = TcpServer::new(config);
    server.start(stack, &EVENT_CHANNEL, router).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(StmConfig::default());

    // 初始化以太网（需要根据你的硬件配置）
    // 这里假设你已经配置好了以太网驱动
    // let eth = ...;

    // 配置网络栈
    let config = Config::dhcpv4(Default::default());

    // 或者使用静态 IP：
    // use embassy_net::{Ipv4Address, Ipv4Cidr, StaticConfigV4};
    // let config = Config::ipv4_static(StaticConfigV4 {
    //     address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 1, 100), 24),
    //     gateway: Some(Ipv4Address::new(192, 168, 1, 1)),
    //     dns_servers: Default::default(),
    // });

    // 创建网络栈
    // let stack = &*STACK.init(Stack::new(
    //     eth,
    //     config,
    //     RESOURCES.init(StackResources::new()),
    //     seed,
    // ));

    // 创建路由器
    let router = ROUTER.init(create_router());

    // 启动任务
    // spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(manager_task()).unwrap();
    // spawner.spawn(tcp_server_task(stack, router)).unwrap();

    loop {
        info!("Main loop running...");
        embassy_time::Timer::after(Duration::from_secs(10)).await;
    }
}

// 路由器创建函数（从上面复制）
fn create_router() -> Router {
    // ... (见上文)
    Router::new()
}
```

## 客户端测试示例（Python）

```python
import socket
import struct

# 数据包格式辅助函数
def create_packet(packet_type, seq, payload):
    """创建外层数据包"""
    magic = 0xAA55
    payload_len = len(payload)

    # 计算校验和
    checksum = magic + packet_type + seq + payload_len
    for byte in payload:
        checksum += byte
    checksum &= 0xFFFF

    # 构建头部
    header = struct.pack('>HBBHH', magic, packet_type, seq, payload_len, checksum)
    return header + payload

def create_command(cmd, data=b''):
    """创建内层命令消息"""
    return struct.pack('>H', cmd) + data

# 连接服务器
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(('192.168.1.100', 8080))

# 发送 Ping 命令 (cmd=0x0001)
cmd_payload = create_command(0x0001)
packet = create_packet(0x20, 1, cmd_payload)  # 0x20 = PacketType::Command
sock.send(packet)

# 接收响应
response = sock.recv(1024)
print(f"Response received: {len(response)} bytes")

# 发送按键事件 (cmd=0x0010, button_id=1)
cmd_payload = create_command(0x0010, b'\x01')
packet = create_packet(0x20, 2, cmd_payload)
sock.send(packet)

# 接收响应
response = sock.recv(1024)
print(f"Response received: {len(response)} bytes")

sock.close()
```

## 响应格式

服务器响应格式（在 PacketType::Response 的 Payload 中）：
```
+--------+--------+--------+--------+---------------------------------------+
| Error  | Error  |  CMD   |  CMD   |         Response Data                |
| Code   | Code   | (u16)  | (u16)  |       (可变长度)                       |
| (u16)  | (u16)  |        |        |                                       |
+--------+--------+--------+--------+---------------------------------------+
```

- error_code: 0=成功, 1=失败
- cmd: 原始命令号
- response_data: 处理器返回的数据

## 关键特性

1. **连接管理**: manager.rs 跟踪所有活动连接
2. **命令路由**: router.rs 根据 cmd 字段路由到不同处理器
3. **事件系统**: 通过 TcpEventChannel 实现连接与管理器的通信
4. **自动 Ping/Pong**: connection.rs 自动响应心跳包
5. **错误处理**: 完整的错误码和错误处理机制

## 数据包类型

定义在 `packet.rs`：
- `Ping (0x01)`: 心跳请求（自动响应）
- `Pong (0x02)`: 心跳响应
- `Button (0x10)`: 按键事件
- `Command (0x20)`: 通用命令
- `Response (0x21)`: 响应
- `Error (0xFF)`: 错误

## 注意事项

1. **单连接限制**: 当前实现一次只处理一个连接（简化版本）
2. **静态内存**: 使用 static mut 管理缓冲区，生产环境建议用 StaticCell
3. **同步处理器**: 路由器的处理器是同步函数，适合嵌入式环境
4. **广播功能**: manager.rs 中的广播功能目前未完全实现

## 扩展建议

1. 使用 embassy_executor::task 为每个连接创建独立任务
2. 实现完整的广播功能（需要保存每个连接的发送通道）
3. 添加 Protobuf 支持到 codec.rs
4. 实现连接认证和权限管理

## 与 Websocket 版本的区别

| 特性 | Websocket 版本 | TCP 版本 |
|------|----------------|----------|
| 运行时 | tokio | embassy |
| 环境 | std | no_std |
| 数据结构 | HashMap, Vec | heapless 容器 |
| 异步 | tokio::spawn | embassy tasks |
| 通道 | tokio::mpsc | embassy_sync::channel |
| 处理器 | async trait | 同步函数指针 |

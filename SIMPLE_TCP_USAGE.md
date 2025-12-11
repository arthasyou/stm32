# 简化版 TCP 服务器使用指南

## 架构说明

**MCU 作为 TCP 服务器**，只接受**一个上位机客户端**连接。

```
上位机 (TCP Client) --> MCU (TCP Server, Port 8080)
                            |
                            +-- Router --> Handler 1
                            +-- Router --> Handler 2
                            +-- Router --> Handler N
```

## 特点

- ✅ 单连接模式（节省资源）
- ✅ 自动 Ping/Pong 心跳
- ✅ 命令路由系统
- ✅ 完整的数据包协议
- ✅ 自动重连（上位机断开后可重新连接）

## 使用步骤

### 1. 创建路由器和处理器

```rust
// src/app/handlers/ping.rs
use crate::error::Result;
use heapless::Vec;
use defmt::info;

pub fn handle_ping(_data: Vec<u8, 512>) -> Result<Vec<u8, 512>> {
    info!("Ping command received");
    let mut response = Vec::new();
    response.push(b'P').ok();
    response.push(b'O').ok();
    response.push(b'N').ok();
    response.push(b'G').ok();
    Ok(response)
}

// src/app/handlers/button.rs
pub fn handle_button(data: Vec<u8, 512>) -> Result<Vec<u8, 512>> {
    if let Some(&button_id) = data.first() {
        info!("Button {} pressed", button_id);
    }
    Ok(Vec::new())
}

// src/app/handlers/mod.rs
pub mod ping;
pub mod button;
```

### 2. 在 main.rs 中设置

```rust
#![no_std]
#![no_main]

mod error;
mod net;
mod app;

use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_stm32::Config as StmConfig;
use embassy_time::Duration;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use net::{Router, TcpServer, TcpServerConfig};
use app::handlers;

// 静态资源
static STACK: StaticCell<Stack<'static>> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
static ROUTER: StaticCell<Router> = StaticCell::new();

// 命令定义
const CMD_PING: u16 = 0x0001;
const CMD_BUTTON: u16 = 0x0010;

/// 网络任务
#[embassy_executor::task]
async fn net_task(stack: &'static Stack<'static>) -> ! {
    stack.run().await
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
    server.start(stack, router).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(StmConfig::default());

    // 初始化以太网（根据你的硬件配置）
    // let eth = Ethernet::new(...);

    // 配置网络栈（DHCP 或静态 IP）
    let config = Config::dhcpv4(Default::default());

    // let stack = &*STACK.init(Stack::new(
    //     eth,
    //     config,
    //     RESOURCES.init(StackResources::new()),
    //     seed,
    // ));

    // 创建路由器并注册处理器
    let router = ROUTER.init({
        let mut r = Router::new();
        r.add_route(CMD_PING, handlers::ping::handle_ping);
        r.add_route(CMD_BUTTON, handlers::button::handle_button);
        r
    });

    // 启动任务
    // spawner.spawn(net_task(stack)).unwrap();
    // spawner.spawn(tcp_server_task(stack, router)).unwrap();

    loop {
        info!("Main loop running...");
        embassy_time::Timer::after(Duration::from_secs(10)).await;
    }
}
```

## 消息格式

### 外层数据包（packet.rs）
```
+--------+--------+--------+--------+--------+--------+--------+--------+
| Magic  | Magic  |  Type  |  Seq   | Length | Length |Checksum|Checksum|
| (0xAA) | (0x55) | (u8)   | (u8)   | (u16)  | (u16)  | (u16)  | (u16)  |
+--------+--------+--------+--------+--------+--------+--------+--------+
|                         Payload                                       |
+-----------------------------------------------------------------------+
```

### 内层命令格式（在 Payload 中）
```
+--------+--------+---------------------------------------+
|   CMD  |   CMD  |         Command Data                |
| (u16)  | (u16)  |       (可变长度)                       |
+--------+--------+---------------------------------------+
```

### 响应格式（在 Response 包的 Payload 中）
```
+--------+--------+--------+--------+---------------------------------------+
| Error  | Error  |  CMD   |  CMD   |         Response Data                |
| (u16)  | (u16)  | (u16)  | (u16)  |       (可变长度)                       |
+--------+--------+--------+--------+---------------------------------------+
```

## Python 上位机示例

```python
import socket
import struct
import time

class MCUClient:
    def __init__(self, host, port=8080):
        self.host = host
        self.port = port
        self.sock = None
        self.seq = 0

    def connect(self):
        """连接到 MCU"""
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((self.host, self.port))
        print(f"Connected to MCU at {self.host}:{self.port}")

    def disconnect(self):
        """断开连接"""
        if self.sock:
            self.sock.close()
            self.sock = None

    def create_packet(self, packet_type, payload):
        """创建数据包"""
        magic = 0xAA55
        seq = self.seq
        self.seq = (self.seq + 1) % 256
        payload_len = len(payload)

        # 计算校验和
        checksum = magic + packet_type + seq + payload_len
        for byte in payload:
            checksum += byte
        checksum &= 0xFFFF

        # 构建数据包
        header = struct.pack('>HBBHH', magic, packet_type, seq, payload_len, checksum)
        return header + payload

    def send_command(self, cmd, data=b''):
        """发送命令"""
        # 构建命令载荷
        cmd_payload = struct.pack('>H', cmd) + data

        # 创建 Command 类型的数据包 (0x20)
        packet = self.create_packet(0x20, cmd_payload)

        self.sock.send(packet)
        print(f"Sent command: {cmd:04X}")

    def receive_response(self, timeout=5):
        """接收响应"""
        self.sock.settimeout(timeout)

        # 接收头部
        header = self.sock.recv(8)
        if len(header) < 8:
            return None

        magic, ptype, seq, plen, checksum = struct.unpack('>HBBHH', header)

        # 接收载荷
        payload = b''
        while len(payload) < plen:
            chunk = self.sock.recv(plen - len(payload))
            if not chunk:
                break
            payload += chunk

        # 解析响应（如果是 Response 类型）
        if ptype == 0x21 and len(payload) >= 4:
            error_code, cmd = struct.unpack('>HH', payload[:4])
            response_data = payload[4:]
            return {
                'error_code': error_code,
                'cmd': cmd,
                'data': response_data
            }

        return {'raw': payload}

# 使用示例
if __name__ == '__main__':
    client = MCUClient('192.168.1.100')

    try:
        client.connect()

        # 发送 Ping 命令
        client.send_command(0x0001)
        response = client.receive_response()
        print(f"Response: {response}")

        # 发送按键命令
        client.send_command(0x0010, b'\x01')  # 按键 ID = 1
        response = client.receive_response()
        print(f"Response: {response}")

        # 保持连接
        while True:
            time.sleep(1)

    except KeyboardInterrupt:
        print("\nClosing...")
    finally:
        client.disconnect()
```

## 资源占用

### RAM 使用
- TCP Socket 缓冲区: 4KB (2KB RX + 2KB TX)
- 编解码器缓冲区: ~2.5KB
- 路由表: 根据注册的处理器数量

### 连接特性
- 同时连接数: **1** 个
- 断开自动重连: ✅
- 心跳检测: ✅ (Ping/Pong)
- 接收超时: 30秒（可配置）

## 调试

查看日志输出（使用 defmt）:
```bash
# 使用 probe-rs
cargo run --release

# 或使用 RTT
probe-rs run --chip STM32F407ZG target/thumbv7em-none-eabi/release/stm32
```

## 常见问题

### Q: 如何处理连接断开？
A: 服务器会自动等待新连接，无需手动处理。

### Q: 能否同时连接多个客户端？
A: 不能，当前架构只支持单个连接以节省资源。

### Q: 如何添加新命令？
A: 1) 定义命令常量 2) 实现处理函数 3) 注册到路由器

### Q: 如何修改端口？
A: 修改 `TcpServerConfig { port: 8080 }` 中的端口号。

## 总结

简化后的架构非常适合资源受限的 MCU：
- 单连接模式节省内存
- 无需复杂的事件系统
- 直接的消息处理流程
- 易于理解和维护

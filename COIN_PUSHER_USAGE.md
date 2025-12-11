# 推币机协议使用指南

## 概述

本项目实现了一个完整的推币机（Coin Pusher）协议系统，基于 embassy 异步框架和 TCP 通信。

## 架构

```
上位机 (Python/C++) <--TCP--> MCU (STM32) <--> 硬件设备
                                    |
                                    +-- 马达控制
                                    +-- 灯光控制
                                    +-- 投币检测
                                    +-- 按钮检测
                                    +-- 故障监控
```

## 已实现的功能

### 1. 消息定义 (src/messages.rs)

所有协议消息都定义为 Rust 结构体，使用 `postcard` 进行序列化/反序列化：

#### 命令码映射

**STM32 → 上位机（发送）：**
- `1001` - 心跳 (Heartbeat)
- `1002` - 状态报告 (StatusReport)
- `1003` - 按钮事件 (ButtonEvent)
- `1004` - 投币事件 (CoinInEvent)
- `1005` - 回币计数事件 (PayoutCountEvent)
- `1006` - 故障事件 (FaultEvent)
- `1007` - 命令执行结果 (CommandResult)

**上位机 → STM32（接收并处理）：**
- `2001` - 请求/订阅状态 (RequestStatus)
- `2002` - 灯光控制 (LightCommand)
- `2003` - 马达控制 (MotorCommand)
- `2004` - 故障清除 (ClearFault)
- `2005` - 模拟故障注入 (SimulateFault)

### 2. 命令处理器 (src/handlers/)

- **status.rs** - 处理状态请求，返回完整或增量状态
- **light.rs** - 控制灯光开关和闪烁模式
- **motor.rs** - 控制马达启停、定时运行、计数运行
- **fault.rs** - 清除故障、模拟故障注入

### 3. 路由系统

路由器自动将命令码分发到对应的处理器：

```rust
let mut router = Router::new();
router.add_route(CMD_REQUEST_STATUS, handlers::handle_request_status);
router.add_route(CMD_LIGHT_COMMAND, handlers::handle_light_command);
router.add_route(CMD_MOTOR_COMMAND, handlers::handle_motor_command);
router.add_route(CMD_CLEAR_FAULT, handlers::handle_clear_fault);
router.add_route(CMD_SIMULATE_FAULT, handlers::handle_simulate_fault);
```

## 使用示例

### 在 MCU 端添加新的命令处理器

1. 在 `src/handlers/` 中创建新的处理器：

```rust
// src/handlers/my_handler.rs
use crate::error::Result;
use crate::messages::*;
use defmt::info;
use heapless::Vec;

pub fn handle_my_command(data: Vec<u8, 512>) -> Result<Vec<u8, 512>> {
    info!("Handler: My Command");

    // 解析命令
    let command: MyCommand = postcard::from_bytes(&data)
        .map_err(|_| crate::error::Error::InvalidParameter)?;

    // 处理逻辑
    // ...

    // 返回响应
    let result = CommandResult {
        seq: 0,
        ok: BoolFlag::True,
        error_code: Some(0),
        message: None,
        state_version: Some(1),
    };

    let mut response = Vec::new();
    response.resize_default(512).ok();
    let size = postcard::to_slice(&result, &mut response)
        .map_err(|_| crate::error::Error::BufferFull)?
        .len();
    response.truncate(size);

    Ok(response)
}
```

2. 在 `src/handlers/mod.rs` 中导出：

```rust
pub mod my_handler;
pub use my_handler::*;
```

3. 在 `main.rs` 中注册路由：

```rust
router.add_route(CMD_MY_COMMAND, handlers::handle_my_command);
```

### Python 上位机示例

```python
import socket
import struct
from postcard import encode, decode

class CoinPusherClient:
    def __init__(self, host, port=8080):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((host, port))
        self.seq = 0

    def send_command(self, cmd, payload_data):
        """发送命令"""
        # 使用 postcard 序列化消息
        payload = encode(payload_data)

        # 构建外层数据包（见 SIMPLE_TCP_USAGE.md）
        magic = 0xAA55
        packet_type = 0x20  # Command
        seq = self.seq
        self.seq = (self.seq + 1) % 256

        # 计算校验和
        checksum = magic + packet_type + seq + len(payload) + 2  # +2 for cmd
        for byte in struct.pack('>H', cmd):
            checksum += byte
        for byte in payload:
            checksum += byte
        checksum &= 0xFFFF

        # 构建完整数据包
        header = struct.pack('>HBBHH', magic, packet_type, seq, len(payload) + 2, checksum)
        cmd_bytes = struct.pack('>H', cmd)
        packet = header + cmd_bytes + payload

        self.sock.send(packet)

    def control_light(self, light_id, on):
        """控制灯光"""
        light_cmd = {
            'lights': [{
                'light_id': light_id,
                'on': 1 if on else 2,  # BoolFlag::True=1, False=2
                'pattern': 0
            }]
        }
        self.send_command(2002, light_cmd)

    def control_motor(self, motor_type, command, duration_ms=None):
        """控制马达"""
        motor_cmd = {
            'motor_type': motor_type,
            'command': command,
            'duration_ms': duration_ms,
            'count': None,
            'speed_level': 1
        }
        self.send_command(2003, motor_cmd)

# 使用示例
if __name__ == '__main__':
    client = CoinPusherClient('192.168.1.100')

    # 打开 1 号灯
    client.control_light(1, True)

    # 启动推币马达
    client.control_motor(motor_type=2, command=2)  # Pusher, Start

    # 定时运行上币马达 500ms
    client.control_motor(motor_type=3, command=4, duration_ms=500)  # Feed, RunTime
```

## 硬件集成

### 添加 GPIO 控制

在 handler 中添加实际的硬件控制：

```rust
// src/handlers/light.rs
use embassy_stm32::gpio::{Output, Level};

pub fn handle_light_command(data: Vec<u8, 512>) -> Result<Vec<u8, 512>> {
    let command: LightCommand = postcard::from_bytes(&data)?;

    for light in command.lights.iter() {
        // TODO: 获取对应的 GPIO pin
        // let mut pin = get_light_pin(light.light_id);

        if light.on.to_bool() {
            // pin.set_high();
            info!("Light {} ON", light.light_id);
        } else {
            // pin.set_low();
            info!("Light {} OFF", light.light_id);
        }
    }

    Ok(create_success_response())
}
```

## 网络配置

当你有网络硬件时，在 `main.rs` 中配置：

```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Config::default());

    // 初始化以太网
    let eth = Ethernet::new(
        p.ETH,
        p.PA1, p.PA2, p.PC1, // RMII pins
        // ... 其他引脚
    );

    // 配置网络栈
    let config = Config::dhcpv4(Default::default());
    let stack = &*STACK.init(Stack::new(
        eth,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    ));

    // 创建路由器
    let router = ROUTER.init(setup_router());

    // 启动网络任务
    spawner.spawn(net_task(stack)).unwrap();

    // 启动 TCP 服务器
    let server_config = TcpServerConfig {
        port: 8080,
        recv_timeout: Duration::from_secs(30),
    };
    let server = TcpServer::new(server_config);
    server.start(stack, router).await
}
```

## 调试和测试

### 查看日志

```bash
# 在真实硬件上运行
cargo run --release

# 或使用 probe-rs
probe-rs run --chip STM32F407ZG target/thumbv7em-none-eabi/release/stm32
```

### 单元测试

程序启动时会自动运行测试：
- 测试状态请求处理
- 测试灯光控制命令
- 测试马达控制命令
- 测试未知命令拒绝
- 测试数据包编解码

## 协议特点

1. **类型安全**: 使用 Rust 强类型系统，编译时检查
2. **Zero-copy**: 使用 `heapless` 避免动态内存分配
3. **高效序列化**: `postcard` 比 protobuf 更轻量，适合嵌入式
4. **扩展性强**: 添加新命令只需实现 handler 函数
5. **错误处理**: 完善的 Result 类型和错误码系统

## 文件结构

```
src/
├── main.rs              # 主程序和路由配置
├── error.rs             # 错误定义
├── messages.rs          # 所有协议消息定义
├── handlers/            # 命令处理器
│   ├── mod.rs
│   ├── status.rs        # 状态请求
│   ├── light.rs         # 灯光控制
│   ├── motor.rs         # 马达控制
│   └── fault.rs         # 故障处理
└── net/                 # 网络层
    ├── mod.rs
    ├── packet.rs        # 数据包格式
    ├── codec.rs         # 编解码器
    ├── router.rs        # 命令路由
    ├── tcp_server.rs    # TCP 服务器
    └── connection.rs    # 连接处理
```

## 下一步

1. 根据你的硬件配置 GPIO、PWM、ADC 等外设
2. 实现实际的硬件控制逻辑
3. 添加状态监控和定时上报
4. 实现心跳机制
5. 添加故障检测和自动恢复

## 参考

- 协议定义: `proto/coin_pusher.proto`
- TCP 使用指南: `SIMPLE_TCP_USAGE.md`
- Embassy 文档: https://embassy.dev

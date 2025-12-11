# Protobuf 方案使用指南

## 概述

项目已成功切换到标准 **Protobuf** 方案，使用 `prost` 库自动从 `.proto` 文件生成 Rust 代码。

## 已完成的配置

### 1. 依赖配置 (Cargo.toml)

```toml
[dependencies]
embedded-alloc = "0.6"  # 全局分配器
prost = { version = "0.13", default-features = false, features = ["prost-derive"] }
prost-types = { version = "0.13", default-features = false }

[build-dependencies]
prost-build = "0.13"  # 代码生成
```

### 2. 自动代码生成 (build.rs)

```rust
pub fn main() {
    // ... defmt 配置 ...

    // 从 .proto 文件生成 Rust 代码
    let mut config = prost_build::Config::new();
    config.btree_map(&["."]);  // 为 no_std 使用 BTreeMap

    config
        .compile_protos(&["proto/coin_pusher.proto"], &["proto/"])
        .unwrap();

    println!("cargo:rerun-if-changed=proto/coin_pusher.proto");
}
```

### 3. 全局分配器配置 (main.rs)

```rust
extern crate alloc;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    // 初始化 32KB 堆内存
    {
        use core::mem::MaybeUninit;
        use core::ptr::addr_of_mut;
        const HEAP_SIZE: usize = 32 * 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe {
            let heap_ptr = addr_of_mut!(HEAP_MEM) as *mut u8;
            HEAP.init(heap_ptr as usize, HEAP_SIZE)
        }
    }
    // ...
}
```

## 生成的 Protobuf 消息

代码自动生成在 `target/.../out/coinpusher.v1.rs`，使用方式：

```rust
// 引入生成的代码
pub mod coinpusher {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/coinpusher.v1.rs"));
    }
}

use coinpusher::v1::*;
```

### 消息结构

| Proto 定义 | 生成的 Rust 结构 |
|-----------|----------------|
| `message m_1001_toc` | `struct M1001Toc` |
| `message m_2001_tos` | `struct M2001Tos` |
| `enum BoolFlag` | `enum BoolFlag` |

### 枚举命名规则

**Proto 定义：**
```protobuf
enum BoolFlag {
  BOOL_TRUE   = 1;
  BOOL_FALSE  = 2;
}

enum MotorCommandType {
  MOTOR_CMD_START = 2;
  MOTOR_CMD_STOP  = 3;
}
```

**生成的 Rust：**
```rust
enum BoolFlag {
    BoolTrue = 1,   // BOOL_TRUE -> BoolTrue
    BoolFalse = 2,  // BOOL_FALSE -> BoolFalse
}

enum MotorCommandType {
    MotorCmdStart = 2,  // MOTOR_CMD_START -> MotorCmdStart
    MotorCmdStop = 3,   // MOTOR_CMD_STOP -> MotorCmdStop
}
```

## 使用示例

### 创建和编码消息

```rust
use prost::Message;
use coinpusher::v1::*;

// 创建心跳消息
let heartbeat = M1001Toc {
    uptime_ms: 12345,
    all_ok: BoolFlag::BoolTrue as i32,  // 枚举 -> i32
    error_count: 0,
    state_version: Some(1),
};

// 编码为字节
let mut buf = alloc::vec::Vec::new();
heartbeat.encode(&mut buf).unwrap();

// buf 现在包含 protobuf 编码的数据
```

### 解码消息

```rust
use prost::Message;

// 从字节解码
let heartbeat = M1001Toc::decode(&buf[..]).unwrap();

println!("Uptime: {} ms", heartbeat.uptime_ms);
```

### 在 Handler 中使用

```rust
fn handle_light_command(data: Vec<u8, 512>) -> Result<Vec<u8, 512>> {
    use prost::Message;

    // 解码 protobuf 消息
    let command = M2002Tos::decode(&data[..])
        .map_err(|_| Error::InvalidParameter)?;

    // 处理灯光控制
    for light in command.lights.iter() {
        let is_on = light.on == BoolFlag::BoolTrue as i32;
        info!("Light {}: {}", light.light_id, if is_on { "ON" } else { "OFF" });

        // TODO: 实际硬件控制
    }

    // 创建响应
    let result = M1007Toc {  // CommandResult
        seq: 0,
        ok: BoolFlag::BoolTrue as i32,
        error_code: Some(0),
        message: None,
        state_version: Some(1),
    };

    // 编码响应
    let mut response = alloc::vec::Vec::new();
    result.encode(&mut response).unwrap();

    // 转换为 heapless::Vec
    let mut heapless_vec = heapless::Vec::new();
    heapless_vec.extend_from_slice(&response).ok();

    Ok(heapless_vec)
}
```

## Python 上位机使用标准 Protobuf

现在你可以用任何语言的标准 protobuf 库了！

### 安装

```bash
pip install protobuf
```

### 生成 Python 代码

```bash
protoc --python_out=. proto/coin_pusher.proto
```

### Python 示例

```python
import socket
import struct
from proto import coin_pusher_pb2 as cp

class CoinPusherClient:
    def __init__(self, host, port=8080):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((host, port))
        self.seq = 0

    def _create_packet(self, packet_type, cmd, payload):
        """创建外层数据包"""
        magic = 0xAA55
        seq = self.seq
        self.seq = (self.seq + 1) % 256

        # 内层: cmd (2 bytes) + protobuf payload
        inner = struct.pack('>H', cmd) + payload

        # 计算校验和
        checksum = magic + packet_type + seq + len(inner)
        for byte in inner:
            checksum += byte
        checksum &= 0xFFFF

        # 外层头部
        header = struct.pack('>HBBHH', magic, packet_type, seq, len(inner), checksum)
        return header + inner

    def send_light_command(self, lights):
        """发送灯光控制命令"""
        # 创建 protobuf 消息
        cmd = cp.m_2002_tos()
        for light in lights:
            l = cmd.lights.add()
            l.light_id = light['light_id']
            l.on = cp.BOOL_TRUE if light['on'] else cp.BOOL_FALSE
            if 'pattern' in light:
                l.pattern = light['pattern']

        # 序列化
        payload = cmd.SerializeToString()

        # 发送
        packet = self._create_packet(0x20, 2002, payload)
        self.sock.send(packet)

    def send_motor_command(self, motor_type, command, **kwargs):
        """发送马达控制命令"""
        cmd = cp.m_2003_tos()
        cmd.motor_type = motor_type
        cmd.command = command

        if 'duration_ms' in kwargs:
            cmd.duration_ms = kwargs['duration_ms']
        if 'count' in kwargs:
            cmd.count = kwargs['count']
        if 'speed_level' in kwargs:
            cmd.speed_level = kwargs['speed_level']

        payload = cmd.SerializeToString()
        packet = self._create_packet(0x20, 2003, payload)
        self.sock.send(packet)

    def receive_response(self):
        """接收响应"""
        # 接收外层头部
        header = self.sock.recv(8)
        magic, ptype, seq, plen, checksum = struct.unpack('>HBBHH', header)

        # 接收内层数据
        inner = self.sock.recv(plen)

        # 解析命令码
        cmd = struct.unpack('>H', inner[:2])[0]
        payload = inner[2:]

        # 解码 protobuf 响应
        if cmd == 1007:  # CommandResult
            result = cp.m_1007_toc()
            result.ParseFromString(payload)
            return {
                'seq': result.seq,
                'ok': result.ok == cp.BOOL_TRUE,
                'error_code': result.error_code if result.HasField('error_code') else None,
                'message': result.message if result.HasField('message') else None
            }

        return {'cmd': cmd, 'payload': payload}

# 使用示例
if __name__ == '__main__':
    client = CoinPusherClient('192.168.1.100')

    # 控制灯光
    client.send_light_command([
        {'light_id': 1, 'on': True, 'pattern': 0},
        {'light_id': 2, 'on': False}
    ])

    response = client.receive_response()
    print(f"Response: {response}")

    # 控制马达
    client.send_motor_command(
        motor_type=cp.MOTOR_TYPE_PUSHER,
        command=cp.MOTOR_CMD_START,
        speed_level=1
    )
```

## C++ 上位机

```cpp
#include <iostream>
#include "coin_pusher.pb.h"

// 编译: protoc --cpp_out=. proto/coin_pusher.proto

coinpusher::v1::m_2002_tos create_light_command() {
    coinpusher::v1::m_2002_tos cmd;

    auto* light = cmd.add_lights();
    light->set_light_id(1);
    light->set_on(coinpusher::v1::BOOL_TRUE);
    light->set_pattern(0);

    return cmd;
}

int main() {
    auto cmd = create_light_command();

    // 序列化
    std::string payload;
    cmd.SerializeToString(&payload);

    // 发送 payload...

    return 0;
}
```

## 总结

### ✅ 已实现

- 标准 Protobuf 兼容性
- 自动代码生成
- 支持任何语言的上位机（Python、C++、Java、Go等）
- 32KB 堆内存配置
- 测试代码验证消息编解码

### 📝 下一步

1. 实现完整的命令处理器（handlers）
2. 集成 TCP 服务器
3. 添加硬件控制逻辑
4. 实现心跳和状态上报

### 🔧 开发工作流

1. 修改 `.proto` 文件
2. 运行 `cargo build` 自动重新生成代码
3. 使用生成的结构体和枚举
4. 上位机也重新生成代码（`protoc --python_out=...`）

### 💡 提示

- 枚举值需要转换为 `i32`：`BoolFlag::BoolTrue as i32`
- Optional 字段使用 `Option<T>`
- Repeated 字段使用 `Vec<T>`（需要 alloc）
- 生成的代码在 `target/.../out/` 目录

现在你拥有一个完整的、标准的 Protobuf 通信系统了！

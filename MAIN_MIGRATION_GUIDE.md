# main.rs 迁移说明

## 📋 变更概览

### 文件状态

| 文件 | 大小 | 说明 |
|------|------|------|
| `src/main_original.rs` | 2.2KB | ✅ **原始备份**（仅事件系统，无传输层） |
| `src/main.rs` | 3.4KB | ✅ **新版本**（集成 Serial Transport） |

---

## 🔄 主要变更

### 1. 导入模块变更

#### 原版 (`main_original.rs`)
```rust
// 引入测试需要的模块
use net::{PacketCodec, PacketType, Router};
use heapless::Vec;
```

#### 新版 (`main.rs`)
```rust
// 引入 Serial Transport
use net::{SerialTransport, SerialTransportConfig};
use static_cell::StaticCell;
```

**说明**：移除未使用的测试导入，添加 Serial Transport 所需模块。

---

### 2. 启动信息增强

#### 新增
```rust
info!("Transport Mode: Serial (USB-to-Ethernet via External Chip)");
```

**说明**：明确指示当前使用的传输模式。

---

### 3. 新增 Serial Transport 启动逻辑

**新增代码块**（main.rs:78-106）：

```rust
// ========== 启动 Serial Transport（新增）==========

// 创建 Serial Transport 配置
let serial_config = SerialTransportConfig {
    read_timeout: embassy_time::Duration::from_secs(30),
    mock_mode: true,  // Demo 模式，接入真实硬件时改为 false
};

// 使用 StaticCell 创建静态实例
static SERIAL_TRANSPORT: StaticCell<SerialTransport> = StaticCell::new();
let serial_transport = SERIAL_TRANSPORT.init(SerialTransport::new(serial_config));

// 定义 Serial Transport Task
#[embassy_executor::task]
async fn serial_transport_task(
    transport: &'static SerialTransport,
    event_tx: embassy_sync::channel::Sender<
        'static,
        CriticalSectionRawMutex,
        Event,
        32,
    >,
) -> ! {
    transport.start(event_tx).await
}

// 启动 Serial Transport Task
spawner.spawn(serial_transport_task(serial_transport, event_tx.clone())).unwrap();
info!("  - Serial Transport task spawned (MOCK mode)");
```

**关键点**：
- ✅ 使用 `StaticCell` 创建静态生命周期实例（Embassy 要求）
- ✅ 定义内联 `serial_transport_task`（避免污染全局命名空间）
- ✅ 通过 `event_tx.clone()` 传递 Event Channel 发送端
- ✅ 默认 `mock_mode: true`（Demo 模式）

---

### 4. 系统就绪信息更新

#### 原版
```rust
info!("Event-driven architecture running...");
```

#### 新版
```rust
info!("Event-driven architecture running with Serial Transport...");
info!("Waiting for serial data (mock: every 5 seconds)...");
```

**说明**：提示用户当前 Mock 模式会每 5 秒生成一次测试数据。

---

## 🚀 启动流程对比

### 原版启动流程

```
1. 初始化堆内存
2. 初始化 STM32 外设
3. 创建 Event Channel
4. 启动 button_task
5. 启动 heartbeat_task
6. 启动 dispatch_task
7. 主循环空转
```

### 新版启动流程

```
1. 初始化堆内存
2. 初始化 STM32 外设
3. 创建 Event Channel
4. 启动 button_task
5. 启动 heartbeat_task
6. 启动 dispatch_task
7. ✨ 启动 serial_transport_task（新增）
8. 主循环空转
```

---

## 🔧 如何切换传输模式

### 模式 1: Mock 模式（当前默认）

```rust
let serial_config = SerialTransportConfig {
    read_timeout: embassy_time::Duration::from_secs(30),
    mock_mode: true,  // ← 保持 true
};
```

**行为**：每 5 秒自动生成测试包（cmd=0x2001 Request Status）

---

### 模式 2: 真实 UART 模式

#### 步骤 1: 修改配置

```rust
let serial_config = SerialTransportConfig {
    read_timeout: embassy_time::Duration::from_secs(30),
    mock_mode: false,  // ← 改为 false
};
```

#### 步骤 2: 修改 `serial_transport.rs`

在 `serial_transport.rs:67-77` 替换 UART 读取代码：

```rust
// 替换前（Mock）
let rx_data = if self.config.mock_mode { ... } else { ... }

// 替换后（真实 UART）
let mut rx_buffer = [0u8; 512];
let rx_data = match uart.read(&mut rx_buffer).await {
    Ok(n) => &rx_buffer[..n],
    Err(_) => { Timer::after(Duration::from_millis(100)).await; continue; }
};
```

#### 步骤 3: 初始化 UART（在 main.rs 中 `let _p = ...` 之后）

```rust
use embassy_stm32::usart::{Config as UartConfig, Uart};

let mut uart_config = UartConfig::default();
uart_config.baudrate = 115200;

let uart = Uart::new(
    p.USART1,
    p.PA10,      // RX
    p.PA9,       // TX
    Irqs,
    p.DMA1_CH4,
    p.DMA1_CH5,
    uart_config,
);

// 将 uart 传递给 SerialTransport（需要修改结构）
```

---

## 📊 预期运行日志

### Mock 模式日志示例

```
=== Coin Pusher System (Event-Driven Architecture) ===
Transport Mode: Serial (USB-to-Ethernet via External Chip)
Initializing...

Event system initialized
Spawning tasks...
  - Button task spawned
  - Heartbeat task spawned
  - Dispatch task spawned
  - Serial Transport task spawned (MOCK mode)

=== System ready ===
Event-driven architecture running with Serial Transport...
Waiting for serial data (mock: every 5 seconds)...

[5 秒后]
Serial Transport: Mock: Simulating serial data reception
Serial Transport: Serial received 10 bytes
Serial Transport: Decoded packet: type=Command, seq=1, len=2
Serial Transport: Injecting NetworkIncoming event: cmd=2001
Dispatch: Dispatching event
Router: Routing network event: cmd=2001
Handler: Network message (cmd: 2001, 0 bytes)
Handler:   -> Request Status
```

---

## 🔙 如何回滚到原版

### 方法 1: 恢复备份文件

```bash
cp src/main_original.rs src/main.rs
cargo check --target thumbv7em-none-eabihf
```

### 方法 2: 注释 Serial Transport 代码

在 `main.rs` 中注释第 78-106 行：

```rust
// ========== 启动 Serial Transport（新增）==========
/*
let serial_config = SerialTransportConfig { ... };
...
spawner.spawn(serial_transport_task(...)).unwrap();
*/
```

---

## ✅ 验证清单

- [x] **备份完成**：`src/main_original.rs` 已创建（2.2KB）
- [x] **代码更新**：`src/main.rs` 已集成 Serial Transport（3.4KB）
- [x] **编译通过**：`cargo check --target thumbv7em-none-eabihf` ✅
- [x] **警告处理**：仅有未使用代码警告（不影响功能）
- [x] **文档完善**：SERIAL_TRANSPORT_GUIDE.md 已创建

---

## 📝 总结

### 变更范围

- **修改行数**：+29 行（新增 Serial Transport 启动逻辑）
- **侵入性**：最小（仅在 main 函数末尾添加）
- **兼容性**：完全向后兼容（可随时回滚）

### 设计亮点

1. **内联 Task 定义**：避免全局命名空间污染
2. **StaticCell 模式**：符合 Embassy 静态生命周期要求
3. **清晰日志**：明确指示传输模式和运行状态
4. **易于回滚**：原版代码完整保留

---

**当前状态**：✅ 系统已启用 Serial Transport（Mock 模式），可直接运行测试。

# Serial Transport 架构说明

## 📋 概述

`serial_transport` 是与 `tcp_server` 并列的 **Event Producer**，专为 **USB 转网口硬件方案**设计。

### 硬件背景

- **硬件方案**：USB 转网口转换芯片
- **协议栈位置**：TCP/IP 完整实现在**外部芯片**
- **MCU 串口语义**：接收到的字节流 = TCP socket 的应用层 payload
- **已完成处理**：
  - ✅ TCP 三次握手、四次挥手
  - ✅ 数据包重组、排序
  - ✅ 校验和验证、丢包重传
  - ✅ 流量控制、拥塞控制

**结论**：串口数据 = 可靠、有序、完整的应用层字节流

---

## 🏗️ 架构对比：TCP vs Serial

### 数据流对比

#### TCP Server (`src/net/tcp_server.rs`)

```
┌─────────────────┐
│  TcpSocket      │  Embassy 网络栈
│  ::read()       │
└────────┬────────┘
         │ 字节流（应用层）
         ↓
┌─────────────────┐
│  PacketCodec    │  协议解码
│  feed + decode  │
└────────┬────────┘
         │ Packet { type, seq, payload }
         ↓
┌─────────────────┐
│  提取 cmd       │  cmd (2 bytes) + payload
│  + payload      │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  router         │  ⚠️ 直接调用 handler
│  .handle_msg()  │  （不走 Event Channel）
└─────────────────┘
```

#### Serial Transport (`src/net/serial_transport.rs`)

```
┌─────────────────┐
│  UART           │  串口接收（硬件已完成 TCP 处理）
│  ::read()       │
└────────┬────────┘
         │ 字节流（应用层，与 TCP 语义等价）
         ↓
┌─────────────────┐
│  PacketCodec    │  协议解码（完全相同）
│  feed + decode  │
└────────┬────────┘
         │ Packet { type, seq, payload }
         ↓
┌─────────────────┐
│  提取 cmd       │  cmd (2 bytes) + payload（完全相同）
│  + payload      │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  Event Channel  │  ✅ 标准 Event Producer
│  ::send()       │
└────────┬────────┘
         │ Event::NetworkIncoming { cmd, payload }
         ↓
┌─────────────────┐
│  dispatch_task  │  事件分发
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  route_event    │  事件路由
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  handlers::     │  业务处理
│  network        │
└─────────────────┘
```

---

## 🔑 关键差异总结

| 维度           | TCP Server               | Serial Transport             |
| -------------- | ------------------------ | ---------------------------- |
| **数据源**     | `TcpSocket::read()`      | `UART::read()` (mock)        |
| **数据语义**   | 应用层 payload           | 应用层 payload（硬件已处理） |
| **协议解码**   | `PacketCodec`            | `PacketCodec`（完全相同）    |
| **cmd 提取**   | `BigEndian::read_u16()`  | `BigEndian::read_u16()`（相同） |
| **事件注入**   | ❌ 直接调用 handler      | ✅ 发送 `Event::NetworkIncoming` |
| **上层感知**   | 绕过 Event Channel       | 标准 Event Channel 路径      |
| **可替换性**   | 需要修改 handler 调用逻辑 | 完全透明，上层无感           |

---

## 📦 Serial Transport 职责清单

### ✅ 负责的事情

1. **从串口读取字节流**（Demo 中使用 `mock_serial_read()`）
2. **使用现有 `PacketCodec` 解码**（与 TCP 完全一致）
3. **提取 cmd 和 payload**（协议格式：`[cmd: 2B][payload: nB]`）
4. **构造 `Event::NetworkIncoming`**
5. **通过 `event_tx` 注入事件系统**

### ❌ 不负责的事情（硬件已完成）

1. ❌ **TCP 握手/挥手**（外部芯片完成）
2. ❌ **数据包重组、排序**（外部芯片完成）
3. ❌ **CRC 校验、校验和**（外部芯片完成，MCU 只做应用层 checksum）
4. ❌ **丢包检测、重传**（外部芯片完成）
5. ❌ **流量控制、拥塞控制**（外部芯片完成）
6. ❌ **响应发送**（由 handlers 处理，Serial Transport 仅负责接收）

---

## 🔧 接入真实硬件的步骤

### 当前 Demo 实现

```rust
// src/net/serial_transport.rs:94-104
async fn mock_serial_read(&self) -> Option<&'static [u8]> {
    Timer::after(Duration::from_secs(5)).await;

    static MOCK_DATA: &[u8] = &[
        // PacketHeader + Payload
        0xAA, 0x55, 0x20, 0x01, 0x00, 0x02, 0x00, 0x00,
        0x20, 0x01,
    ];

    Some(MOCK_DATA)
}
```

### 替换为真实 UART（仅需修改一处）

#### 步骤 1：在 `main.rs` 中初始化 UART

```rust
use embassy_stm32::usart::{Config as UartConfig, Uart};

let mut uart_config = UartConfig::default();
uart_config.baudrate = 115200;

let uart = Uart::new(
    p.USART1,
    p.PA10,      // RX
    p.PA9,       // TX
    Irqs,
    p.DMA1_CH4,  // TX DMA
    p.DMA1_CH5,  // RX DMA
    uart_config,
);

// 将 uart 传递给 SerialTransport
```

#### 步骤 2：修改 `serial_transport.rs` 的 `start()` 函数

**替换前**（第 67-77 行）：
```rust
let rx_data = if self.config.mock_mode {
    match self.mock_serial_read().await {
        Some(data) => data,
        None => {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
    }
} else {
    error!("Real UART not implemented yet");
    Timer::after(Duration::from_secs(1)).await;
    continue;
};
```

**替换后**：
```rust
let mut rx_buffer = [0u8; 512];
let rx_data = match uart.read(&mut rx_buffer).await {
    Ok(0) => {
        // 超时或连接断开
        Timer::after(Duration::from_millis(100)).await;
        continue;
    }
    Ok(n) => &rx_buffer[..n],
    Err(e) => {
        error!("UART read error: {:?}", e);
        Timer::after(Duration::from_millis(100)).await;
        continue;
    }
};
```

#### 步骤 3：删除 `mock_serial_read()` 函数

删除 `serial_transport.rs` 第 147-175 行的 mock 实现。

**完成！**后续的 codec 解码、Event 注入逻辑**完全不变**。

---

## 🚀 系统启动配置

在 `main.rs` 中选择启用哪种传输方式：

### 选项 1: TCP 模式（现有）

```rust
// 需要先创建 tcp_server_task
use net::{TcpServer, TcpServerConfig};

let tcp_server = TcpServer::new(TcpServerConfig::default());
spawner.spawn(tcp_server_task(stack, router)).unwrap();
```

### 选项 2: Serial 模式（新增）

```rust
use net::{SerialTransport, SerialTransportConfig};

let serial_config = SerialTransportConfig {
    read_timeout: Duration::from_secs(30),
    mock_mode: false,  // 使用真实 UART
};

let serial_transport = SerialTransport::new(serial_config);

#[embassy_executor::task]
async fn serial_transport_task(
    transport: &'static SerialTransport,
    event_tx: Sender<'static, CriticalSectionRawMutex, Event, 32>,
) -> ! {
    transport.start(event_tx).await
}

static SERIAL_TRANSPORT: StaticCell<SerialTransport> = StaticCell::new();
let transport = SERIAL_TRANSPORT.init(serial_transport);

spawner.spawn(serial_transport_task(transport, event_tx.clone())).unwrap();
```

### 选项 3: 同时运行（多路复用）

```rust
// 同时启动 TCP 和 Serial
spawner.spawn(tcp_server_task(stack, router)).unwrap();
spawner.spawn(serial_transport_task(transport, event_tx)).unwrap();

// 两者都会产生 Event::NetworkIncoming
// 上层系统无法区分（也不需要区分）
```

---

## 🧪 测试与验证

### Demo 模式测试

当前 `mock_mode: true` 时，每 5 秒自动生成一个测试包：

```
cmd: 0x2001 (Request Status)
payload: []
```

**预期日志**：

```
INFO  Serial Transport: Starting (Event Producer mode)
INFO  Serial Transport: ⚠️  Running in MOCK mode (for Demo)
...
INFO  Serial Transport: Mock: Simulating serial data reception
DEBUG Serial Transport: Serial received 10 bytes
DEBUG Serial Transport: Decoded packet: type=Command, seq=1, len=2
DEBUG Serial Transport: Injecting NetworkIncoming event: cmd=2001
...
INFO  Dispatch: Dispatching event
INFO  Router: Routing network event: cmd=2001
INFO  Handler: Network message (cmd: 2001, 2 bytes)
INFO  Handler:   -> Request Status
```

### 真实硬件测试

1. **连接硬件**：USB 转网口模块连接到 STM32 的 USART1
2. **配置 `mock_mode: false`**
3. **使用网络调试助手发送数据**（TCP 客户端）
4. **观察日志**：应看到与 Demo 模式相同的事件流

---

## 📝 总结

### 为什么不修改 TCP Server？

`tcp_server` 目前直接调用 `router.handle_message()`，不走 Event Channel。这是**现有代码的设计决策**，可能出于：

- 减少延迟（避免 Event Channel 排队）
- 简化响应发送（TCP 需要同步返回响应）

### Serial Transport 的设计理念

1. **最小侵入**：不修改任何现有代码（tcp_server、router、handlers）
2. **标准化**：遵循 Event-Driven 架构的标准模式
3. **可替换**：真实硬件接入只需修改 10 行代码
4. **可并存**：TCP 和 Serial 可同时运行，互不干扰

### 未来改进方向

如果需要统一 TCP 和 Serial 的处理路径，可以：

1. **修改 `tcp_server`**：改为发送 `Event::NetworkIncoming`
2. **响应处理**：在 `handlers::network` 中通过新的 Channel 返回响应数据
3. **Response Router**：创建新的 task 处理响应发送（TCP 或 Serial）

但这是**架构重构**，不在本次"最小侵入式扩展"的范围内。

---

## 📚 相关文件

- **实现**：`src/net/serial_transport.rs`
- **模块导出**：`src/net/mod.rs`
- **事件定义**：`src/event.rs`（`Event::NetworkIncoming`）
- **事件分发**：`src/tasks/dispatch_task.rs`
- **事件路由**：`src/app/router.rs`（`route_event`）
- **业务处理**：`src/app/handlers/network.rs`（`on_network_message`）

---

**✅ 实现完成：Serial Transport 已作为标准 Event Producer 集成到系统中。**

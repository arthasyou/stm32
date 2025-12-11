

# 📘 **Architect Specification Document（仿真版 / 无硬件版 / protobuf-ready）**

**Project: MCU Event Node (Simulation Version)**
**Author: Architect**
**Version: 1.1**

---

# **1. System Intent（系统意图）**

目标：
构建一个可运行于 PC/QEMU/任何无硬件环境的 **事件驱动 TCP 客户端框架**，结构与未来 MCU 环境一致，但当前仅通过：

* `print!`
* `tokio::time`
* 模拟事件
* protobuf 编码/解码
* TCP client

来实现整体流程。

该框架未来将迁移到 MCU，仅替换 driver 层，无需重构架构。

---

# **2. System Architecture（系统架构）**

架构保持五层，但 driver 层暂时为模拟版：

```
┌─────────────────────────┐
│ Application Layer       │（业务逻辑 / 路由 / handler）
├─────────────────────────┤
│ Task Layer              │（事件任务 / 网络任务 / 心跳任务）
├─────────────────────────┤
│ Network Layer           │（TCP 客户端、protobuf encode/decode）
├─────────────────────────┤
│ Drivers Layer (mock)    │（模拟事件：print、计时器）
├─────────────────────────┤
│ Utils Layer             │（日志、缓冲工具）
└─────────────────────────┘
```

**核心思想：
所有硬件事件都改为“模拟事件”，但系统行为保持一致。**

---

# **3. Module Specifications（模块规格）**

以下规范是 AI 可以直接执行的“模块行为定义”。

---

## **3.1 Drivers Layer（模拟驱动层）**

### **3.1.1 Mock Button Driver**

```
fn simulate_button_press() -> Future
```

行为：

* 使用 `tokio::time::sleep(Duration)` 模拟间隔随机事件
* 每次触发后打印 `"Simulated Button Press"`
* 发出 Event::ButtonPress

禁止：

* 不允许逻辑判断
* 不允许网络调用

---

## **3.2 Network Layer（真实 TCP + Protobuf）**

### **3.2.1 TCP Client**

行为规范：

```
loop:
    connect(server_ip, port)
    if fail → sleep 1s retry
    while connected:
         read protobuf packet → emit Event
         write protobuf messages queued from tasks
```

要求：

* 自动重连
* 非阻塞
* 使用 tokio TCP（不使用硬件网络栈）

---

### **3.2.2 Protobuf Codec**

使用你已有的 protobuf schema。

AI 需要自动生成：

* encode(message) -> Vec<u8>
* decode(bytes) -> Message enum

网络数据格式：

```
LEN (u32)
PROTOBUF_BYTES
```

（AI 自动实现 framing）

---

## **3.3 Tasks Layer（异步任务层）**

### **3.3.1 button_task（事件模拟）**

```
loop:
    wait simulated button press
    send Event::ButtonPress
```

### **3.3.2 network_task**

```
loop:
    maintain tcp
    incoming packets → Event::NetworkIncoming
```

### **3.3.3 heartbeat_task**

```
every 5 seconds:
    create Heartbeat protobuf
    send via tcp write queue
```

### **3.3.4 dispatch_task**

```
loop:
    read event queue
    route(event)
```

网络与事件完全分离。

---

## **3.4 Application Layer（业务层）**

### Event enum

```
enum Event {
    ButtonPress,
    NetworkIncoming(MyProtoMessage)
}
```

### Router

```
fn route(event: Event)
```

路由规则：

| Event 类型        | handler                      |
| --------------- | ---------------------------- |
| ButtonPress     | handlers::button::on_press() |
| NetworkIncoming | 根据 protobuf.type 调用 handler  |

---

## **3.5 Handlers Layer**

### button handler

```
on_press():
    print!("Button Press Event");
    build protobuf ButtonPressed message
    send to tcp_client
```

### misc handlers

根据 protobuf 的 type 调用不同处理函数。

---

# **4. Event System Specification**

使用 tokio mpsc：

```
EVENT_TX: Sender<Event>
EVENT_RX: Receiver<Event>
```

事件流：

```
task → event queue → dispatcher → application router
```

所有任务都必须通过事件队列，而不是直接调用 router。

---

# **5. Directory Blueprint（AI 必须生成以下结构）**

```
src/
    main.rs

    drivers/
        mock_button.rs
        mock_hw.rs

    net/
        tcp_client.rs
        codec.rs
        packet_framing.rs

    tasks/
        button_task.rs
        network_task.rs
        heartbeat_task.rs
        dispatch_task.rs

    app/
        router.rs
        handlers/
            button.rs
            heartbeat.rs
            misc.rs

    utils/
        log.rs
        buf.rs

protobuf/
    your.proto files
```

AI 必须：

* 生成 Rust protobuf struct（使用 prost）
* 创建上述所有文件
* 自动补齐必要的 mod.rs

---

# **6. Initialization Specification（初始化流程）**

main.rs:

```
initialize logging
load protobuf modules
create event queue
spawn:
    button_task
    network_task
    heartbeat_task
    dispatch_task

await forever
```

---

# **7. Evolution Rules（未来升级规则）**

## 当你把项目移植到 MCU 时：

* drivers/mock → drivers/stm32
* tokio → embassy
* TCP std → embassy-net
* 业务层完全不变
* router 完全不变
* protobuf 不变
* task 结构保持一致

架构不能被破坏。

---

# **8. Non-Goals（当前版本不做）**

* GPIO
* ETH 外设
* 中断
* DMA
* 硬件驱动
* 低功耗
* 多 socket

---

# **9. Success Criteria（成功标准）**

* 全项目可执行（tokio runtime）
* 可连接真实服务器
* 能 encode/decode protobuf
* 框架能持续运行
* 模拟按键触发事件
* 心跳按时发送
* 路由逻辑正常
* 结构可无缝迁移到 MCU

---

# ⭐ **最终使用方法**

你只需要把这句话丢给 Claude Code：

```
请根据以下 Architect Specification 文档创建一个完整的 Rust 项目（使用 tokio + prost），实现所有模块、目录结构、任务框架、事件系统、TCP client 和 protobuf 编码/解码。所有硬件驱动使用模拟版本（print）。
```



# QEMU 测试指南

## ⚠️ 重要说明

**STM32F407 + embassy-net 在 QEMU 中的限制：**

1. QEMU 对 STM32F407 的支持有限
2. **网络栈需要真实以太网硬件**，QEMU 无法完整模拟
3. embassy-net 依赖硬件驱动，在 QEMU 中无法正常工作

## 推荐测试方案

### 方案 1：真实硬件测试（推荐）

使用真实的 STM32F407 开发板 + 以太网模块：

```bash
# 使用 probe-rs（当前配置）
cargo run --release

# 或使用 OpenOCD
openocd -f interface/stlink.cfg -f target/stm32f4x.cfg
```

### 方案 2：QEMU 基本测试（仅验证编译和基本逻辑）

虽然无法测试网络功能，但可以验证代码结构：

```bash
# 1. 安装 QEMU ARM
brew install qemu  # macOS
# sudo apt install qemu-system-arm  # Linux

# 2. 切换到 QEMU 配置
cp .cargo/config-qemu.toml .cargo/config.toml

# 3. 编译
cargo build --release

# 4. 运行（会立即退出，因为没有网络硬件）
cargo run --release
```

### 方案 3：单元测试

创建单元测试验证核心逻辑：

```bash
# 运行测试
cargo test --lib
```

## QEMU 手动启动

如果需要手动启动 QEMU：

```bash
# 构建二进制文件
cargo build --release

# 手动运行 QEMU
qemu-system-arm \
  -cpu cortex-m4 \
  -machine netduinoplus2 \
  -nographic \
  -semihosting-config enable=on,target=native \
  -kernel target/thumbv7em-none-eabi/release/stm32
```

## 模拟网络（实验性）

QEMU 可以模拟网络，但需要复杂配置：

```bash
qemu-system-arm \
  -cpu cortex-m4 \
  -machine netduinoplus2 \
  -nographic \
  -semihosting-config enable=on,target=native \
  -netdev user,id=net0,hostfwd=tcp::8080-:8080 \
  -net nic,model=lan9118,netdev=net0 \
  -kernel target/thumbv7em-none-eabi/release/stm32
```

**注意**：即使配置了网络，embassy-net 仍需要真实硬件驱动初始化。

## 调试选项

添加 GDB 调试：

```bash
# 终端 1：启动 QEMU with GDB server
qemu-system-arm \
  -cpu cortex-m4 \
  -machine netduinoplus2 \
  -nographic \
  -semihosting-config enable=on,target=native \
  -gdb tcp::3333 -S \
  -kernel target/thumbv7em-none-eabi/release/stm32

# 终端 2：连接 GDB
arm-none-eabi-gdb target/thumbv7em-none-eabi/release/stm32
(gdb) target remote :3333
(gdb) continue
```

## 查看日志输出

使用 defmt-rtt 查看日志需要真实硬件。在 QEMU 中：

```bash
# 使用 semihosting 输出
# 修改 panic-probe 为 panic-semihosting

# 然后运行 QEMU
qemu-system-arm ... -semihosting-config enable=on,target=native
```

## 功能测试矩阵

| 功能 | QEMU | 真实硬件 |
|------|------|----------|
| 代码编译 | ✅ | ✅ |
| 基本执行 | ✅ | ✅ |
| defmt 日志 | ❌ | ✅ |
| GPIO/按键 | ❌ | ✅ |
| 网络栈初始化 | ❌ | ✅ |
| TCP 服务器 | ❌ | ✅ |
| 以太网通信 | ❌ | ✅ |

## 替代测试方案

### 1. Renode 模拟器（更好的 STM32 支持）

```bash
# 安装 Renode
brew install --cask renode  # macOS

# 创建 .resc 脚本
# 参考: https://renode.io/
```

### 2. 主机端测试

创建一个 std 版本的测试程序：

```rust
// tests/integration_test.rs
#[cfg(test)]
mod tests {
    use std_version_of_codec::*;

    #[test]
    fn test_packet_encoding() {
        // 测试编解码逻辑
    }
}
```

## 恢复硬件配置

完成 QEMU 测试后，恢复硬件配置：

```bash
# 恢复 probe-rs 配置
git checkout .cargo/config.toml

# 或手动编辑 runner
# runner = "probe-rs run --no-location --chip STM32F407ZG"
```

## 总结

**推荐流程**：
1. 使用真实硬件进行完整功能测试
2. QEMU 仅用于验证代码结构和基本逻辑
3. 创建单元测试验证核心算法
4. 考虑使用 Renode 进行更完整的模拟

**最佳实践**：
- 开发时：使用 QEMU 快速迭代代码结构
- 测试时：使用真实硬件验证功能
- CI/CD：使用单元测试确保代码质量

#!/bin/bash

# QEMU 测试启动脚本

set -e

echo "=== STM32 QEMU Test Runner ==="
echo ""

# 检查 QEMU 是否安装
if ! command -v qemu-system-arm &> /dev/null; then
    echo "❌ QEMU not found!"
    echo "Please install QEMU:"
    echo "  macOS:  brew install qemu"
    echo "  Linux:  sudo apt install qemu-system-arm"
    exit 1
fi

echo "✅ QEMU found: $(qemu-system-arm --version | head -n1)"
echo ""

# 备份当前配置
if [ -f .cargo/config.toml ]; then
    echo "📦 Backing up current .cargo/config.toml"
    cp .cargo/config.toml .cargo/config.toml.backup
fi

# 使用 QEMU 配置
echo "🔧 Switching to QEMU configuration"
cp .cargo/config-qemu.toml .cargo/config.toml

# 编译
echo "🔨 Building project..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    # 恢复配置
    if [ -f .cargo/config.toml.backup ]; then
        mv .cargo/config.toml.backup .cargo/config.toml
    fi
    exit 1
fi

echo "✅ Build successful"
echo ""

# 运行 QEMU
echo "🚀 Starting QEMU..."
echo "Note: Network functionality will NOT work in QEMU"
echo "Press Ctrl+A then X to exit QEMU"
echo ""

qemu-system-arm \
  -cpu cortex-m4 \
  -machine netduinoplus2 \
  -nographic \
  -semihosting-config enable=on,target=native \
  -serial mon:stdio \
  -kernel target/thumbv7em-none-eabi/release/stm32

# 恢复配置
echo ""
echo "🔄 Restoring original configuration"
if [ -f .cargo/config.toml.backup ]; then
    mv .cargo/config.toml.backup .cargo/config.toml
    echo "✅ Configuration restored"
else
    echo "⚠️  No backup found, keeping QEMU config"
fi

echo ""
echo "=== Test Complete ==="

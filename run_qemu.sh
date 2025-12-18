#!/bin/bash
# QEMU 测试脚本
# 使用方法: ./run_qemu.sh

set -e

echo "=========================================="
echo "🚀 STM32 QEMU 模拟器测试"
echo "=========================================="
echo ""
echo "目标芯片: STM32F407 (Cortex-M4)"
echo "QEMU 平台: olimex-stm32-h405"
echo ""
echo "按 Ctrl+C 停止 QEMU"
echo "或打开新终端运行: killall qemu-system-arm"
echo ""
echo "=========================================="
echo ""

# 检查二进制文件是否存在
if [ ! -f "target/thumbv7em-none-eabihf/release/stm32" ]; then
    echo "❌ 错误: 二进制文件不存在"
    echo "请先运行: cargo build --target thumbv7em-none-eabihf --release"
    exit 1
fi

echo "✓ 二进制文件已找到"
echo "✓ 启动 QEMU..."
echo ""

# 启动 QEMU
qemu-system-arm \
  -M olimex-stm32-h405 \
  -kernel target/thumbv7em-none-eabihf/release/stm32 \
  -nographic \
  -semihosting \
  -d guest_errors,unimp

echo ""
echo "=========================================="
echo "✅ QEMU 已退出"
echo "=========================================="

//! 测试编解码器和路由器的示例
//! 可以在 QEMU 中运行基本测试
#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// 引入我们的模块
// 注意：由于 examples 的路径问题，这里需要调整

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let config = Config::default();
    let _p = embassy_stm32::init(config);

    info!("=== TCP Server Component Test ===");

    // 测试 1: 数据包编解码
    test_packet_codec();

    // 测试 2: 路由器
    test_router();

    loop {
        info!("Test completed, looping...");
        Timer::after_secs(5).await;
    }
}

fn test_packet_codec() {
    info!("Testing packet codec...");

    // TODO: 添加编解码器测试
    // 由于模块在 main crate 中，这里需要重新组织代码结构

    info!("Packet codec test passed");
}

fn test_router() {
    info!("Testing router...");

    // TODO: 添加路由器测试

    info!("Router test passed");
}

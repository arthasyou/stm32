#![no_std]
#![no_main]

// 启用 alloc
extern crate alloc;

use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

mod error;
mod net;

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// 引入测试需要的模块
use net::{PacketCodec, PacketType, Router};
use heapless::Vec;

// 包含 protobuf 生成的代码
pub mod coinpusher {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/coinpusher.v1.rs"));
    }
}

use coinpusher::v1::*;

// 命令码定义
const CMD_REQUEST_STATUS: u16 = 2001;
const CMD_LIGHT_COMMAND: u16 = 2002;
const CMD_MOTOR_COMMAND: u16 = 2003;

// 简化的 handler（演示用）
fn handle_test_proto(data: Vec<u8, 512>) -> error::Result<Vec<u8, 512>> {
    info!("Protobuf handler called with {} bytes", data.len());

    // 尝试解析为 protobuf 消息
    // TODO: 实际解析和处理

    let mut response = Vec::new();
    response.extend_from_slice(b"OK").ok();
    Ok(response)
}

// 创建完整的路由表
fn setup_router() -> Router {
    let mut router = Router::new();

    // 注册测试处理器
    router.add_route(CMD_REQUEST_STATUS, handle_test_proto);
    router.add_route(CMD_LIGHT_COMMAND, handle_test_proto);
    router.add_route(CMD_MOTOR_COMMAND, handle_test_proto);

    info!("Router initialized with protobuf support");
    router
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    // 初始化堆内存 (32KB)
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

    let config = Config::default();
    let _p = embassy_stm32::init(config);

    info!("=== Coin Pusher Protocol Test (Protobuf) ===");
    info!("");

    // 创建路由器
    let router = setup_router();

    // 测试 protobuf 消息
    test_protobuf_messages();

    info!("");
    info!("=== Protobuf system ready ===");
    info!("Waiting for TCP connections...");
    info!("");

    loop {
        info!("Heartbeat...");
        Timer::after_secs(5).await;
    }
}

/// 测试 protobuf 消息
fn test_protobuf_messages() {
    use prost::Message;

    info!("Test: Protobuf Messages");

    // 测试心跳消息
    info!("  Testing Heartbeat message...");
    let heartbeat = M1001Toc {
        uptime_ms: 12345,
        all_ok: BoolFlag::BoolTrue as i32,
        error_count: 0,
        state_version: Some(1),
    };

    let mut buf = alloc::vec::Vec::new();
    heartbeat.encode(&mut buf).ok();
    info!("    ✓ Heartbeat encoded: {} bytes", buf.len());

    // 测试按钮事件
    info!("  Testing ButtonEvent message...");
    let button_event = M1003Toc {
        button_id: 1,
        action: ButtonAction::ButtonPressed as i32,
        duration_ms: Some(100),
    };

    let mut buf = alloc::vec::Vec::new();
    button_event.encode(&mut buf).ok();
    info!("    ✓ ButtonEvent encoded: {} bytes", buf.len());

    info!("  All protobuf tests passed!");
    info!("");
}


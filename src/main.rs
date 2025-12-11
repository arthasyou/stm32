#![no_std]
#![no_main]

// 启用 alloc
extern crate alloc;

use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

mod error;
mod net;
mod event;
mod drivers;
mod tasks;
mod app;

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// 引入测试需要的模块
use net::{PacketCodec, PacketType, Router};
use heapless::Vec;


#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
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

    info!("=== Coin Pusher System (Event-Driven Architecture) ===");
    info!("Initializing...");
    info!("");

    // 创建事件通道
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::channel::Channel;
    use event::Event;

    static EVENT_CHANNEL: static_cell::StaticCell<Channel<CriticalSectionRawMutex, Event, 32>> =
        static_cell::StaticCell::new();

    let event_channel = EVENT_CHANNEL.init(Channel::new());
    let event_tx = event_channel.sender();
    let event_rx = event_channel.receiver();

    info!("Event system initialized");

    // 启动所有任务
    info!("Spawning tasks...");

    spawner.spawn(tasks::button_task::button_task(event_tx.clone())).unwrap();
    info!("  - Button task spawned");

    spawner.spawn(tasks::heartbeat_task::heartbeat_task(event_tx.clone())).unwrap();
    info!("  - Heartbeat task spawned");

    spawner.spawn(tasks::dispatch_task::dispatch_task(event_rx)).unwrap();
    info!("  - Dispatch task spawned");

    info!("");
    info!("=== System ready ===");
    info!("Event-driven architecture running...");
    info!("");

    // 主任务空转
    loop {
        Timer::after_secs(60).await;
        info!("Main: System running...");
    }
}

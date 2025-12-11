// 按钮事件任务
use crate::event::Event;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};

/// 按钮任务
///
/// 监听按钮事件并发送到事件队列
#[embassy_executor::task]
pub async fn button_task(
    event_tx: Sender<'static, CriticalSectionRawMutex, Event, 32>,
) -> ! {
    info!("Button task started");

    let mut button_id = 0u32;

    loop {
        // 模拟按钮按下（每3秒）
        Timer::after(Duration::from_secs(3)).await;

        button_id = (button_id + 1) % 4; // 循环 4 个按钮

        let event = Event::ButtonPress {
            button_id,
            duration_ms: Some(100),
        };

        info!("Button {} pressed, sending event", button_id);

        // 发送事件到队列
        event_tx.send(event).await;
    }
}

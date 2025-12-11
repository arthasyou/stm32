// 心跳任务
use crate::event::Event;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};

/// 心跳任务
///
/// 定期发送心跳事件
#[embassy_executor::task]
pub async fn heartbeat_task(
    event_tx: Sender<'static, CriticalSectionRawMutex, Event, 32>,
) -> ! {
    info!("Heartbeat task started");

    loop {
        // 每5秒发送一次心跳
        Timer::after(Duration::from_secs(5)).await;

        info!("Heartbeat tick");

        let event = Event::HeartbeatTick;

        // 发送心跳事件
        event_tx.send(event).await;
    }
}

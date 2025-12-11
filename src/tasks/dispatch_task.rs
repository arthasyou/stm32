// 事件分发任务
use crate::app::router::route_event;
use crate::event::Event;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;

/// 事件分发任务
///
/// 从事件队列接收事件并路由到对应的处理器
#[embassy_executor::task]
pub async fn dispatch_task(
    event_rx: Receiver<'static, CriticalSectionRawMutex, Event, 32>,
) -> ! {
    info!("Dispatch task started");

    loop {
        // 从队列接收事件
        let event = event_rx.receive().await;

        info!("Dispatching event");

        // 路由到对应的处理器
        if let Err(_e) = route_event(event) {
            defmt::warn!("Event routing failed");
        }
    }
}

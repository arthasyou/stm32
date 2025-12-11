// 事件路由器
use crate::app::handlers;
use crate::error::Result;
use crate::event::Event;
use defmt::info;

/// 路由事件到对应的处理器
pub fn route_event(event: Event) -> Result<()> {
    match event {
        Event::ButtonPress {
            button_id,
            duration_ms,
        } => {
            info!("Routing button event: id={}", button_id);
            handlers::button::on_button_press(button_id, duration_ms)
        }

        Event::CoinInsert { channel_id, value } => {
            info!("Routing coin event: channel={}, value={}", channel_id, value);
            handlers::coin::on_coin_insert(channel_id, value)
        }

        Event::HeartbeatTick => {
            info!("Routing heartbeat event");
            handlers::heartbeat::on_heartbeat()
        }

        Event::NetworkIncoming { cmd, payload } => {
            info!("Routing network event: cmd={:04X}", cmd);
            handlers::network::on_network_message(cmd, payload)
        }

        Event::MotorStateChanged { motor_id, running } => {
            info!("Routing motor event: id={}, running={}", motor_id, running);
            handlers::motor::on_motor_state_changed(motor_id, running)
        }

        Event::FaultDetected {
            hardware_type,
            severity,
        } => {
            info!("Routing fault event: hw_type={}", hardware_type);
            handlers::fault::on_fault_detected(hardware_type, severity)
        }
    }
}

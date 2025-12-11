// 马达事件处理
use crate::error::Result;
use defmt::info;

/// 处理马达状态变化事件
pub fn on_motor_state_changed(motor_id: u32, running: bool) -> Result<()> {
    info!(
        "Handler: Motor {} state changed: {}",
        motor_id,
        if running { "RUNNING" } else { "STOPPED" }
    );

    // TODO: 更新马达状态
    // TODO: 发送状态更新到服务器

    Ok(())
}

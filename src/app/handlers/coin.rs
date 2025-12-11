// 投币事件处理
use crate::error::Result;
use defmt::info;

/// 处理投币事件
pub fn on_coin_insert(channel_id: u32, value: u32) -> Result<()> {
    info!("Handler: Coin inserted (channel: {}, value: {})", channel_id, value);

    // TODO: 更新投币统计
    // TODO: 触发马达或其他动作
    // TODO: 发送投币事件到服务器

    Ok(())
}

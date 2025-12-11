// 按键事件处理
use crate::error::Result;
use defmt::info;
use prost::Message;

/// 处理按钮按下事件
pub fn on_button_press(button_id: u32, duration_ms: Option<u32>) -> Result<()> {
    info!(
        "Handler: Button {} pressed (duration: {:?} ms)",
        button_id,
        duration_ms
    );

    // 创建 protobuf 消息
    let button_event = crate::event::coinpusher::v1::M1003Toc {
        button_id,
        action: crate::event::coinpusher::v1::ButtonAction::ButtonPressed as i32,
        duration_ms,
    };

    // 编码为字节
    let mut buf = alloc::vec::Vec::new();
    button_event.encode(&mut buf).map_err(|_| crate::error::Error::InvalidParameter)?;

    info!("  Encoded button event: {} bytes", buf.len());

    // TODO: 发送到网络层
    // 在真实系统中，这里会将消息发送给 TCP 客户端或服务器

    Ok(())
}

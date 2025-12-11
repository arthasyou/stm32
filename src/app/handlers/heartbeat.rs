// 心跳事件处理
use crate::error::Result;
use defmt::info;
use embassy_time::Instant;
use prost::Message;

/// 处理心跳事件
pub fn on_heartbeat() -> Result<()> {
    let uptime_ms = Instant::now().as_millis() as u32;

    info!("Handler: Heartbeat (uptime: {} ms)", uptime_ms);

    // 创建心跳 protobuf 消息
    let heartbeat = crate::event::coinpusher::v1::M1001Toc {
        uptime_ms,
        all_ok: crate::event::coinpusher::v1::BoolFlag::BoolTrue as i32,
        error_count: 0,
        state_version: Some(1),
    };

    // 编码为字节
    let mut buf = alloc::vec::Vec::new();
    heartbeat.encode(&mut buf).map_err(|_| crate::error::Error::InvalidParameter)?;

    info!("  Encoded heartbeat: {} bytes", buf.len());

    // TODO: 发送到网络层

    Ok(())
}

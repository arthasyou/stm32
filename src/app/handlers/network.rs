// 网络消息处理
use crate::error::Result;
use alloc::vec::Vec;
use defmt::info;

/// 处理网络接收的消息
pub fn on_network_message(cmd: u16, payload: Vec<u8>) -> Result<()> {
    info!("Handler: Network message (cmd: {:04X}, {} bytes)", cmd, payload.len());

    // 根据命令码分发到具体处理器
    match cmd {
        0x2001 => handle_request_status(&payload),
        0x2002 => handle_light_command(&payload),
        0x2003 => handle_motor_command(&payload),
        0x2004 => handle_clear_fault(&payload),
        0x2005 => handle_simulate_fault(&payload),
        _ => {
            defmt::warn!("Unknown network command: {:04X}", cmd);
            Err(crate::error::Error::NotFound)
        }
    }
}

fn handle_request_status(_payload: &[u8]) -> Result<()> {
    info!("  -> Request Status");
    // TODO: 发送状态报告
    Ok(())
}

fn handle_light_command(_payload: &[u8]) -> Result<()> {
    info!("  -> Light Command");
    // TODO: 控制灯光
    Ok(())
}

fn handle_motor_command(_payload: &[u8]) -> Result<()> {
    info!("  -> Motor Command");
    // TODO: 控制马达
    Ok(())
}

fn handle_clear_fault(_payload: &[u8]) -> Result<()> {
    info!("  -> Clear Fault");
    // TODO: 清除故障
    Ok(())
}

fn handle_simulate_fault(_payload: &[u8]) -> Result<()> {
    info!("  -> Simulate Fault");
    // TODO: 模拟故障
    Ok(())
}

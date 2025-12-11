// 事件系统
//
// 所有系统事件都通过这个枚举传递

use alloc::vec::Vec;
use coinpusher::v1::*;

/// 系统事件
#[derive(Debug, Clone)]
pub enum Event {
    /// 按钮按下事件
    ButtonPress {
        button_id: u32,
        duration_ms: Option<u32>,
    },

    /// 投币事件
    CoinInsert {
        channel_id: u32,
        value: u32,
    },

    /// 网络接收到的消息
    NetworkIncoming {
        cmd: u16,
        payload: Vec<u8>,
    },

    /// 心跳定时器触发
    HeartbeatTick,

    /// 马达状态变化
    MotorStateChanged {
        motor_id: u32,
        running: bool,
    },

    /// 故障事件
    FaultDetected {
        hardware_type: i32,
        severity: i32,
    },
}

impl Event {
    /// 将按钮事件转换为 protobuf 消息
    pub fn to_button_event_proto(&self) -> Option<M1003Toc> {
        match self {
            Event::ButtonPress {
                button_id,
                duration_ms,
            } => Some(M1003Toc {
                button_id: *button_id,
                action: ButtonAction::ButtonPressed as i32,
                duration_ms: *duration_ms,
            }),
            _ => None,
        }
    }

    /// 将投币事件转换为 protobuf 消息
    pub fn to_coin_event_proto(&self) -> Option<M1004Toc> {
        match self {
            Event::CoinInsert { channel_id, value } => Some(M1004Toc {
                channel_id: *channel_id,
                coin_value: Some(*value),
                quantity: 1,
                total: None,
            }),
            _ => None,
        }
    }
}

// 重新导出 protobuf 模块以便事件系统使用
pub mod coinpusher {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/coinpusher.v1.rs"));
    }
}

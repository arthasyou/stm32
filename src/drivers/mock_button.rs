// 模拟按钮驱动（用于测试）
use defmt::info;
use embassy_time::{Duration, Timer};

/// 模拟按钮按下事件
///
/// 在真实硬件上，这会被替换为 GPIO 中断处理
pub async fn simulate_button_press(button_id: u32, interval_secs: u64) -> ! {
    loop {
        // 等待一段时间模拟按钮按下
        Timer::after(Duration::from_secs(interval_secs)).await;

        info!("Mock: Button {} pressed", button_id);

        // 在真实硬件上，这里会读取 GPIO 状态
        // 现在只是周期性触发事件
    }
}

/// 模拟多个按钮
pub struct MockButtonDriver {
    pub button_count: u32,
}

impl MockButtonDriver {
    pub const fn new(button_count: u32) -> Self {
        Self { button_count }
    }

    /// 获取按钮状态（模拟）
    pub fn is_pressed(&self, button_id: u32) -> bool {
        // 在真实硬件上，这里会读取 GPIO
        button_id < self.button_count
    }
}

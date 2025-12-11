// 模拟其他硬件（投币器、马达等）
use defmt::info;
use embassy_time::{Duration, Timer};

/// 模拟投币事件
pub async fn simulate_coin_insert(interval_secs: u64) -> ! {
    loop {
        Timer::after(Duration::from_secs(interval_secs)).await;

        info!("Mock: Coin inserted");

        // 在真实硬件上，这里会读取投币器脉冲信号
    }
}

/// 模拟马达驱动
pub struct MockMotorDriver {
    pub motor_id: u32,
    pub running: bool,
}

impl MockMotorDriver {
    pub const fn new(motor_id: u32) -> Self {
        Self {
            motor_id,
            running: false,
        }
    }

    /// 启动马达
    pub fn start(&mut self) {
        info!("Mock: Motor {} started", self.motor_id);
        self.running = true;
    }

    /// 停止马达
    pub fn stop(&mut self) {
        info!("Mock: Motor {} stopped", self.motor_id);
        self.running = false;
    }

    /// 运行指定时间
    pub async fn run_for_duration(&mut self, duration_ms: u32) {
        self.start();
        Timer::after(Duration::from_millis(duration_ms as u64)).await;
        self.stop();
    }
}

/// 模拟灯光驱动
pub struct MockLightDriver {
    pub light_id: u32,
    pub is_on: bool,
}

impl MockLightDriver {
    pub const fn new(light_id: u32) -> Self {
        Self {
            light_id,
            is_on: false,
        }
    }

    /// 打开灯光
    pub fn turn_on(&mut self) {
        info!("Mock: Light {} ON", self.light_id);
        self.is_on = true;
    }

    /// 关闭灯光
    pub fn turn_off(&mut self) {
        info!("Mock: Light {} OFF", self.light_id);
        self.is_on = false;
    }

    /// 设置状态
    pub fn set(&mut self, on: bool) {
        if on {
            self.turn_on();
        } else {
            self.turn_off();
        }
    }
}

pub mod button;
pub mod led;
pub mod sensor;
pub mod hw_init;

// 模拟驱动（用于测试）
pub mod mock_button;
pub mod mock_hw;

pub use mock_button::*;
pub use mock_hw::*;

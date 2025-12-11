// 错误定义
use defmt::Format;

/// 结果类型
pub type Result<T> = core::result::Result<T, Error>;

/// 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum Error {
    /// 未找到
    NotFound,
    /// 系统错误
    SystemError,
    /// 无效参数
    InvalidParameter,
    /// 缓冲区满
    BufferFull,
    /// 网络错误
    NetworkError,
    /// 超时
    Timeout,
}

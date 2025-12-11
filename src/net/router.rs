// 命令路由器（简化版）
use crate::error::Result;
use defmt::{info, warn};
use heapless::Vec;

/// 路由器最大路由数量
const MAX_ROUTES: usize = 32;

/// 命令处理器函数指针类型（简化，无需 event channel）
pub type HandlerFn = fn(Vec<u8, 512>) -> Result<Vec<u8, 512>>;

/// 路由条目
struct Route {
    cmd: u16,
    handler: HandlerFn,
}

/// 路由器
pub struct Router {
    routes: Vec<Route, MAX_ROUTES>,
}

impl Router {
    /// 创建新的路由器
    pub const fn new() -> Self {
        Self {
            routes: Vec::new(),
        }
    }

    /// 添加路由
    pub fn add_route(&mut self, cmd: u16, handler: HandlerFn) -> &mut Self {
        if self.routes.push(Route { cmd, handler }).is_err() {
            panic!("Too many routes");
        }
        self
    }

    /// 处理消息（简化版，移除 event channel）
    pub fn handle_message(
        &self,
        cmd: u16,
        data: Vec<u8, 512>,
    ) -> Result<Vec<u8, 512>> {
        // 查找对应的处理器
        for route in self.routes.iter() {
            if route.cmd == cmd {
                info!("Routing cmd {} to handler", cmd);
                return (route.handler)(data);
            }
        }

        warn!("No handler found for cmd {}", cmd);
        Err(crate::error::Error::NotFound)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

// 示例处理器
pub fn example_handler(data: Vec<u8, 512>) -> Result<Vec<u8, 512>> {
    info!("Example handler called with {} bytes", data.len());
    // 简单地回显数据
    Ok(data)
}

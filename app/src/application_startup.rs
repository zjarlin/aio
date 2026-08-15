//! AIO 服务端启动阶段的路由装配状态。

use axum::Router;

/// 由应用启动器顺序合并的最终 HTTP 路由。
#[derive(Default)]
pub struct ApplicationStartup {
    router: Router,
}

impl ApplicationStartup {
    pub fn merge_router(&mut self, router: Router) {
        let current = std::mem::take(&mut self.router);
        self.router = current.merge(router);
    }

    pub fn into_router(self) -> Router {
        self.router
    }
}

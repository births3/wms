//! spike-001-server — 启动 demo 服务（用于手工 curl 验证）
//!
//! 运行：
//!     cargo run --bin spike-001-server
//!     curl -X POST http://localhost:8080/login \
//!          -H 'Content-Type: application/json' \
//!          -d '{"user_name":"alice","password":"alice_pwd"}'
//!
//! tokio::test 集成测试见 tests/auth.rs。

use spike_001_axum_jwt::{build_router, build_test_state};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let state = build_test_state();
    let app = build_router(state);

    let addr: std::net::SocketAddr = "0.0.0.0:8080".parse().expect("addr 解析失败");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("绑定失败");
    tracing::info!("spike-001-server listening on {}", addr);

    axum::serve(listener, app).await.expect("serve 失败");
}

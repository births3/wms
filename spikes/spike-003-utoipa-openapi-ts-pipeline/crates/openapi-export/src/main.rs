//! openapi-export — 把 wms_api::ApiDoc 序列化为 openapi.json
//!
//! 用法：
//!     cargo run --bin openapi-export > shared/openapi.json
//!
//! 不需要数据库连接、不需要启动 HTTP server，纯静态序列化。
//! H2 假设：CI 上 `cargo run --bin openapi-export` 即可生成 openapi.json。

use utoipa::OpenApi;
use wms_api::ApiDoc;

fn main() {
    let openapi = ApiDoc::openapi();
    let json = openapi
        .to_pretty_json()
        .expect("OpenAPI 序列化失败（utoipa to_pretty_json）");
    println!("{}", json);
}

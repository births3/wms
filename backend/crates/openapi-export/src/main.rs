//! 把主仓 `wms_api::ApiDoc` 导出为 OpenAPI JSON。

use utoipa::OpenApi;
use wms_api::ApiDoc;

fn main() {
    let json = ApiDoc::openapi()
        .to_pretty_json()
        .expect("OpenAPI 序列化失败");
    println!("{json}");
}

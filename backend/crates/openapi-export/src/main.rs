//! 把主仓 `wms_api::ApiDoc` 导出为 OpenAPI JSON。

use utoipa::OpenApi;
use wms_api::ApiDoc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = ApiDoc::openapi().to_pretty_json()?;
    println!("{json}");
    Ok(())
}

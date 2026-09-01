use serde_json::Value;
use utoipa::OpenApi;
use wms_api::ApiDoc;

#[test]
fn pda_runtime_openapi_registrations_are_complete() {
    let document =
        serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI document should serialize");
    let paths = document
        .get("paths")
        .and_then(Value::as_object)
        .expect("OpenAPI document should contain paths");

    for (path, method) in [
        (
            "/api/v1/master-data/locations/by-code/{location_code}",
            "get",
        ),
        ("/api/v1/inventory/counts/quick-spot-count", "post"),
    ] {
        assert!(
            paths.get(path).and_then(|item| item.get(method)).is_some(),
            "missing OpenAPI operation: {} {}",
            method.to_uppercase(),
            path
        );
    }

    let schemas = document
        .get("components")
        .and_then(|value| value.get("schemas"))
        .and_then(Value::as_object)
        .expect("OpenAPI document should contain component schemas");
    for schema in [
        "PdaLocationInfo",
        "QuickSpotCountRequest",
        "QuickSpotCountResponse",
    ] {
        assert!(
            schemas.contains_key(schema),
            "missing OpenAPI schema: {schema}"
        );
    }
}

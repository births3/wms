use serde_json::{Map, Value};
use utoipa::OpenApi;
use wms_api::ApiDoc;

const SCHEMA_REF_PREFIX: &str = "#/components/schemas/";

fn operation<'a>(paths: &'a Map<String, Value>, path: &str, method: &str) -> &'a Value {
    paths
        .get(path)
        .and_then(|item| item.get(method))
        .unwrap_or_else(|| {
            panic!(
                "missing OpenAPI operation: {} {}",
                method.to_uppercase(),
                path
            )
        })
}

fn collect_local_schema_refs<'a>(value: &'a Value, refs: &mut Vec<&'a str>) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                if reference.starts_with(SCHEMA_REF_PREFIX) {
                    refs.push(reference);
                }
            }
            for child in object.values() {
                collect_local_schema_refs(child, refs);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_local_schema_refs(child, refs);
            }
        }
        _ => {}
    }
}

#[test]
fn pda_runtime_openapi_registrations_are_complete() {
    let document =
        serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI document should serialize");
    let paths = document
        .get("paths")
        .and_then(Value::as_object)
        .expect("OpenAPI document should contain paths");
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

    let quick_spot_count = operation(
        paths,
        "/api/v1/inventory/counts/quick-spot-count",
        "post",
    );
    assert_eq!(
        quick_spot_count
            .pointer("/requestBody/content/application~1json/schema/$ref")
            .and_then(Value::as_str),
        Some("#/components/schemas/QuickSpotCountRequest")
    );
    assert_eq!(
        quick_spot_count
            .pointer("/responses/200/content/application~1json/schema/$ref")
            .and_then(Value::as_str),
        Some("#/components/schemas/QuickSpotCountResponse")
    );

    let location_by_code = operation(
        paths,
        "/api/v1/master-data/locations/by-code/{location_code}",
        "get",
    );
    assert_eq!(
        location_by_code
            .pointer("/responses/200/content/application~1json/schema/$ref")
            .and_then(Value::as_str),
        Some("#/components/schemas/PdaLocationInfo")
    );

    let mut local_refs = Vec::new();
    collect_local_schema_refs(&document, &mut local_refs);
    let mut dangling_refs = local_refs
        .into_iter()
        .filter(|reference| {
            let schema = (*reference)
                .strip_prefix(SCHEMA_REF_PREFIX)
                .expect("local schema reference should have the expected prefix");
            !schemas.contains_key(schema)
        })
        .collect::<Vec<_>>();
    dangling_refs.sort_unstable();
    dangling_refs.dedup();

    assert!(
        dangling_refs.is_empty(),
        "dangling local OpenAPI schema refs: {dangling_refs:?}"
    );
}

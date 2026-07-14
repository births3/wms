use chrono::{TimeZone, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::CreateProductRequest;

use super::{ctx, MasterDataError, MasterDataStore};

#[test]
fn product_rejects_uncontrolled_storage_condition() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 9, 0, 0)
        .single()
        .expect("valid time");
    let ctx = ctx(Uuid::new_v4());
    let mut store = MasterDataStore::default();

    let result = store.create_product(
        &ctx,
        CreateProductRequest {
            product_code: "P-INVALID-STORAGE".to_string(),
            product_name: "未受控商品".to_string(),
            approval_no: None,
            spec: Some("1盒".to_string()),
            dosage_form: None,
            manufacturer: None,
            special_drug_category_code: None,
            attrs: json!({"storage_condition": "室温保存"}),
        },
        now,
    );

    assert!(
        result.is_err(),
        "uncontrolled storage condition must be rejected"
    );
}

#[test]
fn product_rejects_unknown_special_drug_category() {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 9, 0, 0)
        .single()
        .expect("valid time");
    let ctx = ctx(Uuid::new_v4());
    let mut store = MasterDataStore::default();

    let result = store.create_product(
        &ctx,
        CreateProductRequest {
            product_code: "P-INVALID-CATEGORY".to_string(),
            product_name: "未知分类商品".to_string(),
            approval_no: None,
            spec: Some("1盒".to_string()),
            dosage_form: None,
            manufacturer: None,
            special_drug_category_code: Some("custom".to_string()),
            attrs: json!({"storage_condition": "normal"}),
        },
        now,
    );

    assert!(
        matches!(result, Err(MasterDataError::InvalidSpecialDrugCategory)),
        "unknown special drug category must be rejected"
    );
}

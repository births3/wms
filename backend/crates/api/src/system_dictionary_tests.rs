use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    DisableSystemDictionaryItemRequest, UpsertSystemDictionaryItemRequest,
    SYSTEM_DICTIONARY_DOCUMENT_TYPE,
};

use crate::auth::AuthContext;

use super::{
    ensure_request_owner, validate_params, PgSystemDictionaryRepository, SystemDictionaryError,
};

fn schema() -> serde_json::Value {
    json!({
        "required": ["direction", "workflow_template", "batch_policy"],
        "properties": {
            "direction": {"type": "string", "enum": ["inbound", "outbound"]},
            "workflow_template": {
                "type": "string",
                "enum": ["purchase_inbound", "sales_return"]
            },
            "batch_policy": {"type": "string", "enum": ["standard_batch"]}
        }
    })
}

#[test]
fn params_schema_accepts_required_string_enums() {
    validate_params(
        &schema(),
        &json!({
            "direction": "inbound",
            "workflow_template": "purchase_inbound",
            "batch_policy": "standard_batch"
        }),
    )
    .expect("valid document_type params should pass");
}

#[test]
fn params_schema_rejects_missing_or_invalid_enum() {
    let missing = validate_params(
        &schema(),
        &json!({
            "direction": "inbound",
            "workflow_template": "purchase_inbound"
        }),
    )
    .expect_err("missing required field should fail");
    assert!(matches!(
        missing,
        SystemDictionaryError::ParamInvalid { ref field, .. } if field == "batch_policy"
    ));

    let invalid = validate_params(
        &schema(),
        &json!({
            "direction": "sideways",
            "workflow_template": "purchase_inbound",
            "batch_policy": "standard_batch"
        }),
    )
    .expect_err("invalid enum should fail");
    assert!(matches!(
        invalid,
        SystemDictionaryError::ParamInvalid { ref field, .. } if field == "direction"
    ));
}

#[test]
fn params_schema_validates_json_types_used_by_controlled_dictionaries() {
    let schema = json!({
        "properties": {
            "requires_dual_sign": {"type": "boolean"},
            "requires_dual_person_matrix": {"type": "array"},
            "requires_dedicated_ledger": {"type": "boolean"},
            "temperature": {"type": "number"}
        }
    });
    validate_params(
        &schema,
        &json!({
            "requires_dual_sign": true,
            "requires_dual_person_matrix": [],
            "requires_dedicated_ledger": false,
            "temperature": 2.0
        }),
    )
    .expect("controlled dictionary JSON types should pass");

    let error = validate_params(&schema, &json!({"requires_dual_sign": "yes"}))
        .expect_err("boolean dictionary params must reject strings");
    assert!(matches!(
        error,
        SystemDictionaryError::ParamInvalid { ref field, .. } if field == "requires_dual_sign"
    ));

    let error = validate_params(
        &json!({"properties": {"value": {"type": "uuid"}}}),
        &json!({"value": "not-a-uuid"}),
    )
    .expect_err("unsupported schema types must fail closed");
    assert!(
        matches!(error, SystemDictionaryError::ParamInvalid { ref field, .. } if field == "value")
    );
}

#[test]
fn params_schema_enforces_numeric_bounds() {
    let schema = json!({
        "required": ["warning_days"],
        "properties": {
            "warning_days": {"type": "integer", "minimum": 1, "maximum": 3650}
        }
    });
    validate_params(&schema, &json!({"warning_days": 180}))
        .expect("valid expiry warning days should pass");
    let below_minimum = validate_params(&schema, &json!({"warning_days": 0}))
        .expect_err("zero warning days should fail");
    assert!(matches!(
        below_minimum,
        SystemDictionaryError::ParamInvalid { ref field, .. } if field == "warning_days"
    ));
    let above_maximum = validate_params(&schema, &json!({"warning_days": 3651}))
        .expect_err("excessive warning days should fail");
    assert!(matches!(
        above_maximum,
        SystemDictionaryError::ParamInvalid { ref field, .. } if field == "warning_days"
    ));
}

#[test]
fn request_owner_scope_rejects_cross_owner_write() {
    let owner_id = Uuid::new_v4();
    let ctx = AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "system-dictionary-test".to_string(),
        permissions: vec![],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };

    ensure_request_owner(&ctx, None).expect("global scope is allowed by repository");
    ensure_request_owner(&ctx, Some(owner_id)).expect("same owner is allowed");
    let error = ensure_request_owner(&ctx, Some(Uuid::new_v4()))
        .expect_err("cross owner write must be rejected");
    assert_eq!(error, SystemDictionaryError::CrossOwnerAccess);
}

#[tokio::test]
async fn mutations_reject_cross_owner_before_database() {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let ctx = AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "system-dictionary-test".to_string(),
        permissions: vec![],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let repo = PgSystemDictionaryRepository::new(
        PgPool::connect_lazy("postgres://localhost/wms").expect("lazy pool"),
    );
    let now = chrono::Utc::now();

    let upsert_error = repo
        .upsert_item(
            &ctx,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            "purchase_inbound",
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(other_owner_id),
                item_name: "跨货主采购入库".to_string(),
                enabled: true,
                params: json!({}),
                effective_from: None,
                effective_to: None,
            },
            now,
            "cross-owner-upsert",
        )
        .await
        .expect_err("cross-owner upsert must fail before database");
    assert_eq!(upsert_error, SystemDictionaryError::CrossOwnerAccess);

    let disable_error = repo
        .disable_item(
            &ctx,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            "purchase_inbound",
            DisableSystemDictionaryItemRequest {
                owner_id: Some(other_owner_id),
                disabled_reason: Some("cross-owner".to_string()),
            },
            now,
            "cross-owner-disable",
        )
        .await
        .expect_err("cross-owner disable must fail before database");
    assert_eq!(disable_error, SystemDictionaryError::CrossOwnerAccess);
}

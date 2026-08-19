use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;
use wms_domain::{SystemDictionaryCategory, SystemDictionaryItem};

#[derive(FromRow)]
pub(super) struct SystemDictionaryCategoryRow {
    dict_code: String,
    dict_name: String,
    pub(super) enabled: bool,
    control_level: String,
    param_schema: Value,
    scope_mode: String,
    override_policy: Value,
    sort_order: i32,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, FromRow)]
pub(super) struct SystemDictionaryItemRow {
    pub(super) id: Uuid,
    dict_code: String,
    item_code: String,
    item_name: String,
    enabled: bool,
    owner_id: Option<Uuid>,
    sort_order: i32,
    pub(super) params: Value,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    source: String,
    disabled_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SystemDictionaryCategoryRow> for SystemDictionaryCategory {
    fn from(row: SystemDictionaryCategoryRow) -> Self {
        Self {
            dict_code: row.dict_code,
            dict_name: row.dict_name,
            enabled: row.enabled,
            control_level: row.control_level,
            param_schema: row.param_schema,
            scope_mode: row.scope_mode,
            override_policy: row.override_policy,
            sort_order: row.sort_order,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<SystemDictionaryItemRow> for SystemDictionaryItem {
    fn from(row: SystemDictionaryItemRow) -> Self {
        Self {
            id: row.id,
            dict_code: row.dict_code,
            item_code: row.item_code,
            item_name: row.item_name,
            enabled: row.enabled,
            owner_id: row.owner_id,
            sort_order: row.sort_order,
            params: row.params,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            source: row.source,
            disabled_reason: row.disabled_reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

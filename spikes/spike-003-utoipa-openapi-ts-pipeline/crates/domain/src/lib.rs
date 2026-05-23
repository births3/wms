//! domain — 5 种典型类型，覆盖 GSP 业务真实场景的复杂度
//!
//! 验证 utoipa 对以下类型的支持：
//! 1. Item            — 基础结构 + Option + chrono::NaiveDate
//! 2. InventoryStatus — tagged union（带数据的 enum）
//! 3. Audit           — serde_json::Value（任意 JSONB）
//! 4. PaginatedItems  — 嵌套泛型容器
//! 5. ColdChainPoint  — chrono::DateTime<Utc>
//!
//! 所有类型必须能：
//! - serde 序列化/反序列化
//! - utoipa::ToSchema 生成 OpenAPI 3.x schema
//! - 导出后被 openapi-typescript 转 TS 类型，前端 strict 模式无报错

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// 商品档案（M1-001 商品档案最小集）
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Item {
    /// 商品唯一 ID（UUID v4）
    pub id: Uuid,
    /// 商品编码（如 P-001234）
    pub code: String,
    /// 品名
    pub name: String,
    /// 批号（部分商品无批号）
    pub batch_no: Option<String>,
    /// 有效期（NaiveDate；时区由仓库默认时区决定）
    pub expiry: NaiveDate,
    /// 库存数量
    pub stock: u32,
    /// 状态
    pub status: InventoryStatus,
}

/// 库存状态（tagged union — 验证 utoipa 对带数据 enum 的支持）
///
/// serde 默认外部 tag："Qualified" / { "Isolated": { "reason": "..." } }
/// 可通过 #[serde(tag = "type")] 改为内部 tag（可读性更好但需测试）
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum InventoryStatus {
    /// 合格
    Qualified,
    /// 隔离（含原因）
    Isolated { reason: String },
    /// 待检
    Quarantined,
    /// 销毁待审批
    PendingDestruction { approver_id: Uuid },
}

/// 审计事件（H2-001 append-only）
///
/// diff 字段存任意 JSON：utoipa 应当能识别 serde_json::Value 并映射为 OpenAPI
/// 的 `additionalProperties: true` 或 `type: object`（具体生成形式留给 spike 验证）
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Audit {
    pub id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub action: String,
    /// 任意 JSON diff（before / after / changed_keys）
    #[schema(value_type = Object)]
    pub diff: serde_json::Value,
}

/// 分页结果（包装 Item 列表）
///
/// 注：utoipa 对泛型类型的支持需要 #[aliases] + lifetime bound（utoipa 4.2
/// 的 ToSchema trait 带 lifetime）。本 spike 直接用非泛型版本，避免 spike
/// 复杂度过高；Wave 1 落地时如有大量分页类型可再评估泛型方案。
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct PaginatedItems {
    /// 数据列表
    pub data: Vec<Item>,
    /// 总数
    pub total: u64,
    /// 当前页（从 1 起）
    pub page: u32,
    /// 每页大小
    pub page_size: u32,
}

/// 冷链温度采样点（M5-002 冷链监控）
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct ColdChainPoint {
    /// 采样时间（UTC）
    pub t: DateTime<Utc>,
    /// 温度（摄氏度）
    pub v: f64,
    /// 是否超阈
    pub out_of_range: bool,
}

/// 错误响应（统一格式；ADR-0010 错误码模式）
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct ErrorResponse {
    /// 错误码（如 M1-001-VR-014）
    pub code: String,
    /// 错误消息（中文）
    pub message: String,
}

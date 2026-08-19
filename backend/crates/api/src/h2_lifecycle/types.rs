use chrono::{DateTime, NaiveDate, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum H2LifecycleError {
    Database(String),
    Audit(String),
    NotFound,
    InvalidInput(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageTier {
    Online,
    Archive,
    DeepArchive,
}

impl StorageTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Archive => "archive",
            Self::DeepArchive => "deep_archive",
        }
    }

    pub(super) fn from_str(value: &str) -> Result<Self, H2LifecycleError> {
        match value {
            "online" => Ok(Self::Online),
            "archive" => Ok(Self::Archive),
            "deep_archive" => Ok(Self::DeepArchive),
            _ => Err(H2LifecycleError::InvalidInput(format!(
                "unknown storage tier: {value}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    DeadLetter,
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Delivered => "delivered",
            Self::DeadLetter => "dead_letter",
        }
    }

    pub(super) fn from_str(value: &str) -> Result<Self, H2LifecycleError> {
        match value {
            "pending" => Ok(Self::Pending),
            "delivered" => Ok(Self::Delivered),
            "dead_letter" => Ok(Self::DeadLetter),
            _ => Err(H2LifecycleError::InvalidInput(format!(
                "unknown delivery status: {value}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditArchiveRun {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub reference_date: NaiveDate,
    pub partitions_seen: i32,
    pub partitions_archived: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditPartitionState {
    pub partition_name: String,
    pub partition_start: NaiveDate,
    pub partition_end: NaiveDate,
    pub storage_tier: StorageTier,
    pub target_tier: StorageTier,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSubscription {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub subscriber_key: String,
    pub event_pattern: String,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub event_type: String,
    pub delivery_count: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDelivery {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: DeliveryStatus,
    pub attempt_count: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventReplayResult {
    pub matched_events: i64,
    pub deliveries_created: i64,
    pub deliveries_requeued: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BusinessRetentionPolicy {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub policy_code: String,
    pub retention_years: Option<i32>,
    pub online_retention_months: i32,
    pub permanent: bool,
    pub special_drug: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BusinessArchiveJob {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub policy_code: String,
    pub table_name: String,
    pub target_layer: String,
    pub status: String,
    pub cutoff_date: Option<NaiveDate>,
    pub delete_allowed: bool,
}

#[derive(Debug, FromRow)]
pub(super) struct AuditPartitionStateRow {
    pub partition_name: String,
    pub partition_start: NaiveDate,
    pub partition_end: NaiveDate,
    pub storage_tier: String,
    pub target_tier: String,
}

#[derive(Debug, FromRow)]
pub(super) struct AuditArchiveRunRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub reference_date: NaiveDate,
    pub partitions_seen: i32,
    pub partitions_archived: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(super) struct EventSubscriptionRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub subscriber_key: String,
    pub event_pattern: String,
    pub active: bool,
}

#[derive(Debug, FromRow)]
pub(super) struct EventDeliveryRow {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: String,
    pub attempt_count: i32,
}

#[derive(Debug, FromRow)]
pub(super) struct BusinessRetentionPolicyRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub policy_code: String,
    pub retention_years: Option<i32>,
    pub online_retention_months: i32,
    pub permanent: bool,
    pub special_drug: bool,
}

#[derive(Debug, FromRow)]
pub(super) struct BusinessArchiveJobRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub policy_code: String,
    pub table_name: String,
    pub target_layer: String,
    pub status: String,
    pub cutoff_date: Option<NaiveDate>,
    pub delete_allowed: bool,
}

impl From<AuditArchiveRunRow> for AuditArchiveRun {
    fn from(row: AuditArchiveRunRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            reference_date: row.reference_date,
            partitions_seen: row.partitions_seen,
            partitions_archived: row.partitions_archived,
            created_at: row.created_at,
        }
    }
}

impl TryFrom<AuditPartitionStateRow> for AuditPartitionState {
    type Error = H2LifecycleError;

    fn try_from(row: AuditPartitionStateRow) -> Result<Self, Self::Error> {
        Ok(Self {
            partition_name: row.partition_name,
            partition_start: row.partition_start,
            partition_end: row.partition_end,
            storage_tier: StorageTier::from_str(&row.storage_tier)?,
            target_tier: StorageTier::from_str(&row.target_tier)?,
        })
    }
}

impl From<EventSubscriptionRow> for EventSubscription {
    fn from(row: EventSubscriptionRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            subscriber_key: row.subscriber_key,
            event_pattern: row.event_pattern,
            active: row.active,
        }
    }
}

impl TryFrom<EventDeliveryRow> for EventDelivery {
    type Error = H2LifecycleError;

    fn try_from(row: EventDeliveryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            event_id: row.event_id,
            status: DeliveryStatus::from_str(&row.status)?,
            attempt_count: row.attempt_count,
        })
    }
}

impl From<BusinessRetentionPolicyRow> for BusinessRetentionPolicy {
    fn from(row: BusinessRetentionPolicyRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            policy_code: row.policy_code,
            retention_years: row.retention_years,
            online_retention_months: row.online_retention_months,
            permanent: row.permanent,
            special_drug: row.special_drug,
        }
    }
}

impl From<BusinessArchiveJobRow> for BusinessArchiveJob {
    fn from(row: BusinessArchiveJobRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            policy_code: row.policy_code,
            table_name: row.table_name,
            target_layer: row.target_layer,
            status: row.status,
            cutoff_date: row.cutoff_date,
            delete_allowed: row.delete_allowed,
        }
    }
}

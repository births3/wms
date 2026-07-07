//! H2 archive, event bus and business retention baseline.

mod archive;
mod event_bus;
mod retention;
mod support;
mod types;

pub use archive::{audit_target_tier, run_audit_archive_cycle, sync_audit_partition_states};
pub use event_bus::{
    pending_event_deliveries, publish_event, record_delivery_failure, upsert_event_subscription,
};
pub use retention::{
    list_business_retention_policies, plan_business_archive_job,
    seed_default_business_retention_policies,
};
pub use types::{
    AuditArchiveRun, AuditPartitionState, BusinessArchiveJob, BusinessRetentionPolicy,
    DeliveryStatus, EventDelivery, EventEnvelope, EventSubscription, H2LifecycleError, StorageTier,
};

pub const DEFAULT_ONLINE_QUARTERS: i32 = 4;
pub const DEFAULT_AUDIT_RETENTION_YEARS: i32 = 5;
pub const DEFAULT_EVENT_MAX_ATTEMPTS: i32 = 3;

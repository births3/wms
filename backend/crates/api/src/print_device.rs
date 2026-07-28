//! US-H9-011 printers, trays and device leases scoped to physical print sites.
//!
//! 物理打印站点是 US-H9-012 Print Agent 的资源边界：打印机、纸盒、租约都绑定
//! 唯一站点并禁止跨站点引用；租约释放模式在创建时刻冻结快照，busy_state 的
//! 真实来源在 US-H9-010/012，本故事只实现字段与硬安全校验。

use sqlx::PgPool;
use wms_domain::PrintDeviceValidationError;

pub use crate::print_orchestration::IdempotentMutation;

mod leases;
mod printers;
mod sites;
mod support;

pub use leases::resolve_lease_release_mode;

/// 人工释放设备租约的专用权限。
pub const DEVICE_LEASE_RELEASE_PERMISSION: &str = "h9.device_lease.release";
pub(super) const PRINT_DEVICE_READ_PERMISSION: &str = "h9.print_device.read";
pub(super) const PRINT_DEVICE_WRITE_PERMISSION: &str = "h9.print_device.write";
const DEFAULT_RELEASE_MODE: &str = "manual_only";
const TEST_PRINT_DISPATCH_NOTE: &str =
    "测试指令已受控下发；真实硬件回执由 Print Agent（US-H9-012）或 S4 硬件验收登记";

/// H9 打印设备维护失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintDeviceError {
    InvalidRequest,
    ConfirmationRequired,
    ReleasePermissionRequired,
    OwnerPermissionRequired,
    SiteNotFound,
    SiteCodeConflict,
    MappingNotFound,
    MappingConflict,
    MappingAlreadyDisabled,
    PrinterNotFound,
    PrinterNameConflict,
    PrinterDisabled,
    TrayNotFound,
    TrayConflict,
    TrayDisabled,
    LeaseNotFound,
    LeaseAlreadyReleased,
    LeaseBusy(String),
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<PrintDeviceValidationError> for PrintDeviceError {
    fn from(value: PrintDeviceValidationError) -> Self {
        match value {
            PrintDeviceValidationError::ConfirmationRequired => Self::ConfirmationRequired,
            _ => Self::InvalidRequest,
        }
    }
}

/// H9 打印设备应用服务。
#[derive(Clone, Debug)]
pub struct PrintDeviceService {
    pool: PgPool,
}

impl PrintDeviceService {
    /// Builds the H9 print device service with PostgreSQL persistence.
    pub fn with_postgres(pool: PgPool) -> Self {
        Self { pool }
    }
}

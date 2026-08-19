use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

/// H9 物理打印站点：打印机、纸盒、设备租约与 Print Agent（US-H9-012）的资源边界。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSite {
    pub id: Uuid,
    pub site_code: String,
    pub site_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// H9 物理打印站点列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSiteListResponse {
    pub data: Vec<PrintSite>,
}

/// 创建 H9 物理打印站点。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CreatePrintSiteRequest {
    pub site_code: String,
    pub site_name: String,
}

/// H9 站点 ↔ 货主+仓库显式映射；停用为软删。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSiteOwnerMapping {
    pub id: Uuid,
    pub site_id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub disabled_at: Option<DateTime<Utc>>,
}

/// H9 站点货主仓映射列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSiteOwnerMappingListResponse {
    pub data: Vec<PrintSiteOwnerMapping>,
}

/// 新增 H9 站点货主仓映射。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CreateSiteOwnerMappingRequest {
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
}

/// H9 打印机；`connection_type = usb` 的租约语义为单机（仅实际连接它的本机 Agent）。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct Printer {
    pub id: Uuid,
    pub site_id: Uuid,
    pub site_code: String,
    pub site_name: String,
    pub printer_name: String,
    pub printer_model: Option<String>,
    pub connection_type: String,
    pub status: String,
    /// 打印机级释放模式覆盖；空表示继承全局默认。
    pub release_mode_override: Option<String>,
    /// 覆盖优先、否则全局默认的当前生效释放模式。
    pub effective_release_mode: String,
    pub created_at: DateTime<Utc>,
}

/// H9 打印机列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrinterListResponse {
    pub data: Vec<Printer>,
}

/// 创建 H9 打印机。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CreatePrinterRequest {
    pub site_id: Uuid,
    pub printer_name: String,
    pub printer_model: Option<String>,
    pub connection_type: String,
    pub release_mode_override: Option<String>,
}

/// 维护 H9 打印机状态与释放模式覆盖；`release_mode_override = "inherit"` 表示清除覆盖。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpdatePrinterRequest {
    pub status: Option<String>,
    pub release_mode_override: Option<String>,
}

/// H9 纸盒：纸张能力、启用状态与设备标识。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrinterTray {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub tray_code: String,
    pub paper_size: String,
    pub paper_type: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// H9 纸盒列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrinterTrayListResponse {
    pub data: Vec<PrinterTray>,
}

/// 创建 H9 纸盒。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CreatePrinterTrayRequest {
    pub tray_code: String,
    pub paper_size: String,
    pub paper_type: String,
}

/// 维护 H9 纸盒能力与启用状态。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpdatePrinterTrayRequest {
    pub paper_size: Option<String>,
    pub paper_type: Option<String>,
    pub enabled: Option<bool>,
}

/// 对指定打印机和纸盒发起测试打印。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TestPrintRequest {
    pub tray_id: Uuid,
}

/// H9 测试打印记录；本机无真实硬件时 result 停留在 `dispatched`，
/// `result/result_at/result_note` 为 Print Agent（US-H9-012）或人工回执字段。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrinterTestPrint {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub tray_id: Uuid,
    pub result: String,
    pub result_note: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub result_at: Option<DateTime<Utc>>,
}

/// H9 设备租约；`release_mode` 是租约创建时的策略快照；
/// `busy_state` 真实来源在 US-H9-010/012，printing/result_unknown/reconciling 禁止释放。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DeviceLease {
    pub id: Uuid,
    pub site_id: Uuid,
    pub printer_id: Uuid,
    pub printer_name: String,
    pub connection_type: String,
    pub holder_agent_id: Option<Uuid>,
    pub lease_token: String,
    pub release_mode: String,
    pub busy_state: String,
    pub status: String,
    pub assigned_at: DateTime<Utc>,
    pub acquired_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
    pub release_reason: Option<String>,
}

/// H9 设备租约列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DeviceLeaseListResponse {
    pub data: Vec<DeviceLease>,
    pub page: PageMeta,
}

/// 人工释放设备租约：专用权限 + 原因必填 + 二次确认。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct ReleaseDeviceLeaseRequest {
    pub reason: String,
    /// 必须为 true 表示已完成二次确认。
    pub confirm: bool,
}

/// H9 打印设备维护的纯业务校验失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintDeviceValidationError {
    SiteCodeRequired,
    SiteCodeTooLong,
    SiteNameRequired,
    SiteNameTooLong,
    IdentifierRequired,
    PrinterNameRequired,
    PrinterNameTooLong,
    InvalidConnectionType,
    InvalidReleaseMode,
    TrayCodeRequired,
    TrayCodeTooLong,
    PaperSizeRequired,
    PaperSizeTooLong,
    PaperTypeRequired,
    PaperTypeTooLong,
    ReasonRequired,
    ReasonTooLong,
    ConfirmationRequired,
}

/// 校验创建物理打印站点命令。
pub fn validate_create_print_site(
    request: &CreatePrintSiteRequest,
) -> Result<(), PrintDeviceValidationError> {
    let code = request.site_code.trim();
    if code.is_empty() {
        return Err(PrintDeviceValidationError::SiteCodeRequired);
    }
    if code.chars().count() > 64 {
        return Err(PrintDeviceValidationError::SiteCodeTooLong);
    }
    let name = request.site_name.trim();
    if name.is_empty() {
        return Err(PrintDeviceValidationError::SiteNameRequired);
    }
    if name.chars().count() > 100 {
        return Err(PrintDeviceValidationError::SiteNameTooLong);
    }
    Ok(())
}

/// 校验创建打印机命令。
pub fn validate_create_printer(
    request: &CreatePrinterRequest,
) -> Result<(), PrintDeviceValidationError> {
    if request.site_id.is_nil() {
        return Err(PrintDeviceValidationError::IdentifierRequired);
    }
    let name = request.printer_name.trim();
    if name.is_empty() {
        return Err(PrintDeviceValidationError::PrinterNameRequired);
    }
    if name.chars().count() > 100 {
        return Err(PrintDeviceValidationError::PrinterNameTooLong);
    }
    if !matches!(request.connection_type.as_str(), "network" | "usb") {
        return Err(PrintDeviceValidationError::InvalidConnectionType);
    }
    if let Some(mode) = &request.release_mode_override {
        if !matches!(mode.as_str(), "manual_only" | "safe_auto") {
            return Err(PrintDeviceValidationError::InvalidReleaseMode);
        }
    }
    Ok(())
}

/// 校验创建纸盒命令。
pub fn validate_create_printer_tray(
    request: &CreatePrinterTrayRequest,
) -> Result<(), PrintDeviceValidationError> {
    let code = request.tray_code.trim();
    if code.is_empty() {
        return Err(PrintDeviceValidationError::TrayCodeRequired);
    }
    if code.chars().count() > 64 {
        return Err(PrintDeviceValidationError::TrayCodeTooLong);
    }
    validate_paper_capability(&request.paper_size, &request.paper_type)
}

/// 校验纸盒纸张能力字段。
pub fn validate_paper_capability(
    paper_size: &str,
    paper_type: &str,
) -> Result<(), PrintDeviceValidationError> {
    let size = paper_size.trim();
    if size.is_empty() {
        return Err(PrintDeviceValidationError::PaperSizeRequired);
    }
    if size.chars().count() > 32 {
        return Err(PrintDeviceValidationError::PaperSizeTooLong);
    }
    let paper = paper_type.trim();
    if paper.is_empty() {
        return Err(PrintDeviceValidationError::PaperTypeRequired);
    }
    if paper.chars().count() > 64 {
        return Err(PrintDeviceValidationError::PaperTypeTooLong);
    }
    Ok(())
}

/// 校验人工释放租约命令：原因必填且不超长，且必须携带二次确认。
pub fn validate_release_device_lease(
    request: &ReleaseDeviceLeaseRequest,
) -> Result<(), PrintDeviceValidationError> {
    if !request.confirm {
        return Err(PrintDeviceValidationError::ConfirmationRequired);
    }
    let reason = request.reason.trim();
    if reason.is_empty() {
        return Err(PrintDeviceValidationError::ReasonRequired);
    }
    if reason.chars().count() > 500 {
        return Err(PrintDeviceValidationError::ReasonTooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        validate_create_print_site, validate_create_printer, validate_release_device_lease,
        CreatePrintSiteRequest, CreatePrinterRequest, PrintDeviceValidationError,
        ReleaseDeviceLeaseRequest,
    };
    use uuid::Uuid;

    #[test]
    fn create_site_rejects_blank_code_and_name() {
        assert_eq!(
            validate_create_print_site(&CreatePrintSiteRequest {
                site_code: " ".to_string(),
                site_name: "一号站".to_string(),
            }),
            Err(PrintDeviceValidationError::SiteCodeRequired)
        );
        assert_eq!(
            validate_create_print_site(&CreatePrintSiteRequest {
                site_code: "SITE-1".to_string(),
                site_name: "".to_string(),
            }),
            Err(PrintDeviceValidationError::SiteNameRequired)
        );
    }

    #[test]
    fn create_printer_rejects_bad_connection_and_release_mode() {
        let mut request = CreatePrinterRequest {
            site_id: Uuid::new_v4(),
            printer_name: "东库网络打印机".to_string(),
            printer_model: None,
            connection_type: "bluetooth".to_string(),
            release_mode_override: None,
        };
        assert_eq!(
            validate_create_printer(&request),
            Err(PrintDeviceValidationError::InvalidConnectionType)
        );
        request.connection_type = "network".to_string();
        request.release_mode_override = Some("auto".to_string());
        assert_eq!(
            validate_create_printer(&request),
            Err(PrintDeviceValidationError::InvalidReleaseMode)
        );
    }

    #[test]
    fn release_lease_requires_confirm_and_reason() {
        assert_eq!(
            validate_release_device_lease(&ReleaseDeviceLeaseRequest {
                reason: "打印机撤场".to_string(),
                confirm: false,
            }),
            Err(PrintDeviceValidationError::ConfirmationRequired)
        );
        assert_eq!(
            validate_release_device_lease(&ReleaseDeviceLeaseRequest {
                reason: "  ".to_string(),
                confirm: true,
            }),
            Err(PrintDeviceValidationError::ReasonRequired)
        );
    }
}

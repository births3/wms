pub use crate::document_numbering::{
    DocumentNumberRule, DocumentNumberRuleListResponse, SetDocumentNumberRuleEnabledRequest,
    UpsertDocumentNumberRuleRequest,
};
pub use crate::print_template::{
    PrintFieldDefinition, PrintFieldDefinitionListResponse, PrintFieldLibraryListResponse,
    PrintFieldLibrarySummary, PrintRecord, PrintTemplateBinding, PrintTemplateListResponse,
    PrintTemplatePreviewRequest, PrintTemplatePreviewResponse, PrintTemplatePrintRequest,
    PrintTemplateScope, PrintTemplateSummary, PrintTemplateVersion,
    PrintTemplateVersionListResponse, ResolvePrintTemplateRequest, ResolvePrintTemplateResponse,
    SavePrintTemplateRequest,
};
pub use crate::role_management::{
    BatchAssignRolesRequest, BatchAssignRolesResponse, CreateRoleRequest, CreateUserRequest,
    DeleteRoleResponse, PermissionListResponse, PermissionResponse, ReplaceRolePermissionsRequest,
    RoleListResponse, RoleResponse, RoleUserListResponse, RoleUserResponse, UpdateRoleRequest,
};
pub use crate::tms_plus::{
    ReceiveTmsRoutePlanRequest, TmsRoutePlan, TmsRouteStop, TmsRouteStopRequest,
};
#[allow(unused_imports)]
pub use wms_domain::{
    AdminMenuButtonPermission, AdminMenuNode, AdminMenuTreeResponse, AdminMenuVersion, ApiKey,
    ApiKeyListResponse, ApiKeyRotationResponse, AuditActor, AuditArchivePartitionState,
    AuditArchivePartitionStateListResponse, AuditArchiveRunRequest, AuditArchiveRunResponse,
    AuditEvent, AuditEventListResponse, AuthRevocationResponse, AuthSession,
    AuthSessionListResponse, AuthSessionRevokeResponse, AuthUserStatusRequest,
    BatchCreateLocationsRequest, BatchEnableAdminMenuRequest, BillingAccount,
    BillingChargeCalculation, BillingContract, BillingRule, BillingStatement, BusinessArchiveJob,
    BusinessRetentionPolicy, BusinessRetentionPolicyListResponse, CalculateBillingChargesRequest,
    CancelExpressWaybillRequest, CancelInventoryRecallRequest, CancelReceivingOrderRequest,
    ChangeInventoryStatusRequest, ColdChainDevice, CompletePickTaskRequest, ConfigEntry,
    ConfirmBillingStatementRequest, ConfirmContainerRecoveryRequest, ContainerRecovery,
    CreateAdminMenuNodeRequest, CreateApiKeyRequest, CreateBillingAccountRequest,
    CreateBillingContractRequest, CreateBillingRuleRequest, CreateColdChainDeviceRequest,
    CreateCrossdockPlanRequest, CreateCustomerAddressRequest, CreateCustomerRequest,
    CreateExpressWaybillRequest, CreateH4ApprovalRequest, CreateLocationRequest,
    CreateMaintenanceRecordRequest, CreateOutboundOrderLineRequest, CreateOutboundOrderRequest,
    CreateOutboundWaveRequest, CreatePackJobRequest, CreatePackingStationRequest,
    CreateProductRequest, CreateReceivingOrderRequest, CreateRetailReplenishmentSuggestionRequest,
    CreateSpecialDrugCategoryRequest, CreateSupplierRequest, CreateWarehouseRequest,
    CreateWarehouseZoneRequest, CrossdockPlan, CurrentUser, Customer, CustomerAddress,
    CustomerAddressListResponse, CustomerListResponse, CustomerProfile, CustomerQualification,
    DisableSystemDictionaryItemRequest, DisposeTemperatureExcursionRequest,
    DocumentNumberAllocation, DocumentNumberAllocationListResponse, DriverTask,
    DriverTaskListResponse, DualPersonPolicy, DualPersonPolicyResponse, DualPersonPolicyRule,
    DualPersonPolicyRuleListQuery, DualPersonPolicyRuleListResponse, DualPersonPolicyScope,
    ErrorResponse, EventDelivery, EventDeliveryListResponse, EventDeliveryNackRequest,
    ExecuteMappingRequest, ExecuteMappingResponse, ExpireInventoryBatchesRequest, ExpressCarrier,
    ExpressCarrierListResponse, ExpressRoutingRule, ExpressRoutingRuleListResponse,
    ExpressTrackingEvent, ExpressTrackingResponse, ExpressWaybill, FeatureFlagArchiveRequest,
    FeatureFlagArchiveResult, FeatureFlagBatchImportRequest, FeatureFlagBatchImportResult,
    FeatureFlagConfig, FeatureFlagExportResponse, FeatureFlagMigrationResult,
    FeatureFlagReconcileReport, FeatureFlagSourceSwitchRequest, FeatureFlagSourceSwitchResponse,
    ForceCloseShortageRequest, GenerateBillingStatementRequest, GspLedgerReport, GspLedgerRow,
    H4ApprovalCallbackRequest, H4ApprovalRecord, H4NotificationConfig,
    H4NotificationConfigListResponse, H4NotificationRecord, H4NotificationRecordListResponse,
    H4WechatSettings, H4WechatSettingsResponse, H4WechatSettingsTestResponse,
    HandleInventoryAlertRequest, HealthzResponse, IngestTemperatureExcursionRequest,
    IngestTemperatureReadingRequest, IngestTransitTemperatureRequest, InspectReceivingOrderRequest,
    InspectionSignatureRecord, InventoryAbcClassification, InventoryAbcListResponse,
    InventoryAbcQuery, InventoryAlertEvent, InventoryAlertListResponse, InventoryAlertQuery,
    InventoryBatch, InventoryBatchListResponse, InventoryBatchTrace, InventoryMovement,
    InventoryRecallImpact, InventoryRelocation, InventoryStatusChange, InventoryStatusTransition,
    InventoryStatusTransitionListResponse, Location, LocationHistoryProductShare,
    LocationHistoryQuery, LocationHistoryResponse, LocationHistoryRisk, LocationListResponse,
    LoginRequest, LoginResponse, MaintenanceRecord, MaintenanceRecordListResponse, MaintenanceTask,
    MaintenanceTaskListResponse, MappingDictionary, MappingQueueItem, MappingRule,
    MappingTraceResponse, MarkInventoryRecallRequest, OutboundOrder, OutboundOrderLine,
    OutboundOrderListResponse, OutboundWave, OutboundWaveListResponse, OverrideInventoryAbcRequest,
    PackJob, PackingStation, PageMeta, PasswordChangeRequest, PlanBusinessArchiveJobRequest,
    PrintWaybillRequest, Product, ProductListResponse, PublishAdminMenuRequest,
    PutawayInventoryRequest, PutawayLocationRecommendation, PutawayRecommendationQuery,
    PutawayRecommendationResponse, PutawayRecord, PutawayRequest, PutawayStrategyProfile,
    PutawayStrategyProfileListResponse, ReceiveReceivingOrderRequest, ReceiveTmsDispatchRequest,
    ReceivingDashboardQuery, ReceivingDashboardResponse, ReceivingDashboardRow,
    ReceivingInspectionRecord, ReceivingOrder, ReceivingOrderLine, ReceivingOrderListResponse,
    ReceivingOrderPrintData, ReceivingOrderReceipt, ReceivingReceiptDetails,
    RecomputeInventoryAbcRequest, RejectReceivingOrderRequest, RelocateInventoryRequest,
    ReportQueryRequest, ReportQueryResponse, ReportRow, ResilienceStatus,
    ResolveDualPersonPolicyQuery, RetailReplenishmentSuggestion, ReviewOutboundOrderLineRequest,
    ReviewOutboundOrderRequest, RollbackAdminMenuRequest, RotateApiKeyRequest,
    SendH4NotificationRequest, ShipOutboundOrderRequest, ShippedCustomerHint,
    SignInspectionRequest, SpecialDrugCategory, SpecialDrugCategoryListResponse,
    StateMachineDefinition, StateMachineDefinitionListResponse, StateMachineState,
    StateMachineTransition, StateTransitionValidationResponse, StoreDashboardResponse, Supplier,
    SupplierListResponse, SystemDictionaryCategory, SystemDictionaryImpactPreview,
    SystemDictionaryImpactReference, SystemDictionaryItem, SystemDictionaryItemListResponse,
    TemperatureExcursionDispositionResponse, TemperatureExcursionEvent,
    TemperatureExcursionEventListResponse, TemperatureReading, TmsDispatch,
    TraceabilityOutboundReport, TraceabilityOutboundReportRequest, TraceabilityStatusChangeEvent,
    TransitTemperatureReading, UpdateAdminMenuNodeRequest, UpdateCustomerAddressRequest,
    UpdateCustomerRequest, UpdateLocationRequest, UpdateProductRequest,
    UpdateReceivingOrderRequest, UpdateSpecialDrugCategoryRequest, UpdateSupplierRequest,
    UpdateWarehouseRequest, UpdateWarehouseZoneRequest, UpsertAdminMenuButtonPermissionRequest,
    UpsertCustomerProfileRequest, UpsertDualPersonPolicyRuleRequest, UpsertExpressCarrierRequest,
    UpsertExpressRoutingRuleRequest, UpsertH4NotificationConfigRequest,
    UpsertH4WechatSettingsRequest, UpsertInventoryStatusTransitionRequest,
    UpsertPutawayStrategyProfileRequest, UpsertSystemDictionaryItemRequest, Warehouse,
    WarehouseListResponse, WarehouseZone, WarehouseZoneListResponse, WeighPackJobRequest,
};
pub use wms_domain::{
    AlertActionRequest, AlertChangeEvent, AlertChangeListResponse, AlertEscalationLevelDraft,
    AlertEscalationRule, AlertEscalationRuleListResponse, AlertExportJob, AlertInstance,
    AlertInstanceListQuery, AlertInstanceListResponse, AlertMonthlyMetric, AlertRankingItem,
    AlertStatisticsResponse, CreateAlertExportRequest, GspAlertLifecycleRecord,
    GspAlertLifecycleReport, UpsertAlertEscalationRuleRequest,
};
pub use wms_domain::{
    AlertDefinition, AlertDefinitionChangeOperation, AlertDefinitionDraft,
    AlertDefinitionListQuery, AlertDefinitionListResponse, SubmitAlertDefinitionChangeRequest,
};
pub use wms_domain::{
    ArriveDockAppointmentRequest, CancelDockAppointmentRequest, CreateDockAppointmentRequest,
    CreateDockImportRequest, CreateDockRequest, Dock, DockAppointment,
    UpdateDockAppointmentRequest, UpdateDockRequest,
};
pub use wms_domain::{
    CreateQualityLiaisonRequest, QualityLiaisonApprovalCallbackRequest, QualityLiaisonOrder,
    QualityLiaisonTypeConfig, UpsertQualityLiaisonTypeRequest,
};
pub use wms_domain::{
    CreateStockLossOrderRequest, CreateStockSurplusOrderRequest, ExecuteStockLossOrderRequest,
    ExecuteStockSurplusOrderRequest, StockAdjustmentSource, StockAdjustmentStatus, StockLossOrder,
    StockLossQualityApprovalRequest, StockLossReason, StockSurplusOrder,
    StockSurplusQualityApprovalRequest, StockSurplusReason,
};
pub use wms_domain::{
    CreateWarehouseTaskRequest, TaskGroup, TaskGroupListResponse, TaskGroupMemberQualification,
    TaskListQuery, TaskPriorityRule, TaskTransitionAction, TaskWorker, TaskWorkerListResponse,
    TransitionWarehouseTaskRequest, UpsertTaskGroupRequest, UpsertTaskPriorityRuleRequest,
    WarehouseTask, WarehouseTaskListResponse,
};
pub use wms_domain::{
    SetTaskTypeEnabledRequest, TaskType, TaskTypeListResponse, UpsertTaskTypeRequest,
};

#[allow(dead_code)]
fn _dock_appointment_openapi_type_use(
    _request: Option<CreateDockAppointmentRequest>,
    _update_request: Option<UpdateDockAppointmentRequest>,
    _cancel_request: Option<CancelDockAppointmentRequest>,
    _arrive_request: Option<ArriveDockAppointmentRequest>,
    _appointment: Option<DockAppointment>,
) {
}

#[allow(dead_code)]
fn _h4_openapi_type_use(
    _create_approval: Option<CreateH4ApprovalRequest>,
    _approval_callback: Option<H4ApprovalCallbackRequest>,
    _approval: Option<H4ApprovalRecord>,
    _config: Option<H4NotificationConfig>,
    _config_list: Option<H4NotificationConfigListResponse>,
    _wechat_settings: Option<H4WechatSettings>,
    _wechat_settings_response: Option<H4WechatSettingsResponse>,
    _wechat_settings_test_response: Option<H4WechatSettingsTestResponse>,
    _record: Option<H4NotificationRecord>,
    _record_list: Option<H4NotificationRecordListResponse>,
    _send: Option<SendH4NotificationRequest>,
    _upsert_config: Option<UpsertH4NotificationConfigRequest>,
    _upsert_wechat_settings: Option<UpsertH4WechatSettingsRequest>,
) {
}

mod alert_definition;
mod alert_runtime;
mod core;
mod customer_addresses;
mod customer_profile;
mod dock;
mod dock_appointment;
mod drug_inspection;
mod dual_person_policy;
mod extensions;
mod h8_erp;
mod inventory_count;
mod maintenance;
mod quality_liaison;
mod stock_adjustment;
mod task_engine;
mod task_type;

pub(crate) use alert_definition::*;
pub(crate) use alert_runtime::*;
pub(crate) use core::*;
pub(crate) use customer_addresses::*;
pub(crate) use customer_profile::*;
pub(crate) use dock::*;
pub(crate) use dock_appointment::*;
pub(crate) use drug_inspection::*;
pub(crate) use dual_person_policy::*;
pub(crate) use extensions::*;
pub(crate) use h8_erp::*;
pub(crate) use inventory_count::*;
pub(crate) use maintenance::*;
pub(crate) use quality_liaison::*;
pub(crate) use stock_adjustment::*;
pub(crate) use task_engine::*;
pub(crate) use task_type::*;

pub use wms_domain::{
    ApproveInventoryCountRequest, CreateInventoryCountRequest, InventoryCount, InventoryCountLine,
    SubmitInventoryCountLineRequest, UpdateColdChainDeviceRequest,
};

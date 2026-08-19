pub(crate) use crate::dock_appointment_handlers::DockAppointmentListResponse;
pub use crate::document_numbering::{
    DocumentNumberRule, DocumentNumberRuleListResponse, SetDocumentNumberRuleEnabledRequest,
    UpsertDocumentNumberRuleRequest,
};
pub(crate) use crate::drug_inspection_document_handlers::ReviewQueueListResponse;
pub(crate) use crate::drug_inspection_stamp_handlers::CopyJobListResponse;
pub use crate::print_template::{
    GeneratePrintFieldLibraryDraftRequest, PrintFieldDefinition, PrintFieldDefinitionListResponse,
    PrintFieldLibraryListResponse, PrintFieldLibrarySummary, PrintFieldLibraryVersion, PrintRecord,
    PrintTemplateBinding, PrintTemplateListResponse, PrintTemplatePreviewRequest,
    PrintTemplatePreviewResponse, PrintTemplatePrintRequest, PrintTemplateScope,
    PrintTemplateSummary, PrintTemplateVersion, PrintTemplateVersionListResponse,
    ResolvePrintTemplateRequest, ResolvePrintTemplateResponse, SavePrintTemplateRequest,
    SetPrintTemplateEnabledRequest, UpdatePrintFieldDefinitionRequest,
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
    BatchCreateLocationsRequest, BatchCreateLpnContainerRequest, BatchEnableAdminMenuRequest,
    BatchGenerateLocationsRequest, BatchGenerateLocationsResponse, BillingAccount,
    BillingChargeCalculation, BillingContract, BillingRule, BillingStatement,
    BindReplenishmentLocationsRequest, BindReplenishmentLocationsResponse, BusinessArchiveJob,
    BusinessRetentionPolicy, BusinessRetentionPolicyListResponse, CalculateBillingChargesRequest,
    CancelExpressWaybillRequest, CancelInventoryRecallRequest, CancelReceivingOrderRequest,
    CancelReplenishmentTaskRequest, ChangeInventoryStatusRequest, ClaimH8ErpMessageRequest,
    ClaimReplenishmentTaskRequest, ColdChainDevice, CompletePickTaskRequest, ConfigEntry,
    ConfirmBillingStatementRequest, ConfirmContainerRecoveryRequest,
    ConfirmReplenishmentTaskRequest, ContainerRecovery, CreateAdminMenuNodeRequest,
    CreateApiKeyRequest, CreateBillingAccountRequest, CreateBillingContractRequest,
    CreateBillingRuleRequest, CreateColdChainDeviceRequest, CreateCrossdockPlanRequest,
    CreateCustomerAddressRequest, CreateCustomerRequest, CreateExpressWaybillRequest,
    CreateH4ApprovalRequest, CreateH8ErpConnectorRequest, CreateLocationRequest,
    CreateLpnContainerRequest, CreateMaintenanceRecordRequest, CreateOutboundOrderLineRequest,
    CreateOutboundOrderRequest, CreateOutboundWaveRequest, CreatePackJobRequest,
    CreatePackingStationRequest, CreateProductRequest, CreatePurchaseReturnRequest,
    CreateReceivingOrderRequest, CreateReplenishmentTaskRequest,
    CreateRetailReplenishmentSuggestionRequest, CreateSpecialDrugCategoryRequest,
    CreateSupplierRequest, CreateWarehouseRequest, CreateWarehouseZoneRequest, CrossdockPlan,
    CurrentUser, Customer, CustomerAddress, CustomerAddressListResponse, CustomerListResponse,
    CustomerProfile, CustomerQualification, DisableSystemDictionaryItemRequest,
    DisposeTemperatureExcursionRequest, DocumentNumberAllocation,
    DocumentNumberAllocationListResponse, DriverTask, DriverTaskListResponse, DualPersonPolicy,
    DualPersonPolicyResponse, DualPersonPolicyRule, DualPersonPolicyRuleListQuery,
    DualPersonPolicyRuleListResponse, DualPersonPolicyScope, ErrorResponse, EventDelivery,
    EventDeliveryListResponse, EventDeliveryNackRequest, ExpireInventoryBatchesRequest,
    ExpressCarrier, ExpressCarrierListResponse, ExpressRoutingRule, ExpressRoutingRuleListResponse,
    ExpressTrackingEvent, ExpressTrackingResponse, ExpressWaybill, FeatureFlagArchiveRequest,
    FeatureFlagArchiveResult, FeatureFlagBatchImportRequest, FeatureFlagBatchImportResult,
    FeatureFlagConfig, FeatureFlagExportResponse, FeatureFlagMigrationResult,
    FeatureFlagReconcileReport, FeatureFlagSourceSwitchRequest, FeatureFlagSourceSwitchResponse,
    ForceCloseShortageRequest, GenerateBillingStatementRequest, GspLedgerReport, GspLedgerRow,
    H4ApprovalCallbackRequest, H4ApprovalRecord, H4NotificationConfig,
    H4NotificationConfigListResponse, H4NotificationRecord, H4NotificationRecordListResponse,
    H4WechatSettings, H4WechatSettingsResponse, H4WechatSettingsTestResponse, H8ErpConnector,
    H8ErpConnectorListResponse, H8ErpConnectorTestResult, H8ErpInterfaceTableDetail,
    H8ErpInterfaceTableField, H8ErpInterfaceTableListResponse, H8ErpInterfaceTableQuery,
    H8ErpInterfaceTableRow, H8ErpMessage, H8ErpMessageAttempt, H8ErpMessageDetail,
    H8ErpMessageListResponse, H8ErpMessageStats, HandleInventoryAlertRequest, HealthzResponse,
    IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
    IngestTransitTemperatureRequest, InspectReceivingOrderRequest, InspectionSignatureRecord,
    InventoryAbcClassification, InventoryAbcListResponse, InventoryAbcQuery, InventoryAlertEvent,
    InventoryAlertListResponse, InventoryAlertQuery, InventoryBatch, InventoryBatchListResponse,
    InventoryBatchTrace, InventoryCountListResponse, InventoryMovement, InventoryRecallImpact,
    InventoryRelocation, InventoryRelocationListResponse, InventoryStatusChange,
    InventoryStatusTransition, InventoryStatusTransitionListResponse, Location,
    LocationHistoryProductShare, LocationHistoryQuery, LocationHistoryResponse,
    LocationHistoryRisk, LocationListResponse, LoginRequest, LoginResponse, LpnContainer,
    LpnContainerListResponse, LpnContainerTypePolicy, MaintenanceRecord,
    MaintenanceRecordListResponse, MaintenanceTask, MaintenanceTaskListResponse,
    MapParameterRequest, MapParameterResponse, MarkInventoryRecallRequest, OutboundOrder,
    OutboundOrderLine, OutboundOrderListResponse, OutboundWave, OutboundWaveListResponse,
    OverrideInventoryAbcRequest, PackJob, PackingStation, PageMeta, ParameterMappingStatus,
    PasswordChangeRequest, PickReplenishmentTaskRequest, PlanBusinessArchiveJobRequest,
    PrintWaybillRequest, Product, ProductListResponse, ProductMappingTrace,
    ProductMappingTraceInput, ProductPackagingLevel, ProductPackagingLevelInput,
    PublishAdminMenuRequest, PurchaseReturnOrder, PurchaseReturnOrderListResponse,
    PurgeH8ErpMessagesRequest, PurgeH8ErpMessagesResponse, PutawayInventoryRequest,
    PutawayLocationRecommendation, PutawayRecommendationQuery, PutawayRecommendationResponse,
    PutawayRecord, PutawayRequest, PutawayStrategyProfile, PutawayStrategyProfileListResponse,
    ReassignReplenishmentTaskRequest, ReceiveReceivingOrderRequest, ReceiveTmsDispatchRequest,
    ReceivingDashboardQuery, ReceivingDashboardResponse, ReceivingDashboardRow,
    ReceivingInspectionRecord, ReceivingOrder, ReceivingOrderLine, ReceivingOrderListResponse,
    ReceivingOrderPrintData, ReceivingOrderReceipt, ReceivingReceiptDetails,
    RecomputeInventoryAbcRequest, RejectPurchaseReturnRequest, RejectReceivingOrderRequest,
    ReleaseContainerQualityLockRequest, ReleaseContainerQualityLockResponse,
    RelocateInventoryRequest, ReplayH8ErpMessageRequest, ReplenishmentLocationGroup,
    ReplenishmentLocationGroupListResponse, ReplenishmentPreviewResponse, ReplenishmentStrategy,
    ReplenishmentStrategyListResponse, ReplenishmentTask, ReplenishmentTaskListResponse,
    ReportQueryRequest, ReportQueryResponse, ReportRow, ResilienceStatus,
    ResolveDualPersonPolicyQuery, RetailReplenishmentSuggestion, ReturnReplenishmentTaskRequest,
    ReviewOutboundOrderLineRequest, ReviewOutboundOrderRequest, RollbackAdminMenuRequest,
    RotateApiKeyRequest, SalesReturnReceivingBatch, SendH4NotificationRequest,
    ShipOutboundOrderRequest, ShippedCustomerHint, SignInspectionRequest, SpecialDrugCategory,
    SpecialDrugCategoryListResponse, StateMachineDefinition, StateMachineDefinitionListResponse,
    StateMachineState, StateMachineTransition, StateTransitionValidationResponse,
    StoreDashboardResponse, Supplier, SupplierListResponse, SystemDictionaryCategory,
    SystemDictionaryImpactPreview, SystemDictionaryImpactReference, SystemDictionaryItem,
    SystemDictionaryItemListResponse, TemperatureExcursionDispositionResponse,
    TemperatureExcursionEvent, TemperatureExcursionEventListResponse, TemperatureReading,
    TmsDispatch, TraceabilityOutboundReport, TraceabilityOutboundReportRequest,
    TraceabilityStatusChangeEvent, TransitTemperatureReading, UnlockSkippedBatch,
    UpdateAdminMenuNodeRequest, UpdateCustomerAddressRequest, UpdateCustomerRequest,
    UpdateH8ErpConnectorRequest, UpdateLocationRequest, UpdateLpnContainerRequest,
    UpdateProductRequest, UpdateReceivingOrderRequest, UpdateSpecialDrugCategoryRequest,
    UpdateSupplierRequest, UpdateWarehouseRequest, UpdateWarehouseZoneRequest,
    UpsertAdminMenuButtonPermissionRequest, UpsertCustomerProfileRequest,
    UpsertDualPersonPolicyRuleRequest, UpsertExpressCarrierRequest,
    UpsertExpressRoutingRuleRequest, UpsertH4NotificationConfigRequest,
    UpsertH4WechatSettingsRequest, UpsertInventoryStatusTransitionRequest,
    UpsertLpnContainerTypePolicyRequest, UpsertPutawayStrategyProfileRequest,
    UpsertReplenishmentLocationGroupRequest, UpsertReplenishmentStrategyRequest,
    UpsertSystemDictionaryItemRequest, Warehouse, WarehouseListResponse, WarehouseZone,
    WarehouseZoneListResponse, WeighPackJobRequest,
};
pub use wms_domain::{
    AggregationDimension, AggregationFieldCatalogResponse, AggregationFieldCode,
    AggregationFieldDefinition, AggregationGroupKeyItem, AggregationMethod,
    AggregationRuleTestGroup, AggregationRuleTestResult, AggregationRuleVersion,
    AggregationRuleVersionListResponse, CreateAggregationRuleDraftRequest, CreateCutoffPlanRequest,
    CutoffDateException, CutoffPlan, CutoffPlanListResponse, CutoffPlanScope,
    DeliveryNoteCandidate, DeliveryNoteCandidateListResponse, DeliveryNoteGroup,
    DeliveryNoteGroupListItem, DeliveryNoteGroupListResponse, ManualDeliveryNoteCutoffRequest,
    PublishRouteBindingRequest, RouteBinding, RouteBindingListResponse, TestAggregationRuleRequest,
    WeeklyCutoffSlot,
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
    CategoryPdfOutputListResponse, CategoryPdfPreparation, CreatePrintSuiteDraftRequest,
    PrintDocumentCategoryListResponse, PrintSuiteInstanceListResponse, PrintSuiteTestResult,
    PrintSuiteVersion, PrintSuiteVersionListResponse, SelectCategoryPdfsRequest,
    TestPrintSuiteRequest,
};
pub use wms_domain::{
    CompleteArchiveRevisionRequest, CreateQualityLiaisonRequest,
    QualityLiaisonApprovalCallbackRequest, QualityLiaisonOrder, QualityLiaisonTypeConfig,
    UpsertQualityLiaisonTypeRequest,
};
pub use wms_domain::{
    CreatePrintSiteRequest, CreatePrinterRequest, CreatePrinterTrayRequest,
    CreateSiteOwnerMappingRequest, DeviceLease, DeviceLeaseListResponse, PrintSite,
    PrintSiteListResponse, PrintSiteOwnerMapping, PrintSiteOwnerMappingListResponse, Printer,
    PrinterListResponse, PrinterTestPrint, PrinterTray, PrinterTrayListResponse,
    ReleaseDeviceLeaseRequest, TestPrintRequest, UpdatePrinterRequest, UpdatePrinterTrayRequest,
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
mod drug_inspection_document;
mod dual_person_policy;
mod extensions;
mod file_attachment;
mod h8_erp;
mod inventory_count;
mod lpn_container;
mod maintenance;
mod outbound;
mod print_device;
mod print_orchestration;
mod quality_liaison;
mod reconciliation;
mod replenishment;
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
pub(crate) use drug_inspection_document::*;
pub(crate) use dual_person_policy::*;
pub(crate) use extensions::*;
pub(crate) use file_attachment::*;
pub(crate) use h8_erp::*;
pub(crate) use inventory_count::*;
pub(crate) use lpn_container::*;
pub(crate) use maintenance::*;
pub(crate) use outbound::*;
pub(crate) use print_device::*;
pub(crate) use print_orchestration::*;
pub(crate) use quality_liaison::*;
pub(crate) use reconciliation::*;
pub(crate) use replenishment::*;
pub(crate) use stock_adjustment::*;
pub(crate) use task_engine::*;
pub(crate) use task_type::*;

pub use wms_domain::{
    ApproveDrugInspectionCopyOversizeRequest, ConfirmFileUploadRequest,
    CreateDrugInspectionCorrectionRequest, CreateDrugInspectionStampVersionRequest,
    CreateDrugInspectionVersionRequest, CreateFileUploadRequest,
    CreateUpstreamDeliveryVersionRequest, DrugInspectionCustomerCopyJob,
    DrugInspectionReportVersion, DrugInspectionReviewQueueEntry, DrugInspectionStampVersion,
    FileAttachment, FileAttachmentDownloadUrlResponse, FileUploadSessionResponse,
    InboundDocumentEntry, InboundDocumentEntryListResponse, ReusableDrugInspectionReportResponse,
    ReuseDrugInspectionReportRequest, ReuseDrugInspectionReportResponse,
    ReviewDrugInspectionStampVersionRequest, ReviewDrugInspectionVersionRequest,
    UpdateDrugInspectionDraftRequest, UpstreamDeliveryDocumentVersion,
};
pub use wms_domain::{
    ApproveInventoryCountRequest, CreateInventoryCountRequest, InventoryCount, InventoryCountLine,
    SubmitInventoryCountLineRequest, UpdateColdChainDeviceRequest,
};

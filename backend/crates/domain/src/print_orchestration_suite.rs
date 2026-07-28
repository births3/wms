/// H9 打印组套匹配层级；解析优先级固定为送货地址、客户、线路、货主+仓库默认。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrintSuiteScope {
    DeliveryAddress,
    Customer,
    Route,
    WarehouseDefault,
}

/// H9 打印项来源模式（ADR-0041）。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrintSuiteSourceMode {
    Rendered,
    ExternalFile,
}

/// H9 必需单据未就绪时的受控就绪策略（AC7）。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrintSuiteReadyPolicy {
    WaitHoldInstance,
    PauseAgentQueue,
}

/// H9 打印项失败策略；必需项永远不能跳过（ADR-0041）。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrintSuiteFailurePolicy {
    PauseSuite,
    SkipAndContinue,
}

/// 创建 H9 打印组套草稿的一条有序打印项。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteItemInput {
    pub category_code: String,
    pub copies: i32,
    pub sort_order: i32,
    pub output_slot: String,
    pub required: bool,
    pub ready_policy: PrintSuiteReadyPolicy,
    pub failure_policy: PrintSuiteFailurePolicy,
    pub source_mode: PrintSuiteSourceMode,
    pub template_version_id: Option<Uuid>,
    pub external_file_ref: Option<String>,
}

/// 创建 H9 打印组套草稿版本。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CreatePrintSuiteDraftRequest {
    pub name: String,
    pub warehouse_id: Uuid,
    pub scope: PrintSuiteScope,
    pub customer_id: Option<Uuid>,
    pub delivery_address_id: Option<Uuid>,
    pub route_code: Option<String>,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub items: Vec<PrintSuiteItemInput>,
}

/// H9 打印组套版本中的一条打印项。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteItem {
    pub id: Uuid,
    pub category_code: String,
    pub category_name: String,
    pub copies: i32,
    pub sort_order: i32,
    pub output_slot: String,
    pub required: bool,
    pub ready_policy: PrintSuiteReadyPolicy,
    pub failure_policy: PrintSuiteFailurePolicy,
    pub source_mode: PrintSuiteSourceMode,
    pub template_version_id: Option<Uuid>,
    pub external_file_ref: Option<String>,
}

/// H9 打印组套不可变版本。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteVersion {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub version_no: i32,
    pub name: String,
    pub status: String,
    pub warehouse_id: Uuid,
    pub scope: PrintSuiteScope,
    pub customer_id: Option<Uuid>,
    pub delivery_address_id: Option<Uuid>,
    pub route_code: Option<String>,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub items: Vec<PrintSuiteItem>,
    pub tested_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// H9 打印组套版本列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteVersionListResponse {
    pub data: Vec<PrintSuiteVersion>,
}

/// 用真实归集组样本测试一版打印组套。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TestPrintSuiteRequest {
    pub group_ids: Vec<Uuid>,
}

/// H9 权威文件绑定（AC6/AC8：文件 ID + 版本 + 内容哈希，不落临时 URL）。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteFileBinding {
    pub file_id: Uuid,
    pub file_ref: String,
    pub file_version: i32,
    pub content_hash: String,
}

/// 一条打印项对一个样本归集组的就绪判定（AC5）。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteItemReadiness {
    pub category_code: String,
    pub category_name: String,
    pub source_mode: PrintSuiteSourceMode,
    pub required: bool,
    pub ready: bool,
    pub missing: Vec<String>,
    pub file_bindings: Vec<PrintSuiteFileBinding>,
}

/// 一个样本归集组的组套解析与就绪预检结果。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteTestSample {
    pub group_id: Uuid,
    pub delivery_note_no: String,
    pub resolved_scope: Option<PrintSuiteScope>,
    pub matches_this_version: bool,
    pub item_readiness: Vec<PrintSuiteItemReadiness>,
}

/// H9 打印组套样本测试结果。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteTestResult {
    pub suite: PrintSuiteVersion,
    pub samples: Vec<PrintSuiteTestSample>,
}

/// H9 组套实例中的一条冻结打印项。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteInstanceItem {
    pub id: Uuid,
    pub category_code: String,
    pub copies: i32,
    pub sort_order: i32,
    pub output_slot: String,
    pub required: bool,
    pub ready_policy: PrintSuiteReadyPolicy,
    pub failure_policy: PrintSuiteFailurePolicy,
    pub source_mode: PrintSuiteSourceMode,
    pub template_version_id: Option<Uuid>,
    pub external_file_ref: Option<String>,
    pub file_bindings: Vec<PrintSuiteFileBinding>,
    pub ready: bool,
    pub missing: Vec<String>,
}

/// H9 组套实例：冻结组套版本、规则版本、源单据和策略快照。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteInstance {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub group_id: Uuid,
    pub delivery_note_no: String,
    pub suite_version_id: Uuid,
    pub suite_version_no: i32,
    pub suite_snapshot: serde_json::Value,
    pub aggregation_rule_version_id: Option<Uuid>,
    pub aggregation_rule_version_no: Option<i32>,
    pub source_documents: serde_json::Value,
    pub status: String,
    pub hold_scope: Option<String>,
    pub items: Vec<PrintSuiteInstanceItem>,
    pub created_at: DateTime<Utc>,
}

/// H9 组套实例列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintSuiteInstanceListResponse {
    pub data: Vec<PrintSuiteInstance>,
}

/// H9 分类 PDF 产物；稳定事实只保存 H-FILE ID、版本与哈希，不保存临时访问 URL。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CategoryPdfOutput {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub instance_item_id: Uuid,
    pub sort_order: i32,
    pub category_code: String,
    pub source_mode: PrintSuiteSourceMode,
    pub source_data_version: Option<String>,
    pub source_file_bindings: Vec<PrintSuiteFileBinding>,
    pub template_version_id: Option<Uuid>,
    pub attachment_id: Option<Uuid>,
    pub content_hash: Option<String>,
    pub processing_status: String,
    pub failure_reason: Option<String>,
    pub retention_policy: String,
    pub cache_expires_at: Option<DateTime<Utc>>,
    pub attempt_count: i32,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

/// 一次分类 PDF 准备结果；同一实例只能复用最初的幂等键重试。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CategoryPdfPreparation {
    pub instance_id: Uuid,
    pub idempotency_key: String,
    pub status: String,
    pub outputs: Vec<CategoryPdfOutput>,
}

/// H9 分类 PDF 产物列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CategoryPdfOutputListResponse {
    pub data: Vec<CategoryPdfOutput>,
    pub preparation_status: Option<String>,
    pub retry_idempotency_key: Option<String>,
}

/// 临时合并或下载的分类 PDF 选择；空数组表示全部已就绪分类。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct SelectCategoryPdfsRequest {
    pub category_pdf_ids: Vec<Uuid>,
}

/// M1 系统字典 print_document_category 的一条受控分类。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintDocumentCategory {
    pub item_code: String,
    pub item_name: String,
    pub source_mode: PrintSuiteSourceMode,
}

/// M1 系统字典 print_document_category 列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PrintDocumentCategoryListResponse {
    pub data: Vec<PrintDocumentCategory>,
}

/// H9 打印组套草稿的纯业务校验失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintSuiteValidationError {
    NameRequired,
    NameTooLong,
    IdentifierRequired,
    ScopeMismatch,
    InvalidEffectivePeriod,
    ItemsRequired,
    InvalidCopies,
    InvalidSortSequence,
    OutputSlotRequired,
    OutputSlotTooLong,
    RequiredItemCannotSkip,
    RenderedItemNeedsTemplateVersion,
    ExternalFileItemNeedsFileRef,
    InvalidExternalFileRef,
}

/// 校验打印组套草稿中不依赖数据库的业务约束。
pub fn validate_print_suite(
    request: &CreatePrintSuiteDraftRequest,
) -> Result<(), PrintSuiteValidationError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(PrintSuiteValidationError::NameRequired);
    }
    if name.chars().count() > 100 {
        return Err(PrintSuiteValidationError::NameTooLong);
    }
    if request.warehouse_id.is_nil() {
        return Err(PrintSuiteValidationError::IdentifierRequired);
    }
    let scope_valid = match request.scope {
        PrintSuiteScope::DeliveryAddress => {
            request.customer_id.is_some_and(|id| !id.is_nil())
                && request.delivery_address_id.is_some_and(|id| !id.is_nil())
                && request.route_code.is_none()
        }
        PrintSuiteScope::Customer => {
            request.customer_id.is_some_and(|id| !id.is_nil())
                && request.delivery_address_id.is_none()
                && request.route_code.is_none()
        }
        PrintSuiteScope::Route => {
            request.customer_id.is_none()
                && request.delivery_address_id.is_none()
                && request
                    .route_code
                    .as_deref()
                    .is_some_and(|code| !code.trim().is_empty() && code.chars().count() <= 64)
        }
        PrintSuiteScope::WarehouseDefault => {
            request.customer_id.is_none()
                && request.delivery_address_id.is_none()
                && request.route_code.is_none()
        }
    };
    if !scope_valid {
        return Err(PrintSuiteValidationError::ScopeMismatch);
    }
    if request
        .effective_to
        .is_some_and(|effective_to| effective_to <= request.effective_from)
    {
        return Err(PrintSuiteValidationError::InvalidEffectivePeriod);
    }
    if request.items.is_empty() {
        return Err(PrintSuiteValidationError::ItemsRequired);
    }
    let mut sort_orders = HashSet::new();
    for item in &request.items {
        if !(1..=20).contains(&item.copies) {
            return Err(PrintSuiteValidationError::InvalidCopies);
        }
        if item.sort_order <= 0 || !sort_orders.insert(item.sort_order) {
            return Err(PrintSuiteValidationError::InvalidSortSequence);
        }
        let output_slot = item.output_slot.trim();
        if output_slot.is_empty() {
            return Err(PrintSuiteValidationError::OutputSlotRequired);
        }
        if output_slot.chars().count() > 64 {
            return Err(PrintSuiteValidationError::OutputSlotTooLong);
        }
        if item.required && item.failure_policy != PrintSuiteFailurePolicy::PauseSuite {
            return Err(PrintSuiteValidationError::RequiredItemCannotSkip);
        }
        match item.source_mode {
            PrintSuiteSourceMode::Rendered => {
                if item.template_version_id.is_none_or(|id| id.is_nil())
                    || item.external_file_ref.is_some()
                {
                    return Err(PrintSuiteValidationError::RenderedItemNeedsTemplateVersion);
                }
            }
            PrintSuiteSourceMode::ExternalFile => {
                if item.template_version_id.is_some() {
                    return Err(PrintSuiteValidationError::ExternalFileItemNeedsFileRef);
                }
                let Some(file_ref) = item.external_file_ref.as_deref() else {
                    return Err(PrintSuiteValidationError::ExternalFileItemNeedsFileRef);
                };
                let file_ref = file_ref.trim();
                if file_ref.is_empty() {
                    return Err(PrintSuiteValidationError::ExternalFileItemNeedsFileRef);
                }
                // AC6：只允许稳定的 H-FILE 文件来源引用，禁止临时外部 URL。
                if !file_ref.starts_with("h-file:")
                    || file_ref.chars().count() > 200
                    || file_ref.starts_with("http://")
                    || file_ref.starts_with("https://")
                {
                    return Err(PrintSuiteValidationError::InvalidExternalFileRef);
                }
            }
        }
    }
    let mut sorted = sort_orders.into_iter().collect::<Vec<_>>();
    sorted.sort_unstable();
    if sorted
        .iter()
        .enumerate()
        .any(|(index, order)| *order != (index + 1) as i32)
    {
        return Err(PrintSuiteValidationError::InvalidSortSequence);
    }
    Ok(())
}

impl PrintSuiteScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeliveryAddress => "delivery_address",
            Self::Customer => "customer",
            Self::Route => "route",
            Self::WarehouseDefault => "warehouse_default",
        }
    }
}

impl TryFrom<&str> for PrintSuiteScope {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "delivery_address" => Ok(Self::DeliveryAddress),
            "customer" => Ok(Self::Customer),
            "route" => Ok(Self::Route),
            "warehouse_default" => Ok(Self::WarehouseDefault),
            _ => Err(()),
        }
    }
}

impl PrintSuiteSourceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rendered => "rendered",
            Self::ExternalFile => "external_file",
        }
    }
}

impl TryFrom<&str> for PrintSuiteSourceMode {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "rendered" => Ok(Self::Rendered),
            "external_file" => Ok(Self::ExternalFile),
            _ => Err(()),
        }
    }
}

impl PrintSuiteReadyPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WaitHoldInstance => "wait_hold_instance",
            Self::PauseAgentQueue => "pause_agent_queue",
        }
    }
}

impl TryFrom<&str> for PrintSuiteReadyPolicy {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "wait_hold_instance" => Ok(Self::WaitHoldInstance),
            "pause_agent_queue" => Ok(Self::PauseAgentQueue),
            _ => Err(()),
        }
    }
}

impl PrintSuiteFailurePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PauseSuite => "pause_suite",
            Self::SkipAndContinue => "skip_and_continue",
        }
    }
}

impl TryFrom<&str> for PrintSuiteFailurePolicy {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pause_suite" => Ok(Self::PauseSuite),
            "skip_and_continue" => Ok(Self::SkipAndContinue),
            _ => Err(()),
        }
    }
}

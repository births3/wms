# 基础设施模块：技术规格

> 本文档定义系统的横向技术基础设施模块。
> 这些模块**不直接面向用户**，而是被业务模块调用。
> 格式：技术规格（职责/接口/约束/消费方），不使用用户故事格式。

---

## H6 状态机引擎

### 职责

为所有有状态流转的业务实体提供统一的状态机定义、转换执行、事件发布能力。

### 当前落地切片

Wave 1 先落地不可变状态机定义注册表和转换校验 API，覆盖 `asn`、`outbound_order`、`warehouse_task` 三类核心状态图。事务内状态转换、事件发布和 H2 审计写入在业务模块接入执行能力时补齐。

### 消费方

| 业务模块 | 状态机实体 | 状态数 |
|---------|-----------|--------|
| M2 入库 | ASN | 7 + 5 异常态 |
| M4 出库 | 出库订单 | 7 + 3 异常态 |
| M4 退货 | 退货单 | 6 |
| M-TE 任务引擎 | 任务 | 6 + 2 异常态 |
| M-QL 质量联系单 | 联系单 | 4（待审批/已通过/已拒绝/已执行） |
| M9 计费 | 账单 | 3（待确认/已确认/已结算） |

### 接口契约

```rust
// 状态机定义（声明式）
StateMachine::define("asn")
    .state("pending_validation")
    .state("pending_receipt")
    // ...
    .transition("pending_validation", "pending_receipt")
        .guard(|ctx| ctx.validation_passed())
        .action(|ctx| ctx.emit_event(AsnValidated))
    .transition("pending_validation", "validation_error")
        .guard(|ctx| !ctx.validation_passed())
    .build();

// 状态转换执行
state_machine.transition(entity_id, target_state, context)?;

// 事件发布（转换成功后自动发布领域事件）
// 其他模块可订阅状态变更事件
```

### 约束

1. 状态转换必须原子性（事务内完成）
2. 非法转换抛出错误（不允许跳过中间状态）
3. 每次转换自动写入审计追踪（旧状态/新状态/操作人/时间）
4. 支持守卫条件（guard）：转换前校验是否允许
5. 支持转换动作（action）：转换后触发副作用（发事件/通知）
6. 状态机定义不可变（运行时不能修改状态图，只能通过代码变更）

### 波次

Wave 1（基础框架）→ Wave 2（业务模块接入）

---

## H7 导入导出引擎

### 职责

为所有模块提供统一的数据导入（Excel/CSV）和导出（Excel/PDF/CSV）能力。

### 消费方

| 场景 | 模块 | 格式 |
|------|------|------|
| 商品批量导入 | M1 | Excel → 数据库 |
| 库存快照导出 | M3 | 数据库 → Excel |
| GSP 台账导出 | M6 | 数据库 → PDF |
| 计费账单导出 | M9 | 数据库 → Excel/PDF |
| 配置导入导出 | M1-008 | JSON ↔ 数据库 |
| 报表导出 | M6 | 数据库 → Excel + 图表 |

### 接口契约

```rust
// 导出
ExportBuilder::new("inventory_snapshot")
    .format(ExportFormat::Excel)
    .query(inventory_query)
    .columns(vec!["商品编码", "商品名称", "批号", "数量", "库位"])
    .filter(owner_id)
    .build()
    .execute() -> Result<FileHandle>

// 导入
ImportBuilder::new("product_master")
    .format(ImportFormat::Excel)
    .file(uploaded_file)
    .mapping(column_mapping)  // Excel列 → 数据库字段
    .validate(validation_rules)  // 导入前校验
    .on_error(ErrorStrategy::SkipRow)  // 错误行跳过/中止
    .build()
    .execute() -> Result<ImportReport>

// PDF 生成（台账/报告）
PdfBuilder::new("gsp_acceptance_record")
    .template("templates/acceptance.html")
    .data(record_data)
    .build()
    .render() -> Result<FileHandle>
```

### 约束

1. 大文件异步处理（>1000 行导入走后台任务）
2. 导入前必须校验（格式/必填/唯一性/业务规则）
3. 导入结果报告：成功行数/失败行数/失败原因
4. 导出支持流式（大数据量不 OOM）
5. PDF 模板使用 HTML 渲染（方便自定义）
6. 多货主隔离（导出只含当前货主数据）

### 波次

Wave 2（基础 Excel 导入导出）→ Wave 4（PDF 台账 + 复杂报表）

---

## H8 ERP 防腐层（Anti-Corruption Layer）

### 职责

统一管理与外部 ERP 系统的所有交互，隔离 ERP 的数据格式和协议差异，使业务模块不直接依赖 ERP 接口细节。

### 能力分层与通用性边界

| 层级 | 职责 | 通用范围 |
|---|---|---|
| H-INT 契约 | 统一弹性、字段规整、凭证、审计、幂等和契约测试要求 | 所有外部系统；当前不提供共享运行时 |
| H8 ERP 防腐层 | ERP 连接配置、运行路由、主备切换和 ERP↔WMS 语义转换 | 多 ERP、多货主、多仓、方向和消息类型 |
| 通道适配器 | REST/接口表连接、协议、外部 DTO 的收发 | 单一通道或 ERP 协议，不包含 WMS 业务规则 |

业务模块只调用 WMS 业务 API 或 H8 端口；ERP DTO、接口表行和认证/连接细节必须止于
H8。H8 是 ADR-0030 的首个参考实现，但不得承载冷链、TMS、快递或企业微信语义；这些
对接只复用 H-INT 契约。共享运行时仍按 ADR-0030 第二段的生产证据条件延后。

### 双通道（2026-07 确认）

| 通道 | 适用 | 机制 |
|------|------|------|
| **A. REST 实时推送 + 回调** | 具备接口开发能力的 ERP | ERP 调 WMS OpenAPI（如 `inbound:push` 创建 ASN）；WMS 回调 ERP URL（clarifications #23） |
| **B. 接口表 + H8 Worker** | **不具备接口开发能力**的 ERP | 在 **ERP 库或约定接口库**落地接口表；实施/DBA/SQL 作业写入；**独立进程 `scripts/h8_erp_interface_sync`** 连接接口库，认领 `pending` 行并调用 WMS API，回写 `success/failed` |

两条通道最终都进入同一 WMS 业务 API（M2/M4/M1…），禁止业务模块直连 ERP 库。

首个真实对接为遵循 H8 契约的自研 ERP：REST 为主通道，独立接口库为备用通道，
US-H8-002 的 5 类入站和 7 类出站均保持双通道能力。交付按入库、出库、主数据/档案补录、
库存/财务四个闭环推进，每个闭环在一个货主、一个仓库取得真实 dev/staging S4 证据后启用。

#### 接口表通道（通道 B）本地联调

| 项 | 说明 |
|----|------|
| Compose | `deploy/docker-compose.h8-erp-if.yml`（MSSQL 模拟 ERP 接口库） |
| 入站表 | `if_in_asn` / `if_in_outbound_order` / `if_in_product_master` / `if_in_return_order` / `if_in_product_change` |
| 出站表 | `if_out_message`（通道 B）；通道 A 为 HTTP 回调（连接 `api_base_url` + path） |
| Worker | `sync_worker.py`：`--direction`；出站逐消息按货主、仓库、方向、消息类型调用 `route-resolve`，连接 `channel_mode` 决定 REST / 接口表 / 主备，禁止命令行覆盖生产路由 |
| 出站源 | putaway / inventory_status / stock_adjustment / **archive_revision** / **reconciliation** / **shipment_confirm** / **inventory_snapshot** outbox |
| 通道 A Mock | `channel_a_callback_mock.py`；确认工具 `ack_if_out.py` |
| Runbook | `docs/runbooks/h8-erp-interface-table-sync.md` |

接口表控制列继续使用 `sync_status`（入站 pending/processing/success/failed/dead；出站另含 acked）、`schema_version`、`retry_count`、`last_error`、`idempotency_key`、`wms_resource_id`，不得与 WMS 消息状态混用。WMS 消息技术接收后需要业务结果时进入 `awaiting_receipt`；业务成功进入 `acked`，明确拒绝或原幂等键重试耗尽进入 `dead`。档案补录 outbox 另有 `max_attempts`/`deadline_at`（5 次 / 5 分钟 / 24h）。

新增 ERP 单据类型 = 新接口表（或统一 staging + type 列）+ H8 映射/handler；ERP DTO 不进入 M2/M3/M4 域模型。只有 WMS 业务语义本身变化时，才修改对应业务 domain。

**本地闭环状态（2026-07 补全）**：通道 B 入站五类 + 出站七源；通道 A 出站 HTTP mock 联调；业务模块自动入队档案/对账/发货 outbox 与产线 S4 仍待接线/联调。完整消息目录、canonical 转换和业务接线按 [H8 用户故事](../domain/user-stories-h8-erp-integration.md) 中的 US-H8-002 验收。

#### ERP 连接配置（US-H8-001）

ERP 连接通过管理端「基础能力 / H8 集成中心 / H8 ERP 连接」独立菜单维护（`view_id=h8-erp-connectors`），持久化在 H8 专用表；权限使用 `h8.erp_connector.read/write`（不复用 M1 配置中心权限）。一个货主可以配置多个连接；每个连接可覆盖全部仓库或显式仓库白名单，并以“货主 + 仓库 + 方向 + 消息类型”形成唯一运行路由。全部仓库范围与任一显式范围重叠，禁止同时启用。

| 项 | 技术约束 |
|---|---|
| 通道 | `rest`、`interface_table`、`rest_primary_table_fallback`；双通道固定 REST 主用、接口表备用，不双写 |
| 入站认证 | REST 使用 H1 `X-WMS-API-Key`，scope 按消息类型最小授权 |
| 出站认证 | REST 使用独立 Bearer secret alias；接口表使用独立最小权限数据库账号和密码 secret alias |
| Secrets 解析 | 连接测试按 ADR-0013：`WMS_H8_SECRET_ALIASES`/`WMS_SECRETS_MAP`（local/e2e）→ `WMS_VAULT_ADDR`+`WMS_VAULT_TOKEN`（KV v2）；`WMS_SECRETS_REQUIRE_RESOLVE=1` 时强制可解析 |
| 配置状态 | `testing` → `active` ↔ `disabled`；有效连接关键配置变化后回到 `testing`；运行健康单独记录，不反写配置状态 |
| 启用门槛 | 当前配置版本通过 secret、TLS/认证、接口表结构/权限和路由重叠测试 |
| 变更 | 有效连接的端点、路由或 secret alias 变化后回到 `testing`；旧测试结果失效 |
| 停用 | 暂停在途消息，保留连接、通道阶段和 Idempotency-Key；重新启用后续传 |
| 删除 | 仅从未启用且无消息/业务引用时允许物理删除；H2 审计独立保留 |
| 权限 | `h8.erp_connector.read` 供系统管理员/仓库主管只读，`h8.erp_connector.write` 只供系统管理员执行配置动作 |
| 审计 | 配置、测试、启停、删除、自动主备切换全部写 H2 append-only 审计，禁止记录明文凭据 |

REST 按 ADR-0018 重试和熔断，满足降级条件后把同一消息及原 Idempotency-Key 转入接口表；半开探测恢复后回到 REST。连接测试只做健康、认证、权限和结构探测，不写真实业务单据。完整字段、状态动作、权限和证据标准见 [US-H8-001](../domain/user-stories-h8-erp-integration.md)。

消息日志、监控、死信、并发租约、人工重放、分区和保留边界见 [H8 用户故事](../domain/user-stories-h8-erp-integration.md) 中的 US-H8-003；本地 Worker 日志或 H2 审计均不能替代该故事的运行记录与故障恢复证据。

Worker 运行治理复用「H8 ERP 消息」页：消息指标之外登记实例心跳，并按连接 + 方向保存
独立暂停控制。暂停允许可选到期时间，在途处理完成后停止新认领；不改变连接配置状态，也
不允许 WMS 直接启动、停止或重启操作系统进程。完整报文默认不保存；按连接启用时加密
保留 7 天、最长 30 天，到期仅删除密文，H2 审计和消息处理结果继续保留。

### 消费方

| 交互方向 | 场景 | 模块 |
|---------|------|------|
| ERP → WMS | 推送 ASN（入库预报） | M2 |
| ERP → WMS | 推送出库订单 | M4 |
| ERP → WMS | 推送退货申请 | M4 |
| ERP → WMS | 推送商品主数据变更（含档案补录响应） | M1 |
| WMS → ERP | 入库完成反馈 | M2 |
| WMS → ERP | 出库发货反馈 | M4 |
| WMS → ERP | 库存快照同步 | M3 |
| WMS → ERP | 对账差异反馈 | M-RC |
| WMS → ERP | 报损报溢反馈 | M-SA |
| WMS → ERP | **档案补录推送**（PDA 验收触发，含商品编码/字段名/新值/拍照证据 URL/ASN 号） | M-QL / M2 |

### H8 防腐层契约

下列 `ErpAdapter` 是 ERP 专用逻辑端口示例，不是 H-INT 通用运行时。实现时可由 ERP
报文映射与 REST/接口表通道适配组合完成；业务模块不得实现该端口或依赖外部 DTO。

```rust
// ERP 适配器 trait（每种 ERP 实现一个）
// 通道 A：parse 来自 HTTP body；通道 B：parse 来自接口表行
trait ErpAdapter {
    // 接收
    fn parse_asn(raw: &RawMessage) -> Result<AsnCommand>;
    fn parse_outbound_order(raw: &RawMessage) -> Result<OutboundOrderCommand>;
    fn parse_product_master_change(raw: &RawMessage) -> Result<ProductChangeEvent>;  // 主数据变更回写

    // 发送
    fn send_inbound_complete(event: &InboundCompleteEvent) -> Result<()>;
    fn send_shipment_confirm(event: &ShipmentConfirmEvent) -> Result<()>;
    fn send_inventory_snapshot(snapshot: &InventorySnapshot) -> Result<()>;
    fn send_archive_revision(revision: &ArchiveRevisionRequest) -> Result<()>;  // 档案补录推送
}

// 档案补录数据结构
struct ArchiveRevisionRequest {
    liaison_id: Uuid,           // M-QL 联系单 ID
    asn_id: Uuid,                // 触发的 ASN
    receipt_record_id: Uuid,     // 验收记录 ID（用于解除阻塞）
    product_code: String,        // 商品编码
    field_name: String,          // 待补录字段名
    current_value: Option<Value>,// 档案当前值
    new_value: Value,            // 收货员录入新值
    photo_urls: Vec<String>,     // 实物拍照（≥1 ≤5）
    operator_id: UserId,         // 收货员
    submitted_at: DateTime<Utc>, // 时间戳
}

// 消息通道（REST API）
// ERP → WMS: POST /api/erp/asn (ERP 推送)
// ERP → WMS: POST /api/erp/product-master-change (ERP 主数据变更回写)
// WMS → ERP: POST {erp_callback_url}/inbound-complete (WMS 回调)
// WMS → ERP: POST {erp_callback_url}/archive-revision (档案补录推送)

// 重试机制
RetryPolicy::new()
    .max_retries(3)
    .backoff(ExponentialBackoff::new(Duration::from_secs(5)))
    .on_failure(|err| notify_admin(err))

// 档案补录专用重试（业务要求：阻塞验收 → 必须更高可靠性）
ArchiveRevisionRetryPolicy::new()
    .max_retries(5)
    .interval(Duration::from_secs(300))  // 5 分钟间隔
    .timeout_total(Duration::from_hours(24))  // 24h 总超时 → 告警
    .on_failure(|err| {
        mark_liaison_status("同步失败");
        notify_supervisor(err);
    })
```

### 约束

1. **协议**：REST API（JSON），实时推送+回调
2. **幂等**：所有接口支持幂等重试（通过 Idempotency-Key）
3. **重试**：发送失败自动重试（3 次，指数退避；档案补录 5 次/5 分钟间隔/24h 总超时）
4. **死信**：重试耗尽后进入死信队列，人工处理
5. **字段映射**：ERP 字段 → WMS 字段的映射可配置（不同 ERP 不同映射）
6. **多 ERP 支持**：通过货主级连接配置和适配器模式支持对接不同 ERP（SAP/用友/金蝶/自研）；路由唯一性按 US-H8-001 执行
7. **监控**：接口调用成功率/延迟/失败记录可查
8. **降级**：ERP 不可用时 WMS 业务不阻塞，消息暂存后补发；**档案补录例外**：阻塞当前 ASN 验收（业务要求），但不阻塞其他 ASN
9. **档案补录闭环**：推送 → ERP 处理 → ERP 通过商品主数据变更接口回写 → WMS 解除 ASN"档案补录中"状态（详见 M-QL-004 6 步闭环）

### 波次

Wave 1（接口定义 + Mock）→ Wave 2（真实对接第一个 ERP）

---

## H9 打印模板、打印组套与 Print Agent

### 职责分层

| 层 | 职责 | 禁止 |
|---|---|---|
| 模板层 | OpenAPI 字段库、模板设计和不可变发布版本 | 保存客户归集规则或物理设备 |
| 组套层 | 归集、截单、单据分类、模板/份数/顺序、就绪和失败策略 | 调用 Windows 驱动 |
| 渲染层 | 服务端按分类生成 PDF、哈希并写入 H-FILE | 选择物理打印机 |
| 任务层 | 队列、严格顺序、重打、暂停、改派、主备 Agent 和审计 | 把结果不明当失败自动重打 |
| Agent 层 | 下载冻结清单/PDF、校验哈希、调用本地打印后台、持久日志与重连对账 | 解释业务规则或渲染模板 |
| 设备层 | 仓库打印机、纸盒、纸张能力、测试结果和设备租约 | 跨物理打印站点引用 |

业务架构基线见 [ADR-0039](../adr/0039-print-suite-and-agent.md)，业务细化与局部取代见
[ADR-0041](../adr/0041-print-orchestration-refinement.md)，机器身份和协议见
[ADR-0040](../adr/0040-print-agent-machine-protocol.md)。

### 服务端组件

```text
HTTP handler
  -> print orchestration service
     -> print domain（归集、组套、状态与顺序）
     -> repository（PostgreSQL / H2 审计）
     -> M-CG port（事务内生成随货同行单号）
     -> H6 port（不可变状态定义与迁移校验）
     -> H-FILE port
     -> render worker queue
     -> Agent long-poll queue
```

- Render Worker 只生成 `rendered` 分类 PDF，不直连仓库打印机；`external_file` 分类由 H9
  校验并引用 H-FILE 中的权威文件。
- 打印分类使用 M1 `print_document_category`，以 `source_mode` 区分服务端渲染和外部权威
  PDF；`print_template_type` 只描述需要字段库的模板类型。随货同行单号以受控限定名
  `print_document_category:delivery_note` 调用 M-CG，不伪装成 M2/M4 `document_type`。
- 任务服务保存组套、模板、规则、输出映射和 Agent 分配快照。
- Web 写操作使用 H1 权限，机器写操作使用 H9 `MachineAuthContext`；Web 与机器业务/安全
  写操作进入 H2，所有写请求都有 L11 重放保护。高频遥测不逐次写 H2，且按下文使用序号
  去重而不累积通用幂等行。并发截单、任务领取和设备租约使用数据库约束或锁。
- 运行指标至少覆盖归集等待、渲染失败、队列时长、打印失败、结果不明、Agent 心跳、
  设备租约、对账冲突和磁盘阈值。

### 状态持久化

| H6 编码 | 实体 | 初始态 | 终态 | H9 持久化状态 |
|---|---|---|---|---|
| `h9_print_suite` | 组套实例 | `waiting_documents` | `completed`、`failed`、`cancelled` | `waiting_documents`、`queued`、`preparing`、`running`、`paused`、`awaiting_reconciliation`、`awaiting_manual_confirmation`、`completed`、`failed`、`cancelled` |
| `h9_print_item` | 打印项 | `pending` | `succeeded`、`skipped`、`cancelled` | `pending`、`printing`、`succeeded`、`failed`、`result_unknown`、`skipped`、`cancelled` |
| `h9_print_attempt` | 打印尝试 | `created` | `succeeded`、`failed` | `created`、`submitted`、`succeeded`、`failed`、`result_unknown` |
| `h9_device_lease` | 设备租约 | `active` | `released` | `active`、`released` |

- 上述状态图在 H6 注册为不可变定义；H9 在同一事务内完成迁移校验、业务状态写入、事件和
  H2 审计。
- 打印项严格串行。失败项只有在非必需且冻结策略允许时才能跳过；`result_unknown` 必须进入
  对账或人工确认，禁止自动重打、改派、跳过或释放租约。
- 每次重打创建新打印尝试；既有尝试不得复用、删除或回退，状态只按 H6 合法前进。完整迁移以
  [H9 用户故事](../domain/user-stories-h9-print-orchestration.md)“状态机与合法迁移”章节为准。

### Print Agent

首期平台为 Windows，客户端使用 Tauri 2 + React/Rust：

- React 仅显示本地状态和操作入口。
- Rust 从 Windows Credential Manager 读取 H9 机器凭据，执行 HTTP 长轮询、下载与校验
  PDF、调用 Windows 打印后台、维护缓存和持久日志；React 和日志不得读取明文凭据。
- Agent 是登录自启动托盘程序，不安装 Windows Service；进程退出即离线。
- 一次只运行一个打印组套实例；开始前完整下载清单、PDF 和哈希。
- 断网后只完成当前实例，重连后先对账再接单。
- H9 使用物理打印站点作为执行资源边界；一个站点显式映射多个
  `owner_id + warehouse_id`，Agent、打印机和设备租约归属站点，不改变 M1 仓库的单货主边界。

### 网络与身份

- Agent 主动连接 WMS，WMS 不向仓库终端发起入站连接。
- 每个 Agent 使用独立 H9 机器凭据并绑定 `agent_id + print_site_id`；H9 复用 H1 的密钥
  哈希、过期、轮换、吊销、失败锁定和审计规则，但使用独立凭据记录与
  `MachineAuthContext`，不放宽 H1 `owner_id` 约束，也不构造普通 owner-scoped
  `AuthContext`。
- 管理端以密码学安全随机数生成器创建至少 128 bit 熵的短期单次注册码，并绑定 Agent、站点、
  精确源 IP 和有效期。激活是唯一预凭据普通 HTTP 例外，只校验注册码与 socket peer IP；
  失败按请求 Agent 标识/IP 限流并锁定告警。能解析到 Agent/站点时按映射 owner 写 H2；
  无法归属的探测只写脱敏安全日志和指标，不伪造 owner，也不记录注册码或秘密。明文机器
  密钥只返回一次，响应丢失时吊销并重新注册。
- 首版凭据轮换先暂停 Agent 并确认没有未决任务，吊销旧凭据后复用新注册码重新激活；不建设
  在线双凭据切换协议。新秘密写入 Windows Credential Manager 失败时保持暂停并重新注册。
- Agent 只按 `agent_id` 领取已分配任务，不得传入或选择 `owner_id`；服务端校验任务的
  `owner_id + warehouse_id` 属于本站显式映射后，才允许下载清单和 PDF。
- 每次分配递增 `assignment_epoch`，设备租约使用不可复用 `lease_token`；预载、启动打印项
  和结果上报必须匹配当前二者。迟到代次不得推进正常状态，结果转入对账并告警。
- suspected/offline Agent 的运行中组套通过 H6 `agent_connection_lost` 进入等待对账，即使
  断联发生在首项提交前或两项之间也不得故障转移。安全转移要求旧 Agent 在线并持久化确认
  停止/交接；租约释放始终先满足无打印中、结果不明或待对账的硬守卫，再要求冻结策略允许
  `safe_auto`，或授权用户提供原因并二次确认；服务端最后在同一事务释放旧占用/租约、递增
  代次并领取同站点兼容备用 Agent 与新租约。
- 站点至少存在一条有效货主仓映射后才允许激活。Web 端站点/Agent 读写和单 Agent pilot
  动作由 H9 service 通过 H1 port 对映射前后 owner 并集逐一鉴权。pilot 提升 stable 和
  全局最低版本变更还要求 `h9.agent_version.global.write`，并校验全部未删除站点映射的
  owner 并集；任一缺少权限即整体 403，owner 并集为空时返回 409 且不得修改全局版本，
  只写脱敏安全日志/指标，不伪造 owner 写 H2。
- 机器协议使用由 WMS 进程直接终止 HTTP 的独立端口专用局域网 listener，只挂载 H9 机器
  白名单并拒绝其他路由。Agent 必须直连且禁止经过会改写源地址的 L7 代理、网关或 NAT。
  机器路由同时校验机器凭据和 listener 读取的 socket peer 精确源 IP，不读取
  `X-Forwarded-For`、`X-Real-IP` 或客户端自报地址，也不复用 H1 owner-scoped 中间件。
- 机器端点白名单只含心跳/运行/磁盘/设备状态、长轮询/任务领取/冻结清单、获授权 PDF
  下载、打印结果/重连对账和 `/agent-releases`。Web、H-FILE 通用上传/下载和其他模块
  仍按 ADR-0031 使用 HTTPS。
- Agent 下载 PDF 必须走 H9 专用机器接口；服务端校验任务、Agent、站点和货主仓映射后，
  以 H-FILE 稳定文件 ID 流式返回内容。不得把 H-FILE 通用下载地址降级为 HTTP，也不得向
  Agent 暴露可绕过 H9 鉴权的长期文件 URL。
- 所有机器写请求都携带 `Idempotency-Key`，客户端重试必须复用原键。非遥测命令还携带
  端点对应资源标识和请求哈希；打印结果绑定任务和尝试。同键不同载荷拒绝，已经进入后续
  状态的尝试不得再次触发物理打印。
- 非遥测命令写入 H9 自有机器幂等记录，作用域由 `activation_code_id` 或 `credential_id`、
  `method`、`resource_id`、`Idempotency-Key` 和 `request_hash` 组成，不复用 owner 必填的
  共享业务幂等表。心跳、磁盘和设备读数重试仍复用原 `Idempotency-Key`，但不落通用幂等行，
  服务端改用 `agent_id + boot_id + sequence` 单调更新，避免积累无界幂等行。
- 任务/尝试动作按任务快照 owner 写 H2；站点、Agent、凭据与安全动作按映射前后 owner 并集
  分别写 H2，全局版本动作按全部未删除站点映射 owner 分别写 H2；同一动作的事件复用 H2 既有
  `request_id`。高频遥测不逐次审计，只有运行状态跃迁、阈值告警和安全事件进入 H2。
- 机器事件写既有 `AuditActor` 时使用 `actor_id = agent_id`、事件时冻结的 Agent 编码/名称
  和 `jti = h9-machine:<credential_id>`；可解析的预凭据激活使用
  `jti = h9-activation:<activation_code_id>`。Web 动作仍使用用户 `AuthContext`。
- 跨网段、公网、无线访客网或其他不可信网络必须启用 HTTPS，本例外不得继承。

### 默认运行阈值

| 参数 | 默认值 | 覆盖范围 |
|---|---:|---|
| 心跳间隔 | 10 秒 | 全局，可按 Agent 覆盖 |
| 疑似离线 | 30 秒 | 全局，可按 Agent 覆盖 |
| 离线 | 60 秒 | 全局，可按 Agent 覆盖 |
| 恢复稳定观察 | 60 秒 | 全局，可按 Agent 覆盖 |
| 磁盘告警 | 剩余 20% | 全局，可按 Agent 覆盖 |
| 停止接单 | 剩余 10% | 全局，可按 Agent 覆盖 |
| 设备租约释放 | 仅人工 | 全局，可按打印机覆盖 |

预下载还必须满足“当前组套大小 + 安全余量”；固定百分比不能绕过单个大组套容量校验。

### 客户端更新

- 初始安装包由已登录且具备 Agent 管理权限的 H1 用户从常规 HTTPS Web 端点下载当前 stable
  完整包，不复用普通 HTTP `/agent-releases`，也不能选择 pilot；管理端同时发布当前、
  推荐和最低版本及下载地址。
- `VelopackApp::build().set_auto_apply_on_startup(false).run()` 必须作为主程序最先执行的启动
  钩子；Velopack 的默认启动自动应用必须关闭。钩子返回后，Agent 先扫描本地缓存并完成
  恢复/对账，确认没有运行中、结果不明或等待对账任务，才显式检查、下载和应用更新；已经
  下载的待应用包也不得在恢复检查前替换版本。该顺序以
  [VelopackApp Rust API](https://docs.rs/velopack/latest/velopack/struct.VelopackApp.html)
  为实现约束。
- 使用 Velopack 增量包；增量或 SHA-256 校验失败时只再尝试一次完整包，仍失败则保留当前
  版本。版本只允许向前，错误版本以更高版本重新发布。
- WMS 固定 `/agent-releases` 提供当前 pilot/stable、相邻增量、完整包、清单和 SHA-256；
  下载复用 H9 机器凭据与 socket peer 精确源 IP 白名单，不允许匿名访问。
- Velopack Rust 内置 `HttpSource` 当前
  [只接收基础 URL](https://docs.rs/velopack/latest/velopack/sources/struct.HttpSource.html)，
  不能直接承载本项目的机器凭据。Agent 必须实现最小 `UpdateSource` 适配器，复用同一
  受控 HTTP 客户端读取发布清单和包并注入 Agent 鉴权；不得为了迁就内置静态源放开匿名
  下载。实现时固定并复核 Velopack 版本。
- 发布脚本/CI 校验临时文件后原子发布不可变包；WMS 进程只读。pilot 提升 stable 只修改
  数据库通道指向，复用同一包和 SHA-256，管理端不能上传可执行文件。提升 stable 和变更
  全局最低版本前，H9 必须校验专用平台权限 `h9.agent_version.global.write`，并通过 H1
  port 校验操作者对全部未删除站点映射 owner 都有 Agent 管理权限；任一鉴权失败整体 403，
  owner 并集为空时返回 409。成功后按该 owner 并集分别写 H2、复用同一 `request_id`。
- 每次启动默认 stable；stable 可直接选择更新。pilot 必须由在线 H1 用户鉴权端点签发绑定
  `agent_id + startup_id + target_version + SHA-256` 的短期服务端授权；机器凭据不能创建
  该授权，用户 token 和密码只驻留内存。pilot 选择只对本次启动有效，不保存通道。
- 最低版本由具备 Agent 管理权限的用户独立维护，并在影响预览后二次确认；低于最低版本的
  Agent 完成恢复/对账后停止领取新任务。在线目录只保留当前 pilot/stable 和相邻增量，
  不建设更新并发调度器。
- 本地日志记录完整过程；H2 只记录通道选择、接受、成功/失败、stable 提升和最低版本变更，
  并包含通道、原版本、目标版本、SHA-256 和结果。
- 正式环境明确接受普通 HTTP + SHA-256 且没有发布者验真。SHA-256 不是来源证明；信任边界
  扩大或发生包替换事件时必须重新评估 HTTPS 或代码签名。
- Windows 无签名程序可能触发 SmartScreen 或杀毒软件警告；真实客户端验收必须记录安装和
  更新是否被拦截以及受控部署方式。失败时保留旧版本并告警，禁止客户端自动关闭或绕过系统
  防护。该运行风险见
  [Velopack Code Signing](https://docs.velopack.io/packaging/signing)。
- 首个正式版本前直接更新/重装测试 Agent；正式发布后的破坏性协议变更按 ADR-0016
  两阶段升级。

### 验收边界

- 单元/API/PostgreSQL/E2E 可验证规则、权限、状态和任务编排。
- Windows Agent、打印后台、打印机、纸盒、断网恢复和实物产物属于 S4，必须使用真实设备
  和人工核对证据。
- 浏览器打印、mock 设备、PDF 截图或“任务已提交”不能替代 S4 证据。

---

## H10 数据库备份与恢复

### 职责

保障 WMS 数据库（PostgreSQL）的数据安全：日常自动备份、异常恢复、归档保留、恢复演练。基础设施层职责，不在用户故事中体现，但与 H2 审计追踪共同构成 GSP 数据完整性保障的两根支柱。

### 与 H2 审计的分工

| 维度 | H2 审计追踪 | H10 数据库备份 |
|------|-----------|------------|
| 关注点 | 业务事件不可篡改记录 | 全库数据可恢复 |
| 粒度 | 单条操作 | 全库快照 + 增量 |
| 保留 | ≥ 5 年（GSP）| ≥ 5 年（GSP）|
| 触发 | 每个写操作（实时）| 定时（夜间）+ 事件（重大变更前）|
| 用途 | 追溯/审计/合规检查 | 灾难恢复/数据回滚 |

### 备份策略

#### 全量备份（Full Backup）

| 项 | 默认值 | 可配置 |
|----|-------|------|
| 频率 | 每日 1 次（凌晨 3:00）| 是（业务低峰期）|
| 工具 | `pg_basebackup` | — |
| 存储位置 | 异地（不与主库同机房）| 是 |
| 加密 | AES-256 | 是 |
| 压缩 | gzip 级别 6 | 是 |
| 保留策略 | 30 天滚动 + 每月 1 日全量永久保留 | 是 |
| 完整性校验 | 备份后 sha256 校验 | 强制 |

#### 增量备份（WAL 归档）

| 项 | 默认值 | 可配置 |
|----|-------|------|
| 模式 | PostgreSQL WAL（Write-Ahead Log）连续归档 | — |
| 触发 | WAL 段满（默认 16MB）即归档 | — |
| 归档间隔 | 最大 5 分钟（即使 WAL 未满）| 是 |
| 存储 | 同全量异地 | — |
| 保留策略 | 30 天，与最近一次全量备份一起保留 | — |
| RPO（数据丢失窗口）| ≤ 5 分钟 | — |

#### 重大变更前手动备份

| 触发场景 | 行为 |
|---------|------|
| 数据库 schema 迁移 | 迁移脚本执行前自动 `pg_dump` 当前库 |
| 大批量数据导入（M-CG 编码批量重置等）| 手动触发全量备份 |
| 系统升级 | 升级前完整备份 |

### 恢复策略

#### 恢复场景与目标

| 场景 | RTO（恢复时间目标）| RPO | 操作 |
|------|------------------|-----|------|
| 单表损坏 | ≤ 30 分钟 | 上次全备时点 | 从全量恢复单表（pg_restore -t）|
| 全库损坏 | ≤ 2 小时 | ≤ 5 分钟 | 全量 + WAL 重放至最新 |
| 误删数据（DELETE/UPDATE 误操作）| ≤ 1 小时 | 误操作前 | PITR（Point-in-time Recovery）|
| 机房故障 | ≤ 4 小时 | ≤ 5 分钟 | 切换到异地热备 |

#### 恢复操作权限

| 操作 | 权限 |
|------|------|
| 触发恢复 | 仅系统管理员 + 仓库主管双人审批（H4 企微） |
| 选择恢复时点（PITR）| 系统管理员 |
| 验证恢复完整性 | DBA 或自动校验脚本 |
| 恢复后审计 | 自动记录到 H2 审计表（actor=`system-restore`，附触发人 user_id） |

### 备份监控与告警

| 监控项 | 告警阈值 | 告警渠道 |
|-------|---------|---------|
| 全量备份失败 | 1 次失败立即告警 | H4 企微 + 邮件 + 短信 |
| WAL 归档延迟 | > 10 分钟 | H4 企微 |
| 备份文件大小异常 | 较前一日 ±50% | H4 企微 |
| 异地存储连接失败 | 连续 3 次失败 | H4 企微 + 短信 |
| 完整性校验失败 | 立即 | H4 企微 + 邮件 + 短信 + 仓库主管 |
| 保留期到期但备份缺失 | 立即 | H4 企微 + 邮件 |

### 恢复演练

| 项 | 频率 | 内容 |
|----|------|------|
| 单表恢复演练 | 每月 1 次 | 在测试环境恢复任一非关键表 |
| 全库恢复演练 | 每季度 1 次 | 测试环境恢复 + 业务回归测试 |
| 切换演练（异地热备）| 每年 1 次 | 模拟机房故障切换 |
| 演练记录 | 每次演练写入 H2 审计 | 含 RTO/RPO 实测值 + 演练参与人 |

### 接口契约

```rust
// 备份操作 trait（可对接不同存储后端：S3 / OSS / 本地 NAS）
trait BackupStorage {
    fn upload(file: &Path, key: &str) -> Result<BackupMetadata>;
    fn download(key: &str, target: &Path) -> Result<()>;
    fn list(prefix: &str) -> Result<Vec<BackupMetadata>>;
    fn delete(key: &str) -> Result<()>;
    fn verify_checksum(key: &str, expected: &str) -> Result<bool>;
}

// 备份元数据
struct BackupMetadata {
    backup_id: Uuid,
    backup_type: BackupType,  // Full / WAL / Manual
    created_at: DateTime<Utc>,
    size_bytes: u64,
    checksum_sha256: String,
    encryption_key_id: String,
    retention_until: DateTime<Utc>,  // 保留至
    triggered_by: Option<UserId>,    // 手动触发时记录
    related_event: Option<String>,   // 如"schema-migration-v2.3"
}

// 恢复请求
struct RestoreRequest {
    backup_id: Uuid,
    restore_point: Option<DateTime<Utc>>,  // PITR 时点
    target_environment: Environment,        // production / staging / drill
    requested_by: UserId,
    approved_by: UserId,                    // 双人审批
    reason: String,
}
```

### 分级保留矩阵（v25 新增 — 满足 GSP 不同药品类型差异化保留期）

不同业务对象 / 药品类型对数据保留期有差异化的法规要求。备份保留策略按下表分级：

| 数据类型 | 法规依据 | 保留期 | 备份策略 |
|---------|---------|------|--------|
| 普通业务数据（库存流水 / 出入库单 / 审计追踪）| GSP 第 5 章 5.69 | ≥ 5 年 | 30 天滚动全备 + 每月 1 日永久保留至 5 年 |
| 麻精毒放药品台账 / 业务流水 | 《麻醉药品和精神药品管理条例》/《医疗用毒性药品管理办法》| ≥ 5 年 | 同上（与普通台账共用 5 年策略）|
| 放射性药品台账 / 业务流水 | 《放射性药品管理办法》| ≥ 30 年 | 独立保留分区：每年 1 日全备永久保留至 30 年 |
| 疫苗 / 血液制品台账 | 《疫苗管理法》/《血液制品管理条例》| ≥ 5 年（部分省地方法规可能要求 ≥ 10 年）| 默认 5 年；按地方法规可单独配置至 10 年 |
| 冷链温度记录 | GSP 冷链专项 | ≥ 5 年（外部冷链系统主管，WMS 缓存 1 年）| 见 docs/compliance/gsp-ch9-cold-chain.md 冷-9 |

**实现机制**：

1. 表级保留标签：每个业务表通过 `retention_class` 标签声明所属保留等级（普通 5 年 / 放射性 30 年 / 等）
2. 备份脚本按 `retention_class` 分类归档：
   - 5 年类：30 天滚动 + 每月 1 日永久保留至 5 年；过期物理删除
   - 30 年类：每年 1 日全备永久保留至 30 年；归档到冷存储（成本优化）
3. 恢复时按时间窗 + 保留类匹配（避免误恢复已过期的普通数据）
4. 监控指标：每个保留类的最早可恢复时点 + 即将过期的备份预警

**业务前置**（2026-05-17 业务方确认）：业务方确认放射性 / 血液制品 / 疫苗 3 类都将承运，但**本期（v25 设计 + Wave 1-3）不实施**，列入 Wave 4+ backlog。本节分级保留矩阵作为设计完整保留，实施时按业务方 + 财务 + 运维联合评估后启用对应分区。

**成本预估**（待财务评估，启用前必做）：

- 30 年保留对存储成本影响约 +6× / 30 年（线性累积，不含冷存储优惠）
- 加密 + 异地双拷贝 + 季度演练成本另估
- 决策路径：业务方启用承运放射性药品业务前 → 财务 + 运维联合评估 → ADR 决策启用 30 年分区 → 实施

### 约束

1. **加密强制**：备份文件**必须**加密（AES-256），密钥独立管理（不在备份介质中）
2. **异地存储**：全量备份**必须**存放在与主库不同的机房 / 云区域
3. **完整性校验**：备份后立即 sha256，恢复前再校验
4. **分级保留** (v25)：默认普通业务数据 ≥ 5 年；放射性药品业务 ≥ 30 年（按上表分级保留矩阵）
5. **恢复必须双人审批**：系统管理员 + 仓库主管，H4 企微审批留痕
6. **恢复操作写入审计**：actor、时点、原因、批准人均记录到 H2 审计
7. **演练强制**：每月单表 + 每季度全库 + 每年切换；演练失败需修复后再次演练
8. **不可在生产环境直接恢复未演练过的备份**：所有恢复路径需先在 staging 验证

### GSP 合规对应

详见 [docs/compliance/gsp-ch5-warehouse-management.md](../compliance/gsp-ch5-warehouse-management.md) 第 5 章数据完整性条款追溯。

### 波次

Wave 0（基础脚本）→ Wave 1（接入 PostgreSQL + WAL 归档配置）→ Wave 4（恢复演练流程）

---

## 模块依赖总览

=== "📊 流程图"

    ```mermaid
    flowchart LR
        subgraph BIZ["业务模块（用户故事）"]
            M1[M1 基础]
            M2[M2 入库]
            M3[M3 库存]
            M4[M4 出库]
            M6[M6 报表]
            MTE[M-TE 任务]
            MQL[M-QL 质量]
        end
        subgraph INFRA["技术基础设施（本文档）"]
            H6[H6 状态机引擎]
            H7[H7 导入导出]
            H8[H8 ERP 防腐层]
            H9[H9 打印编排]
            H10[H10 数据库备份]
        end
        subgraph HORIZ["已有横向模块（用户故事）"]
            H1[H1 权限]
            H2[H2 审计追踪]
            H3[H3 OpenAPI]
            H4[H4 企业微信]
            H5[H5 快递]
        end
        M2 --> H6
        M2 --> H8
        M2 --> H9
        M4 --> H6
        M4 --> H7
        M4 --> H8
        M4 --> H9
        M3 --> H7
        M3 --> H8
        M6 --> H7
        MTE --> H6
        MQL --> H6
        M1 --> H7
        classDef bizCls fill:#e3f2fd,stroke:#1976d2
        classDef infraCls fill:#fff3e0,stroke:#f57c00
        classDef horizCls fill:#f3e5f5,stroke:#7b1fa2
        class M1,M2,M3,M4,M6,MTE,MQL bizCls
        class H6,H7,H8,H9,H10 infraCls
        class H1,H2,H3,H4,H5 horizCls
    ```

=== "📝 源码"

    ```
    flowchart LR
        subgraph BIZ["业务模块（用户故事）"]
            M1[M1 基础]
            M2[M2 入库]
            M3[M3 库存]
            M4[M4 出库]
            M6[M6 报表]
            MTE[M-TE 任务]
            MQL[M-QL 质量]
        end
        subgraph INFRA["技术基础设施（本文档）"]
            H6[H6 状态机引擎]
            H7[H7 导入导出]
            H8[H8 ERP 防腐层]
            H9[H9 打印编排]
            H10[H10 数据库备份]
        end
        subgraph HORIZ["已有横向模块（用户故事）"]
            H1[H1 权限]
            H2[H2 审计追踪]
            H3[H3 OpenAPI]
            H4[H4 企业微信]
            H5[H5 快递]
        end
        M2 --> H6
        M2 --> H8
        M2 --> H9
        M4 --> H6
        M4 --> H7
        M4 --> H8
        M4 --> H9
        M3 --> H7
        M3 --> H8
        M6 --> H7
        MTE --> H6
        MQL --> H6
        M1 --> H7
    ```

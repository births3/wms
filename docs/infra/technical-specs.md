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

### 双通道（2026-07 确认）

| 通道 | 适用 | 机制 |
|------|------|------|
| **A. REST 实时推送 + 回调** | 具备接口开发能力的 ERP | ERP 调 WMS OpenAPI（如 `inbound:push` 创建 ASN）；WMS 回调 ERP URL（clarifications #23） |
| **B. 接口表 + H8 Worker** | **不具备接口开发能力**的 ERP | 在 **ERP 库或约定接口库**落地接口表；实施/DBA/SQL 作业写入；**独立进程 `scripts/h8_erp_interface_sync`** 连接接口库，认领 `pending` 行并调用 WMS API，回写 `success/failed` |

两条通道最终都进入同一 WMS 业务 API（M2/M4/M1…），禁止业务模块直连 ERP 库。

#### 接口表通道（通道 B）本地联调

| 项 | 说明 |
|----|------|
| Compose | `deploy/docker-compose.h8-erp-if.yml`（MSSQL 模拟 ERP 接口库） |
| 入站表 | `01_schema.sql`：`if_in_asn` / `if_in_outbound_order` / `if_in_product_master`；`03_if_out_and_return.sql`：`if_in_return_order`（销退） |
| 出站表 | `if_out_message`（统一 WMS→ERP：event_type + payload_json；同源 outbox 幂等） |
| Worker | `scripts/h8_erp_interface_sync/sync_worker.py`（`--direction in\|out\|both`） |
| 出站源 | WMS PG：`receiving_putaway_erp_feedback_outbox` / `inventory_status_erp_feedback_outbox` / `stock_adjustment_erp_feedback_outbox` → worker 投递 `if_out_message` 后标记 outbox `succeeded` |
| Runbook | `docs/runbooks/h8-erp-interface-table-sync.md` |

控制列约定：`sync_status`（入站 pending/processing/success/failed/dead；出站另含 acked）、`retry_count`、`last_error`、`idempotency_key`、`wms_resource_id`。

新增 ERP 单据类型 = 新接口表（或统一 staging + type 列）+ 新 handler，不改 M2/M3/M4 域模型。

**通道 B 本地闭环状态（2026-07）**：入站四类（商品/ASN/出库/销退）+ 出站统一消息表 + outbox 投递 worker 已交付；产线真实 ERP 实例、档案补录专用重试与对账差异表仍按模块 S4/联调补证。

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

### 接口契约

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
6. **多 ERP 支持**：通过适配器模式支持对接不同 ERP（SAP/用友/金蝶/自研）
7. **监控**：接口调用成功率/延迟/失败记录可查
8. **降级**：ERP 不可用时 WMS 业务不阻塞，消息暂存后补发；**档案补录例外**：阻塞当前 ASN 验收（业务要求），但不阻塞其他 ASN
9. **档案补录闭环**：推送 → ERP 处理 → ERP 通过商品主数据变更接口回写 → WMS 解除 ASN"档案补录中"状态（详见 M-QL-004 6 步闭环）

### 波次

Wave 1（接口定义 + Mock）→ Wave 2（真实对接第一个 ERP）

---

## H9 打印模板引擎

### 职责

统一管理所有打印场景的模板定义、数据填充、设备对接。

### 消费方

| 场景 | 打印内容 | 设备 |
|------|---------|------|
| 入库验收 | 验收单 | A4 打印机 |
| 容器标签 | LPN 码 + 内容物摘要 | 标签打印机 |
| 出库面单 | 快递面单 | 面单打印机 |
| 盲标签 | 顺序编号条码 | 标签打印机 |
| GSP 台账 | 法定台账 | A4 打印机 |
| 温控报告 | 温度曲线 + 二维码 | A4 打印机 |

### 接口契约

```rust
// 模板管理
Template::create("lpn_label")
    .format(TemplateFormat::ZPL)  // ZPL/HTML/PDF
    .content(template_content)
    .variables(vec!["lpn_code", "product_name", "qty", "batch"])
    .device_type(DeviceType::LabelPrinter)
    .save();

// 打印执行
PrintJob::new("lpn_label")
    .data(json!({"lpn_code": "LPN001", "product_name": "阿莫西林", ...}))
    .printer("PRINTER-01")  // 指定打印机或自动选择
    .copies(1)
    .execute() -> Result<PrintJobId>
```

### 约束

1. 模板格式：标签用 ZPL/TSPL；文档用 HTML→PDF
2. 系统内置常用模板（开箱即用）
3. 用户可自定义模板（HTML 编辑器）
4. 打印机管理：注册/状态监控/默认打印机
5. 打印队列：支持批量打印、重打
6. PDA 触发打印（PDA 扫码 → 服务端渲染 → 打印机输出）

### 波次

Wave 2（标签打印）→ Wave 4（PDF 台账 + 面单）

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
            H9[H9 打印模板]
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
            H9[H9 打印模板]
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

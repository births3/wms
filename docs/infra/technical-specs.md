# 基础设施模块：技术规格

> 本文档定义系统的横向技术基础设施模块。
> 这些模块**不直接面向用户**，而是被业务模块调用。
> 格式：技术规格（职责/接口/约束/消费方），不使用用户故事格式。

---

## H6 状态机引擎

### 职责

为所有有状态流转的业务实体提供统一的状态机定义、转换执行、事件发布能力。

### 消费方

| 业务模块 | 状态机实体 | 状态数 |
|---------|-----------|--------|
| M2 入库 | ASN | 7 + 4 异常态 |
| M4 出库 | 出库订单 | 7 |
| M4 退货 | 退货单 | 6 |
| M-TE 任务引擎 | 任务 | 5（待释放/待分配/已分配/执行中/已完成） |
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

### 消费方

| 交互方向 | 场景 | 模块 |
|---------|------|------|
| ERP → WMS | 推送 ASN（入库预报） | M2 |
| ERP → WMS | 推送出库订单 | M4 |
| ERP → WMS | 推送退货申请 | M4 |
| WMS → ERP | 入库完成反馈 | M2 |
| WMS → ERP | 出库发货反馈 | M4 |
| WMS → ERP | 库存快照同步 | M3 |
| WMS → ERP | 对账差异反馈 | M-RC |
| WMS → ERP | 报损报溢反馈 | M-SA |

### 接口契约

```rust
// ERP 适配器 trait（每种 ERP 实现一个）
trait ErpAdapter {
    // 接收
    fn parse_asn(raw: &RawMessage) -> Result<AsnCommand>;
    fn parse_outbound_order(raw: &RawMessage) -> Result<OutboundOrderCommand>;
    
    // 发送
    fn send_inbound_complete(event: &InboundCompleteEvent) -> Result<()>;
    fn send_shipment_confirm(event: &ShipmentConfirmEvent) -> Result<()>;
    fn send_inventory_snapshot(snapshot: &InventorySnapshot) -> Result<()>;
}

// 消息通道（REST API）
// ERP → WMS: POST /api/erp/asn (ERP 推送)
// WMS → ERP: POST {erp_callback_url}/inbound-complete (WMS 回调)

// 重试机制
RetryPolicy::new()
    .max_retries(3)
    .backoff(ExponentialBackoff::new(Duration::from_secs(5)))
    .on_failure(|err| notify_admin(err))
```

### 约束

1. **协议**：REST API（JSON），实时推送+回调
2. **幂等**：所有接口支持幂等重试（通过 Idempotency-Key）
3. **重试**：发送失败自动重试（3 次，指数退避）
4. **死信**：重试耗尽后进入死信队列，人工处理
5. **字段映射**：ERP 字段 → WMS 字段的映射可配置（不同 ERP 不同映射）
6. **多 ERP 支持**：通过适配器模式支持对接不同 ERP（SAP/用友/金蝶/自研）
7. **监控**：接口调用成功率/延迟/失败记录可查
8. **降级**：ERP 不可用时 WMS 业务不阻塞，消息暂存后补发

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

## 模块依赖总览

```
业务模块（用户故事）
    │
    ├── M2 入库 ──→ H6 状态机 + H8 ERP + H9 打印
    ├── M4 出库 ──→ H6 状态机 + H7 导出 + H8 ERP + H9 打印
    ├── M3 库存 ──→ H7 导出 + H8 ERP
    ├── M6 报表 ──→ H7 导出
    ├── M-TE 任务 ──→ H6 状态机
    ├── M-QL 质量 ──→ H6 状态机
    └── M1 基础 ──→ H7 导入导出
    
技术基础设施（本文档）
    │
    ├── H6 状态机引擎
    ├── H7 导入导出引擎
    ├── H8 ERP 防腐层
    └── H9 打印模板引擎
    
已有横向模块（用户故事格式）
    │
    ├── H1 权限（用户可配置角色/权限）
    ├── H2 审计追踪（自动中间件，无需用户操作）
    ├── H4 企业微信（用户配置通知/审批）
    └── H5 快递（用户配置快递商/规则）
```

# ADR-0034：Wave 3 业务表 PostgreSQL 持久化模型

- 状态：Accepted
- 决策日期：2026-06-03
- 决策人：项目主人
- 起草人：AI 助手
- 关联：ADR-0001 / ADR-0018 / ADR-0024 / ADR-0025 / ADR-0026 / ADR-0030 / Wave 3 M2/M3/M5/M9 用户故事

---

## 背景

Wave 3 第一批后端切片已经完成内存服务与 OpenAPI 契约：

- M2 收货闭环、验收、双人签字、上架规则。
- M3 库存批次、可用库存、库存质量状态机与审批源约束。
- M5 外部冷链设备、温控读数、温度超标事件接入。
- M9 计费账户、合同、规则基础模型。

当前代码仍使用 in-memory store。若继续推进 PostgreSQL repository，必须先冻结最小业务表结构，否则会在实现迁移时同时做数据模型取舍，违反“有风险必须确认”的协作约束。

本 ADR 只覆盖 Wave 3 第一批 repository 的表结构，不扩展到 Wave 3 后续故事。

### 已有约束

| 来源 | 约束 | 本 ADR 的处理 |
|------|------|---------------|
| ADR-0001 | 后端主存储为 PostgreSQL + SQLx | 所有业务表使用 PostgreSQL 原生类型，repository 用 SQLx |
| ADR-0018 | 写操作必须支持 Idempotency-Key | 新增共享 `idempotency_request` 表 |
| ADR-0024 | handler 通过 `AuthContext.owner_id` 做多货主隔离 | 所有业务表强制 `owner_id`，查询必须带 owner 过滤 |
| ADR-0025 | 审计写入 H2 `audit_event`，append-only | 业务表不承担审计不可篡改职责，mutation 成功后追加 audit_event |
| ADR-0026 | OpenAPI 是跨端契约单一事实源 | 表字段不能反向悄悄扩 API，新增 API 字段必须同步 OpenAPI |
| ADR-0030 | 外部对接必须走 H-INT 契约 | M5 外部推送保留 external id / source，不在业务表保存明文 secret |

---

## 候选方案

### A. 最小规范化业务表 + 共享幂等表（推荐）

为 Wave 3 第一批路径建立最小规范化表：M2/M3/M5/M9 各自保存稳定业务字段，扩展性弱的字段用少量 JSONB 或 nullable 字段承接。状态流转由领域服务校验，数据库负责 owner 隔离、唯一键、数量非负、外键和幂等去重。

优点：
- 能马上支撑 repository 和 L5/L11 测试。
- 不把未来 M2 打印、M3 盘点/移库、M9 自动出账提前建满。
- 保留数据库约束，避免全 JSONB 失去一致性保护。

缺点：
- 部分未来字段后续仍需要 migration。
- 状态枚举主要由领域服务约束，数据库 CHECK 只放不可争议的数值/唯一性约束。

### B. 业务文档型表（每个聚合一张 JSONB）

每个模块只建主表，业务 payload 存 JSONB。

优点：
- 初期 migration 少，字段变化成本低。
- 原型阶段改字段快。

缺点：
- 难以做数量一致、唯一键、状态查询和跨表 join。
- GSP/审计排查时结构弱，容易把语义错误推迟到运行期。
- SQLx 编译期校验价值下降。

### C. 一次性全量规范化到 Wave 3 全范围

一次性为 M2 全部故事、M3 养护/盘点/移库/预警、M9 自动计费/账单都建表。

优点：
- 长期模型更完整。
- 后续模块少做 migration。

缺点：
- 当前只实现第一批后端切片，全量建表会提前锁定尚未实现和未走查的业务取舍。
- 容易把 M-TE、M-QL、M-RC、H-INT、H-FILE 等后续边界混进本次 migration。

---

## 决策

采用方案 A：**最小规范化业务表 + 共享幂等表**。

### 已确认取舍

| # | 问题 | 决策 |
|---|------|------|
| 1 | M2 收货闭环记录基数 | 一单一次闭环，`receiving_order_receipts` 加 `unique(receiving_order_id)` |
| 2 | M2 上架与 M3 库存事务 | 同事务写 `receiving_putaways` + `inventory_batches` + `inventory_movements` |
| 3 | M9 规则生效期字段 | 现在补 `effective_from` / `effective_to` 到 API 和表 |
| 4 | GSP 资质有效期校验来源 | 冻结为 M1 本地资质档案 + M-VR 校验规则执行；ERP/API 作为供应商资质同步来源，不作为入库运行时资质有效期校验的唯一事实源 |
| 5 | M5 温控读数分区 | 普通表 + 索引，暂不按月分区 |

### 设计原则

1. 所有业务表必须有 `id uuid primary key`、`owner_id uuid not null`、`created_at timestamptz not null default now()`。
2. 可变聚合表增加 `updated_at timestamptz not null default now()` 与 `version bigint not null default 1`，repository 用 optimistic locking 或事务内行锁保护并发写。
3. 所有 owner-scoped 唯一键必须包含 `owner_id`。
4. 所有写 handler 必须先校验权限，再执行事务，成功后写 `audit_event`；失败业务校验不得追加审计。
5. 所有 HTTP 写操作通过 `idempotency_request` 去重；外部系统推送同时使用外部事件 ID 去重。
6. 业务状态字段先用 `text`，领域服务做状态机校验；数据库只对数量、外键、唯一键做硬约束。
7. 不在本 ADR 中新增 secret 表；M5 外部 API Key 按 ADR-0013 / ADR-0030 另行接入。

---

## 表结构草案

### 1. 共享幂等表

| 表 | 用途 | 关键字段 / 约束 |
|----|------|-----------------|
| `idempotency_request` | HTTP 写操作幂等去重 | `owner_id`, `idempotency_key`, `request_hash`, `method`, `path`, `status_code`, `response_body jsonb`, `resource_type`, `resource_id`, `expires_at`; `unique(owner_id, idempotency_key)` |

说明：
- `request_hash` 用于同一 key 携带不同 payload 时返回冲突。
- `response_body` 保存首次成功响应，重复请求返回首次结果。
- TTL 清理按 ADR-0018，清理任务不影响业务表。

### 2. M2 入库 workflow

| 表 | 用途 | 关键字段 / 约束 |
|----|------|-----------------|
| `receiving_orders` | ASN / 收货单主表 | `receipt_no`, `supplier_id`, `warehouse_id`, `external_ref`, `status`, `expected_arrival_at`; `unique(owner_id, receipt_no)` |
| `receiving_order_lines` | ASN 明细 | `receiving_order_id`, `line_no`, `product_id`, `product_code`, `expected_qty`, `batch_no`, `production_date`, `expiry_date`; `expected_qty > 0`; `unique(receiving_order_id, line_no)` |
| `receiving_order_receipts` | 收货闭环结果 | `receiving_order_id`, `actual_qty`, `shortage_qty`, `rejected_qty`, `arrival_temperature_celsius`, `exception_note`, `occurred_at`; 数量均 `>= 0` |
| `receiving_inspections` | 验收记录 | `receiving_order_id`, `batch_no`, `accepted_qty`, `rejected_qty`, `production_date`, `expiry_date`, `quality_status`, `trace_codes text[]`, `occurred_at`; 数量均 `>= 0` |
| `receiving_inspection_signatures` | 验收签字记录 | `receiving_order_id`, `dual_required`, `first_signer_id`, `second_signer_id`, `strategy_rule_id`, `signed_at`; `second_signer_id <> first_signer_id` when second exists |
| `receiving_putaways` | 上架记录 | `receiving_order_id`, `batch_no`, `product_code`, `qty`, `location_id`, `location_code`, `quality_status`, `occurred_at`; `qty > 0` |

确认：一单一次收货闭环，`receiving_order_receipts` 必须加 `unique(receiving_order_id)`。

### 3. M3 库存

| 表 | 用途 | 关键字段 / 约束 |
|----|------|-----------------|
| `inventory_batches` | 当前库存批次余额 | `product_code`, `batch_no`, `production_date`, `expiry_date`, `qty_on_hand`, `qty_locked`, `quality_status`, `location_id`, `location_code`, `recall_flag`; `qty_on_hand >= 0`, `qty_locked >= 0`, `qty_locked <= qty_on_hand` |
| `inventory_movements` | 库存数量流水 | `batch_id`, `movement_type`, `qty_delta`, `source_document_type`, `source_document_id`, `occurred_at` |
| `inventory_status_changes` | 库存质量状态变更 | `batch_id`, `from_status`, `to_status`, `reason`, `approval_source`, `approval_id`, `occurred_at`; `approval_source` / `approval_id` not null |

索引建议：
- `inventory_batches(owner_id, product_code, batch_no)`
- `inventory_batches(owner_id, location_id, quality_status)`
- `inventory_batches(owner_id, expiry_date)`
- `inventory_movements(owner_id, batch_id, occurred_at desc)`
- `inventory_status_changes(owner_id, batch_id, occurred_at desc)`

唯一性建议：
- `unique(owner_id, product_code, batch_no, location_id, quality_status)` 用于同批次同库位同质量状态聚合。

### 4. M5 外部冷链接入

| 表 | 用途 | 关键字段 / 约束 |
|----|------|-----------------|
| `cold_chain_devices` | 冷链设备台账 | `device_code`, `device_type`, `installed_at_location_code`, `calibration_due_at`, `status`; `unique(owner_id, device_code)` |
| `temperature_readings` | 外部系统推送温湿度读数缓存 | `device_code`, `temperature_celsius`, `humidity_percent`, `captured_at`, `external_report_url`, `out_of_range`, `source_system`, `external_reading_id`; `unique(owner_id, device_code, captured_at)` |
| `temperature_excursion_events` | 外部温度超标事件 | `external_event_id`, `device_code`, `location_code`, `started_at`, `ended_at`, `min_temperature_celsius`, `max_temperature_celsius`, `affected_batch_ids uuid[]`, `status`, `created_at`; `unique(owner_id, external_event_id)` |

说明：
- WMS 只接收外部系统判定后的 `out_of_range` / excursion event，不在表内保存阈值判定逻辑。
- 温控读数只作为近 1 年查询缓存，长期原始记录由外部冷链系统负责。
- `temperature_excursion_events.status` 初始为 `pending_disposition`，不自动隔离库存。

### 5. M9 计费账户 / 合同 / 规则

| 表 | 用途 | 关键字段 / 约束 |
|----|------|-----------------|
| `billing_accounts` | 计费账户 | `account_code`, `account_name`, `status`; `unique(owner_id, account_code)` |
| `billing_contracts` | 计费合同 | `account_id`, `contract_no`, `valid_from`, `valid_to`, `status`; `unique(owner_id, contract_no)`, `valid_to >= valid_from` |
| `billing_rules` | 计费规则 | `contract_id`, `charge_item`, `unit`, `unit_price_cents`, `billing_cycle`, `effective_from`, `effective_to`, `created_at`; `unit_price_cents >= 0`, `effective_to >= effective_from` |

确认：M9 用户故事要求“规则生效日期冲突”校验，因此本次同步扩展 `CreateBillingRuleRequest` 与 `BillingRule`，新增 `effective_from` / `effective_to`。

---

## Repository 事务边界

### M2 receive / inspect / sign

单 handler 单事务：

1. `SELECT ... FOR UPDATE` 锁定 `receiving_orders`。
2. 执行业务状态校验。
3. 写 workflow 子表。
4. 更新 `receiving_orders.status` / `updated_at` / `version`。
5. 写 `audit_event`。
6. 提交事务。

业务校验失败直接回滚，不写审计。

### M2 putaway + M3 inventory

推荐同一事务内完成：

1. 锁定 `receiving_orders`。
2. 写 `receiving_putaways`。
3. upsert `inventory_batches`。
4. 写 `inventory_movements`。
5. 更新 ASN 为 `completed` 或保持 `putaway`（部分上架场景）。
6. 写 `audit_event`。
7. 提交事务。

确认：M2 上架与 M3 库存必须同事务编排，不接受“上架记录已写、库存未增加”的一致性窗口。

### M3 status change

1. `SELECT ... FOR UPDATE` 锁定 `inventory_batches`。
2. 校验状态机与 `approval_source/approval_id`。
3. 写 `inventory_status_changes`。
4. 更新 `inventory_batches.quality_status`。
5. 写 `audit_event`。
6. 提交事务。

### M5 ingest

外部推送按 `external_event_id` 或 `(owner_id, device_code, captured_at)` 去重：

1. 校验设备存在且启用。
2. INSERT，冲突时返回既有记录。
3. 写 `audit_event`。

### M9 config

账户、合同、规则配置均单事务写入，成功后追加审计。M9 自动计费和账单表不在本 ADR 范围。

---

## 后果

### 正面

- Wave 3 repository 可以在确认后按表逐步落地，不再把 migration 设计夹在实现提交中。
- 幂等、审计、多货主隔离都有明确落点。
- M5 外部冷链边界清楚：WMS 接收与联动，不做采集和超标判定。
- M3 状态变更的审批源链路可通过表字段和测试强制。

### 负面

- 仍需后续 migration 支撑 M2 打印模板、M3 盘点/移库/养护、M9 自动计费/账单。
- 业务状态用 text，DB 不完全阻断非法枚举，必须依赖领域服务和 L4/L5 测试。
- M9 自动计费和账单表不在本 ADR 范围，后续进入 Wave 5 时需另补 migration。

### 风险

| 风险 | 影响 | 应对 |
|------|------|------|
| M2 收货闭环到底是一单一次还是可多次分批 | 唯一键不同，影响补货/短少处理 | 已确认一单一次并落 `unique(receiving_order_id)`；未来如改分批需新 ADR + migration |
| M2 putaway 与 M3 inventory 分开事务 | 上架记录和库存余额可能短期不一致 | 已确认同事务写 `receiving_putaways` + `inventory_batches` + `inventory_movements` |
| M9 规则生效期缺失 | 未来无法校验“规则生效日期冲突” | 已扩 API 与表字段，并有 PostgreSQL 冲突测试 |
| GSP 资质有效期来源与 ERP 边界混淆 | 可能把供应商资质有效期、经营范围、特殊药品经营资质混成同一校验来源 | 已冻结供应商资质有效期由 M1 本地资质档案 + M-VR 执行；ERP/API 只作为同步来源；经营范围/特殊药品经营资质仍按既有边界由 ERP 校验 |
| M5 外部 API Key 未接入 | 外部推送鉴权不完整 | 已按 ADR-0013 / ADR-0030 落 `X-WMS-API-Key` hash 配置，不在业务表保存 secret |

---

## 实施约束

按以下顺序实施：

1. 新增 migration：`backend/migrations/YYYYMMDDHHMM_wave3_core_tables.sql`。
2. 为 migration 增加真实 PostgreSQL 测试，覆盖唯一键、数量约束、owner 隔离、幂等去重。
3. 新增 SQLx repository，先覆盖 M2 putaway + M3 inventory 关键路径。
4. handler 从 in-memory state 切换到 PgPool/repository。
5. 补 L4/L5/L8/L11 测试：业务错误不写审计，成功写业务表 + audit_event，同一 Idempotency-Key 返回首次结果。
6. 通过 `cargo test`、`just gov-t1`、`task_check T2 --strict`。

---

## 参考

- [ADR-0018 弹性工程](0018-resilience-engineering.md)
- [ADR-0024 鉴权模型](0024-auth-model.md)
- [ADR-0025 审计存储模型](0025-audit-storage-model.md)
- [ADR-0030 H-INT 统一外部系统集成能力](0030-integration-capability.md)
- [M2 入库验收故事](../domain/user-stories-m2-inbound-verify.md)
- [M3 库存查询与批次管理故事](../domain/user-stories-m3-inventory-query.md)
- [M3 库存操作故事](../domain/user-stories-m3-inventory-operation.md)
- [M5 冷链数据集成故事](../domain/user-stories-m5-cold-chain.md)
- [M9 计费故事](../domain/user-stories-m9-billing.md)

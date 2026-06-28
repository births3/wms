# 项目级 RTM 标准与索引

> RTM（需求追溯矩阵）用于把用户故事、前端入口、后端实现、测试证据和合规风险连成可审查链路。本文只放项目级索引和门禁规则；细字段明细继续放各设计文档、用户故事和合规追溯文档。

## 1. 维护原则

- 用户故事 ID 以 `docs/domain/user-stories-*.md` 为唯一来源。
- RTM 按维度拆分，不维护一张人工超级大表。
- 新增或修改故事、API、页面、状态机、字段、合规控制时，必须同步更新对应矩阵。
- `当前结论` 只能写 `已覆盖`、`部分覆盖`、`待补证据`、`不适用`，避免含糊表述。
- 可脚本验证的缺口必须由治理脚本检查，不能只靠人工 review。

## 2. RTM 分层

| RTM | 目的 | 维护位置 | 门禁 |
|---|---|---|---|
| 故事总 RTM | 确认所有故事文件都进入追溯体系 | 本文 §3 | `check_project_rtm.py` |
| 前端体验 RTM | 页面、字段、动作、状态、截图证据 | `docs/*-web-design-plan.md` | `check_web_design_rtm.py` |
| 后端实现 RTM | API、handler/service、domain/repository/migration、测试 | 本文 §5 + 相关后端文件 | `check_project_rtm.py` |
| 测试证据 RTM | 故事到测试命令和证据对象 | 本文 §6 + 运行证据文档 | `check_project_rtm.py` |
| 合规风险 RTM | GSP、审计、幂等、权限、冷链等控制闭环 | `docs/compliance/` + 本文 §7 | `check_project_rtm.py` |

## 3. 故事总 RTM

| 模块/能力 | 用户故事源 | 故事数量 | 当前 RTM |
|---|---|---:|---|
| H-AL 告警引擎 | [user-stories-h-alert.md](domain/user-stories-h-alert.md) | 5 | 合规风险 RTM |
| H-DOCK 月台预约管理 | [user-stories-h-dock-management.md](domain/user-stories-h-dock-management.md) | 7 | 前端体验 RTM / 后端实现 RTM |
| H-Driver 司机端 | [user-stories-h-driver.md](domain/user-stories-h-driver.md) | 5 | 后端实现 RTM / 测试证据 RTM |
| H-Store 门店用户端 | [user-stories-h-store.md](domain/user-stories-h-store.md) | 6 | 后端实现 RTM / 测试证据 RTM |
| H1 权限与多租户 | [user-stories-h1-auth-tenant.md](domain/user-stories-h1-auth-tenant.md) | 6 | 后端实现 RTM / 合规风险 RTM |
| H2 审计追踪 | [user-stories-h2-audit-trail.md](domain/user-stories-h2-audit-trail.md) | 6 | 后端实现 RTM / 合规风险 RTM |
| H3 跨端契约 | [user-stories-h3-contract.md](domain/user-stories-h3-contract.md) | 4 | 后端实现 RTM / 测试证据 RTM |
| H4 企业微信通知 | [user-stories-h4-wechat-notify.md](domain/user-stories-h4-wechat-notify.md) | 4 | 后端实现 RTM |
| H5 快递对接 | [user-stories-h5-express.md](domain/user-stories-h5-express.md) | 6 | 后端实现 RTM |
| M1 主数据商品/供应商/客户 | [user-stories-m1-master-data-product.md](domain/user-stories-m1-master-data-product.md) | 9 | 后端实现 RTM / 合规风险 RTM |
| M1 主数据仓库/库位/配置 | [user-stories-m1-master-data-warehouse.md](domain/user-stories-m1-master-data-warehouse.md) | 12 | 后端实现 RTM / 合规风险 RTM |
| M10 运输协同 | [user-stories-m10-tms-plus.md](domain/user-stories-m10-tms-plus.md) | 4 | 后端实现 RTM / 测试证据 RTM |
| M11 监管 EDI 边界 | [user-stories-m11-regulatory-edi.md](domain/user-stories-m11-regulatory-edi.md) | 2 | 合规风险 RTM |
| M2 ASN 与收货 | [user-stories-m2-inbound-asn.md](domain/user-stories-m2-inbound-asn.md) | 6 | 前端体验 RTM / 后端实现 RTM |
| M2 验收与上架 | [user-stories-m2-inbound-verify.md](domain/user-stories-m2-inbound-verify.md) | 11 | 前端体验 RTM / 后端实现 RTM |
| M3 库存操作 | [user-stories-m3-inventory-operation.md](domain/user-stories-m3-inventory-operation.md) | 9 | 后端实现 RTM / 合规风险 RTM |
| M3 库存查询 | [user-stories-m3-inventory-query.md](domain/user-stories-m3-inventory-query.md) | 4 | 后端实现 RTM |
| M4 出库订单 | [user-stories-m4-outbound-order.md](domain/user-stories-m4-outbound-order.md) | 4 | 前端体验 RTM / 后端实现 RTM |
| M4 拣选复核发货 | [user-stories-m4-outbound-pick.md](domain/user-stories-m4-outbound-pick.md) | 8 | 前端体验 RTM / 后端实现 RTM |
| M4 退货处理 | [user-stories-m4-outbound-return.md](domain/user-stories-m4-outbound-return.md) | 8 | 前端体验 RTM / 后端实现 RTM |
| M5 冷链数据集成 | [user-stories-m5-cold-chain.md](domain/user-stories-m5-cold-chain.md) | 3 | 后端实现 RTM / 合规风险 RTM |
| M6 报表与审计 | [user-stories-m6-audit-report.md](domain/user-stories-m6-audit-report.md) | 7 | 后端实现 RTM / 合规风险 RTM |
| M8 连锁药店 | [user-stories-m8-retail-chain.md](domain/user-stories-m8-retail-chain.md) | 2 | 后端实现 RTM |
| M9 计费管理 | [user-stories-m9-billing.md](domain/user-stories-m9-billing.md) | 3 | 后端实现 RTM |
| M-BA 批号调整 | [user-stories-mba-batch-adjustment.md](domain/user-stories-mba-batch-adjustment.md) | 4 | 后端实现 RTM / 合规风险 RTM |
| M-CG 编码生成 | [user-stories-mcg-code-generator.md](domain/user-stories-mcg-code-generator.md) | 2 | 后端实现 RTM |
| M-DI 药检单查询 | [user-stories-mdi-drug-inspection.md](domain/user-stories-mdi-drug-inspection.md) | 4 | 后端实现 RTM / 合规风险 RTM |
| M-PK 包装站 | [user-stories-mpk-packing-station.md](domain/user-stories-mpk-packing-station.md) | 8 | 后端实现 RTM |
| M-PM 参数对照 | [user-stories-mpm-parameter-mapping.md](domain/user-stories-mpm-parameter-mapping.md) | 6 | 后端实现 RTM |
| M-QL 质量联系单 | [user-stories-mql-quality-liaison.md](domain/user-stories-mql-quality-liaison.md) | 6 | 后端实现 RTM / 合规风险 RTM |
| M-RC 库存对账 | [user-stories-mrc-reconciliation.md](domain/user-stories-mrc-reconciliation.md) | 4 | 后端实现 RTM |
| M-RP 补货 | [user-stories-mrp-replenishment.md](domain/user-stories-mrp-replenishment.md) | 5 | 后端实现 RTM |
| M-SA 报损报溢 | [user-stories-msa-stock-adjustment.md](domain/user-stories-msa-stock-adjustment.md) | 4 | 后端实现 RTM / 合规风险 RTM |
| M-TC 追溯码 | [user-stories-mtc-traceability-code.md](domain/user-stories-mtc-traceability-code.md) | 8 | 后端实现 RTM / 合规风险 RTM |
| M-TE 任务引擎 | [user-stories-mte-task-engine.md](domain/user-stories-mte-task-engine.md) | 11 | 后端实现 RTM |
| M-VR 规则引擎 | [user-stories-mvr-validation-rules.md](domain/user-stories-mvr-validation-rules.md) | 6 | 后端实现 RTM / 合规风险 RTM |

## 4. 前端体验 RTM

| 范围 | 需求来源 | 前端入口 | 设计/截图证据 | 当前结论 |
|---|---|---|---|---|
| M2 PC 入库：收货、验收、上架 | US-M2-002 / US-M2-003 / US-M2-005 / US-M2-006 | `apps/web-admin` 入库业务菜单 | [m2-inbound-web-design-plan.md](m2-inbound-web-design-plan.md) | 部分覆盖 |
| M4 PC 出库：订单、波次、复核发货、采购退货 | US-M4-001 / US-M4-002 / US-M4-004 / US-M4-006 / US-M4-010 | `apps/web-admin` 出库业务菜单 | [m4-outbound-web-design-plan.md](m4-outbound-web-design-plan.md) | 部分覆盖 |
| 原型矩阵与截图证据 | US-H3-001 / US-H3-002 | `prototypes/src/Tabs.tsx` | [prototypes/matrix-e2e-screenshot-gate.md](prototypes/matrix-e2e-screenshot-gate.md) | 部分覆盖 |

## 5. 后端实现 RTM

| 范围 | 需求来源 | API / 契约 | Handler / Service | Domain / Repository / Migration | 测试 / 证据 | 当前结论 |
|---|---|---|---|---|---|---|
| M2 收货单 CRUD 与收货闭环 | US-M2-001 / US-M2-002 | `backend/crates/api/src/lib.rs` inbound OpenAPI；`packages/api-client/src/schema.ts` | `backend/crates/api/src/inbound.rs`；`backend/crates/api/src/wave3_handlers.rs` | `backend/crates/api/src/wave3_repository.rs`；`backend/migrations/202606030001_wave3_core_tables.sql` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 |
| M2 验收、双签、上架入库存 | US-M2-003 / US-M2-004 / US-M2-005 | `/api/v1/inbound/receiving-orders/{id}/inspect`、`/sign`、`/putaway` | `backend/crates/api/src/inbound.rs`；`backend/crates/api/src/wave3_handlers.rs` | `backend/crates/api/src/wave3_repository.rs`；`receiving_inspections`、`receiving_putaways`、`inventory_batches` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 |
| M3 库存状态、批次和幂等基础 | US-M3-001 / US-M3-002 / US-M3-003 / US-M3-004 | `backend/crates/api/src/lib.rs` inventory OpenAPI | `backend/crates/api/src/inventory.rs`；`backend/crates/api/src/wave3_handlers.rs` | `backend/crates/api/src/wave3_repository.rs`；`inventory_status_changes`；`idempotency_request` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 |
| M4 出库订单、波次、拣选、复核、发货 | US-M4-001 / US-M4-002 / US-M4-003 / US-M4-004 / US-M4-006 | `backend/crates/api/src/lib.rs` outbound OpenAPI | `backend/crates/api/src/wave4_handlers.rs` | `backend/crates/api/src/outbound.rs`；`backend/crates/api/src/wave4_repository.rs`；`backend/migrations/202606040001_wave4_outbound_tables.sql` | `backend/crates/api/tests/wave4_postgres.rs` | 已覆盖 |
| M4 追溯码出库上报 | US-M4-006 / US-TC-005 / US-TC-006 | `/api/v1/traceability/outbound-reports` | `backend/crates/api/src/wave4_handlers.rs` | `backend/crates/api/src/wave4_repository.rs`；`backend/crates/api/src/traceability_code.rs` | `backend/crates/api/tests/wave4_postgres.rs` | 已覆盖 |
| H1/H2 认证、审计、不可篡改证据 | US-H1-001 / US-H2-001 / US-H2-002 / US-H2-003 | `/api/v1/auth/*`、`/api/v1/audit/events` | `backend/crates/api/src/auth_handlers.rs`；`backend/crates/api/src/audit.rs` | `backend/crates/api/src/auth_service.rs`；`backend/crates/api/src/auth_repository.rs`；`backend/migrations/202606020001_audit_event.sql` | `backend/crates/api/tests/audit_postgres.rs` | 已覆盖 |
| M5/M10/H5 外部协作与价值增值闭环 | US-M5-001 / US-M10-001 / US-H5-001 | `backend/crates/api/src/lib.rs` cold-chain、tms、express OpenAPI | `backend/crates/api/src/wave5_handlers.rs` | `backend/crates/api/src/wave5_repository.rs`；`backend/migrations/202606050001_wave5_value_added_tables.sql` | `backend/crates/api/tests/wave5_postgres.rs` | 部分覆盖 |

## 6. 测试证据 RTM

| 范围 | 需求来源 | 验证命令 | 证据对象 | 当前结论 |
|---|---|---|---|---|
| T1 治理门禁 | US-H3-001 / US-H3-002 | `just gov-t1` | `scripts/governance/governance_checks.py` | 已覆盖 |
| Web 设计 RTM | US-M2-002 / US-M4-001 | `python3 scripts/governance/check_web_design_rtm.py --json` | `docs/*-web-design-plan.md` | 已覆盖 |
| 项目级 RTM | US-H3-001 / US-H3-002 | `python3 scripts/governance/check_project_rtm.py --json` | 本文 | 已覆盖 |
| 入库/库存后端闭环 | US-M2-002 / US-M2-003 / US-M2-005 / US-M3-004 | `cargo test -p wms-api --test wave3_postgres` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 |
| 出库后端闭环 | US-M4-001 / US-M4-003 / US-M4-004 / US-M4-006 | `cargo test -p wms-api --test wave4_postgres` | `backend/crates/api/tests/wave4_postgres.rs` | 已覆盖 |
| 审计不可篡改 | US-H2-001 / US-H2-002 | `cargo test -p wms-api --test audit_postgres` | `backend/crates/api/tests/audit_postgres.rs` | 已覆盖 |

## 7. 合规风险 RTM

| 范围 | 需求来源 | 合规/风险来源 | 控制措施 | 证据对象 | 当前结论 |
|---|---|---|---|---|---|
| GSP 字段级追溯 | US-M1-001 / US-M2-003 / US-M4-006 / US-TC-001 | [compliance/gsp-field-traceability.md](compliance/gsp-field-traceability.md) | 字段名、故事字段表、合规字段状态三向核对 | `check_gsp_field_traceability.py` | 已覆盖 |
| GSP 条款级追溯 | US-H2-001 / US-M2-003 / US-M4-006 / US-M5-001 | [compliance/README.md](compliance/README.md) | 条款到故事、测试、审计证据映射 | `docs/compliance/*.md` | 部分覆盖 |
| 审计追加不可篡改 | US-H2-001 / US-H2-002 / US-H2-003 | 审计表只能 INSERT | DB trigger + hash chain seal | `backend/migrations/202606020001_audit_event.sql`；`audit_postgres.rs` | 已覆盖 |
| 写操作幂等 | US-M2-002 / US-M4-001 / US-M4-006 | 重复提交、弱网重试、PDA 离线补传 | `Idempotency-Key` + `idempotency_request` | `wave3_postgres.rs`；`wave4_postgres.rs` | 已覆盖 |
| 货主隔离 | US-H1-001 / US-H1-002 | 多货主 / 3PL 数据隔离 | 后端查询显式使用 `ctx.owner_id` | repository 测试与代码 review | 部分覆盖 |
| 冷链温控异常 | US-M5-001 / US-M5-002 / US-M5-003 | 温湿度越界、批次隔离、外部设备证据 | 冷链事件接入 + 库存隔离 + 审计 | `wave4_postgres.rs`；`wave5_postgres.rs` | 部分覆盖 |

## 8. Review 规则

| 变更类型 | 必须检查 |
|---|---|
| 新增用户故事 | §3 故事总 RTM 增加一行；相关维度矩阵至少一行引用该故事 |
| 新增前端页面或按钮 | 前端体验 RTM 和对应 `*-web-design-plan.md` 字段/动作/状态/证据 RTM |
| 新增后端 API | 后端实现 RTM 记录 API、handler/service、domain/repository/migration、测试 |
| 新增合规或风险控制 | 合规风险 RTM 记录来源、控制措施和证据 |
| 新增治理脚本 | 接入 `governance_checks.py`、`gate-rules.toml`、smoke 测试和本文测试证据 RTM |


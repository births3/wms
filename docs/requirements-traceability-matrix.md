# 项目级 RTM 标准与索引

> RTM（需求追溯矩阵）用于把用户故事、前端入口、后端实现、测试证据和合规风险连成可审查链路。本文只放项目级索引和门禁规则；细字段明细继续放各设计文档、用户故事和合规追溯文档。

## 1. 维护原则

- 用户故事 ID 以 `docs/domain/user-stories-*.md` 为唯一来源。
- RTM 按维度拆分，不维护一张人工超级大表。
- 新增或修改故事、API、页面、状态机、字段、合规控制时，必须同步更新对应矩阵。
- `当前结论` 只能写 `已覆盖`、`部分覆盖`、`待补证据`、`不适用`，避免含糊表述。
- `部分覆盖` 和 `待补证据` 必须填写 `缺口说明` 与 `补齐路径`。
- 可脚本验证的缺口必须由治理脚本检查，不能只靠人工 review。

## 2. RTM 分层

| RTM | 目的 | 维护位置 | 门禁 |
|---|---|---|---|
| 故事总 RTM | 确认所有故事文件都进入追溯体系 | 本文 §3 | `check_project_rtm.py` |
| 前端体验 RTM | 页面、字段、动作、状态、截图证据 | `docs/*-web-design-plan.md` | `check_web_design_rtm.py` |
| 后端实现 RTM | API、handler/service、domain/repository/migration、测试 | 本文 §5 + 相关后端文件 | `check_project_rtm.py` |
| 测试证据 RTM | 故事到测试命令和证据对象 | 本文 §6 + 运行证据文档 | `check_project_rtm.py` |
| 合规风险 RTM | GSP、审计、幂等、权限、冷链等控制闭环 | `docs/compliance/` + 本文 §7 | `check_project_rtm.py` |

### 2.1 H3 OpenAPI 契约 P0 切片覆盖矩阵

| 层 | 需要 | 本轮覆盖 | 证据 | 缺口 |
|---|---|---|---|---|
| 用户故事 | 是 | 是 | [user-stories-h3-contract.md](domain/user-stories-h3-contract.md) 的 US-H3-001 / US-H3-002 / US-H3-003 / US-H3-004 | 无。 |
| RTM | 是 | 是 | 本节 + [quality-matrix.md](governance/quality-matrix.md) | 无。 |
| 数据库 | 是 | 是 | H3 限流 / 熔断事件复用 H2 `audit_event` append-only 表；不新增 H3 业务表。 | 无。 |
| 后端服务 | 是 | 是 | `backend/crates/api/src/resilience.rs`、`backend/crates/api/src/bin/wms_api.rs` | 无。 |
| 公开 API | 是 | 是 | `GET /openapi.json`、`GET /api-docs`、`GET /redoc`、`GET /api/v1/resilience/status`、`GET /metrics` | 无。 |
| OpenAPI / api-client | 是 | 是 | `shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | 无。 |
| PC 前端 | 不适用 | 是 | H3 为后端契约和在线文档能力；文档 UI 由 Swagger UI / ReDoc runtime 页面承接。 | 无。 |
| PDA | 是 | 是 | PDA 复用 `packages/api-client/src/schema.ts`，不重复定义类型。 | 真 PDA 离线运行证据属于 Wave 3 PDA gate，不作为 H3 契约完成条件。 |
| E2E / 截图 | 不适用 | 是 | H3 后端路由、文档模式和审计写入由 Rust 测试覆盖。 | 无。 |
| 治理脚本 | 是 | 是 | `scripts/governance/check_openapi_contract.py`、`scripts/governance/check_quality_matrix.py`、`scripts/governance/generate_openapi_curl_examples.py` | 无。 |
| 文档 / runbook | 是 | 是 | [h3-api-access.md](api/h3-api-access.md)、[curl-examples.md](api/curl-examples.md)、[openapi-changelog.md](api/openapi-changelog.md) | 无。 |

## 3. 故事总 RTM

| 模块/能力 | 用户故事源 | 故事数量 | 当前 RTM |
|---|---|---:|---|
| H-AL 告警引擎 | [user-stories-h-alert.md](domain/user-stories-h-alert.md) | 5 | 前端体验 RTM / 后端实现 RTM / 测试证据 RTM / 合规风险 RTM |
| H-DOCK 月台预约管理 | [user-stories-h-dock-management.md](domain/user-stories-h-dock-management.md) | 7 | 前端体验 RTM / 后端实现 RTM |
| H-Driver 司机端 | [user-stories-h-driver.md](domain/user-stories-h-driver.md) | 5 | 后端实现 RTM / 测试证据 RTM |
| H-Store 门店用户端 | [user-stories-h-store.md](domain/user-stories-h-store.md) | 6 | 后端实现 RTM / 测试证据 RTM |
| H1 权限与多租户 | [user-stories-h1-auth-tenant.md](domain/user-stories-h1-auth-tenant.md) | 6 | 后端实现 RTM / 合规风险 RTM |
| H2 审计追踪 | [user-stories-h2-audit-trail.md](domain/user-stories-h2-audit-trail.md) | 6 | 后端实现 RTM / 合规风险 RTM |
| H3 跨端契约 | [user-stories-h3-contract.md](domain/user-stories-h3-contract.md) | 4 | 后端实现 RTM / 测试证据 RTM |
| H4 企业微信通知 | [user-stories-h4-wechat-notify.md](domain/user-stories-h4-wechat-notify.md) | 4 | 后端实现 RTM |
| H5 快递对接 | [user-stories-h5-express.md](domain/user-stories-h5-express.md) | 6 | 后端实现 RTM |
| H6 状态机引擎 | [user-stories-h6-state-machine.md](domain/user-stories-h6-state-machine.md) | 1 | 后端实现 RTM / 测试证据 RTM / 合规风险 RTM |
| H9 打印模板引擎 | [user-stories-h9-print-template.md](domain/user-stories-h9-print-template.md) | 5 | 前端体验 RTM / 后端实现 RTM / 测试证据 RTM / 合规风险 RTM |
| M1 主数据商品/供应商/客户 | [user-stories-m1-master-data-product.md](domain/user-stories-m1-master-data-product.md) | 9 | 后端实现 RTM / 合规风险 RTM |
| M1 主数据仓库/库位/配置 | [user-stories-m1-master-data-warehouse.md](domain/user-stories-m1-master-data-warehouse.md) | 13 | 后端实现 RTM / 合规风险 RTM |
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
| M-QL 质量联系单 | [user-stories-mql-quality-liaison.md](domain/user-stories-mql-quality-liaison.md) | 5 | 后端实现 RTM / 合规风险 RTM |
| M-RC 库存对账 | [user-stories-mrc-reconciliation.md](domain/user-stories-mrc-reconciliation.md) | 4 | 后端实现 RTM |
| M-RP 补货 | [user-stories-mrp-replenishment.md](domain/user-stories-mrp-replenishment.md) | 5 | 后端实现 RTM |
| M-SA 报损报溢 | [user-stories-msa-stock-adjustment.md](domain/user-stories-msa-stock-adjustment.md) | 4 | 后端实现 RTM / 合规风险 RTM |
| M-TC 追溯码 | [user-stories-mtc-traceability-code.md](domain/user-stories-mtc-traceability-code.md) | 8 | 后端实现 RTM / 合规风险 RTM |
| M-TE 任务引擎 | [user-stories-mte-task-engine.md](domain/user-stories-mte-task-engine.md) | 11 | 后端实现 RTM |
| M-VR 规则引擎 | [user-stories-mvr-validation-rules.md](domain/user-stories-mvr-validation-rules.md) | 6 | 后端实现 RTM / 合规风险 RTM |

## 4. 前端体验 RTM

| 范围 | 需求来源 | 前端入口 | 设计/截图证据 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|
| H-AL PC 告警定义 | US-AL-001 | `apps/web-admin` 基础能力 / H-AL 告警定义 | `prototypes/e2e/web-admin-hal-real.spec.ts`；`artifacts/screenshot-portal/real-web/h-al-alert-definitions/alert-definition-approved.png` | 已覆盖 | 无 | 后续告警生命周期、升级、看板和通道静默分别按 US-AL-002~005 扩展，不回填到定义页。 |
| H-AL PC 活跃告警与生命周期 | US-AL-002 | `apps/web-admin` 基础能力 / H-AL 告警看板 | `prototypes/e2e/web-admin-hal-real.spec.ts`；`artifacts/screenshot-portal/real-web/h-al-alert-dashboard/active-alerts.png` | 部分覆盖 | PC 查询、确认、处理、关闭、忽略及真实浏览器证据已覆盖；ADR-0027 尚未 Accepted，真 PDA 离线重放与企业微信点击确认缺外部证据。 | 保持延期；ADR-0027 Accepted 且具备设备/企微环境后补 PDA 与外部回调证据。 |
| H-AL PC 告警升级 | US-AL-003 | `apps/web-admin` 基础能力 / H-AL 升级规则 | `prototypes/e2e/web-admin-hal-real.spec.ts`；`artifacts/screenshot-portal/real-web/h-al-alert-escalations/escalation-rule.png` | 已覆盖 | 无 | 规则变更时保持最多三级、非工作时段路由、接收人回退和幂等升级测试同步。 |
| H-AL PC 看板与统计 | US-AL-004 | `apps/web-admin` 基础能力 / H-AL 告警看板 | `prototypes/e2e/web-admin-hal-real.spec.ts`；`artifacts/screenshot-portal/real-web/h-al-alert-dashboard/statistics-and-export.png` | 已覆盖 | 无 | 新增筛选、统计或导出格式时同步更新同筛选缓存、OpenAPI、真实浏览器动作和截图。 |
| M2 PC 入库：收货、验收、上架 | US-M2-002 / US-M2-003 / US-M2-005 / US-M2-006 | `apps/web-admin` 入库业务菜单 | [m2-inbound-web-design-plan.md](m2-inbound-web-design-plan.md) | 部分覆盖 | 收货扩展字段、整单拒收、质量核对明细、推荐库位校验仍未形成完整 OpenAPI / 后端持久化闭环。 | 按 M2 设计方案 §7.3-§7.5 补 API、后端、前端动作验证和真实截图；不得用原型截图替代。 |
| M4 PC 出库：订单、波次、复核发货、采购退货 | US-M4-001 / US-M4-002 / US-M4-004 / US-M4-006 / US-M4-010 | `apps/web-admin` 出库业务菜单 | [m4-outbound-web-design-plan.md](m4-outbound-web-design-plan.md)；`prototypes/e2e/web-admin-m4-real.spec.ts` | 部分覆盖 | 已接 M4 出库订单列表、刷新、新建、详情、波次列表、波次详情和波次创建真实 API，真实临时数据库 E2E 已验证单据类型、自动单号、列表回显、详情响应、库存分配、波次刷新和截图；容量/路径规则、下发取消、校验、复核、发货、采购退货仍未形成完整后端闭环。 | 按 M4 设计方案 §8.2-§8.5 补容量/路径/动作及其他出库动作 API、动作测试和真实截图。 |
| M1 PC 系统字典中心 | US-M1-011 | `apps/web-admin` 系统管理 / 系统字典菜单 | 待补真实管理页设计与截图 | 待补证据 | 系统字典管理页、字典项参数 schema、导入导出、审批和影响预览尚未实现。 | 先按 US-M1-011 补 OpenAPI / 后端 / 前端管理页，再补真实截图和动作测试；M2/M4 单据类型接入同一字典源。 |
| H1 PC 三层菜单管理 | US-H1-007 | `apps/web-admin` 基础能力 / H1 权限租户 / H1 菜单管理 | [h1-menu-management-design.md](h1-menu-management-design.md) | 部分覆盖 | 已覆盖后端菜单表、草稿/发布/版本 API、前端三层菜单消费、菜单管理页、按钮权限点维护、OpenAPI 和 api-client；9002 真实截图归后续证据收口。 | 后续补角色授权矩阵、真实用户权限组合 E2E、9002 截图归档和发布回滚操作证据。 |
| M-CG PC 单据号规则管理 | US-CG-001 / US-CG-002 | `apps/web-admin` 系统管理 / 单据号规则菜单 | `prototypes/e2e/web-admin-mcg-real.spec.ts`；`apps/web-admin/.e2e-artifacts/mcg-real/screenshots/rule-created.png`；`prototypes/e2e/web-admin-m4-real.spec.ts`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-order-created.png`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-order-detail.png`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-wave-created.png`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-wave-detail.png` | 部分覆盖 | PC 规则管理页、规则预览、动作测试、M1 单据类型字典绑定、M2 ASN 和 M4 出库订单列表/创建/详情/波次列表/详情/创建真实 API 及浏览器证据已实现；配置审批、规则变更审计展示、M4 其他动作、M3/其他创建单据接入和正式版证据仍未完成。 | 后续补配置审批、审计展示、M4 其他动作、M3/其他创建单据接入及正式版发布证据。 |
| H9 PC 打印模板中心 | US-H9-001 / US-H9-002 / US-H9-003 / US-H9-004 / US-H9-005 | `apps/web-admin` 基础能力 / H9 打印模板菜单 | 已补模板类型树、模板列表、hiprint 设计弹窗、预览弹窗；既有 `pc-m2-009` 原型仅作历史参考 | 部分覆盖 | H9 独立菜单、模板类型字典、字段库读取 API、模板主数据、hiprint JSON 版本、浏览器预览打印和打印记录已形成首个真实闭环；9002 真实截图、更多业务单据真实数据和依赖安全跟踪仍需补证据。 | 后续补 9002 真实截图、M2/M4/标签真实数据接入、动作 E2E 和 hiprint 依赖安全复核。 |
| 原型矩阵与截图证据 | US-H3-001 / US-H3-002 | `prototypes/src/Tabs.tsx` | [prototypes/matrix-e2e-screenshot-gate.md](prototypes/matrix-e2e-screenshot-gate.md) | 部分覆盖 | 原型矩阵覆盖 207 个 tab，但视觉回归依赖正确 dev server 和最新 baseline；生产真实前端截图另走 `apps/web-admin` 证据。 | 用正确端口重新 capture 原型 baseline；生产截图按 Matrix E2E 截图门禁“测试环境查看标准”单独归档。 |

## 5. 后端实现 RTM

| 范围 | 需求来源 | API / 契约 | Handler / Service | Domain / Repository / Migration | 测试 / 证据 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|---|---|
| H-AL 告警定义注册 | US-AL-001 | `GET /api/v1/alert-definitions*`；`POST /api/v1/alert-definitions/change-requests`；M-QL 类型/审批回调；OpenAPI/api-client | `alert_definition_handlers.rs`；`alert_definition_service.rs`；`quality_liaison/actions.rs` | `alert_definition.rs`；`alert_definition_repository.rs`；`202607130015_h1_alert_definitions.sql`；`202607150011_hal_alert_definition_workflow.sql` | `alert_definition_postgres.rs`；`alert_definition_repository_postgres.rs`；`alert_definition_change_postgres.rs`；真实 PC E2E | 已覆盖 | 无 | 定义变更继续保持 M-QL 审批原子应用、GSP 不可停删和 append-only H2 审计。 |
| H-AL 告警触发与生命周期 | US-AL-002 | `GET /api/v1/alerts*`；`POST /api/v1/alerts/{id}/{acknowledge,handling,close,ignore}`；OpenAPI/api-client | `alert_engine_job.rs`；`alert_lifecycle_service.rs`；`alert_instance_handlers.rs` | `alert_engine.rs`；`alert_instance_repository.rs`；`202607150012_hal_alert_runtime.sql` | `alert_lifecycle_postgres.rs`；PC 真实 E2E | 部分覆盖 | H2 条件匹配、去重、H4 重试、状态机和 PC 动作已覆盖；真 PDA/企微外部确认链受 ADR-0027 与外部环境阻塞。 | 保持质量矩阵延期项，取得真实设备和企微环境后补端侧/回调测试与证据。 |
| H-AL 告警升级 | US-AL-003 | `GET/PUT /api/v1/alert-escalation-rules*`；OpenAPI/api-client | `alert_escalation_handlers.rs`；`alert_engine_job.rs` | `alert_escalation.rs`；`202607150013_hal_alert_escalation.sql` | `alert_escalation_postgres.rs`；PC 真实 E2E | 已覆盖 | 无 | 保持三级上限、夜间/节假日路由、H1 角色回退、重复升级跳过和 H2 审计。 |
| H-AL 看板、统计与报表 | US-AL-004 | `GET /api/v1/alerts/{active,statistics,gsp-report,changes}`；`POST/GET /api/v1/alerts/exports*`；OpenAPI/api-client | `alert_dashboard_handlers.rs`；`alert_dashboard.rs` | `alert_statistics_snapshots`；`alert_report_exports`；`202607150014_hal_alert_dashboard.sql` | `alert_dashboard_postgres.rs`；PC 真实 E2E | 已覆盖 | 无 | 保持仓库范围、同筛选统计快照回退、10 万行异步导出、7 天下载和查询审计。 |
| M2 收货单 CRUD 与收货闭环 | US-M2-001 / US-M2-002 | `backend/crates/api/src/lib.rs` inbound OpenAPI；`packages/api-client/src/schema.ts` | `backend/crates/api/src/inbound.rs`；`backend/crates/api/src/wave3_handlers.rs` | `backend/crates/api/src/wave3_repository.rs`；`backend/migrations/202606030001_wave3_core_tables.sql` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 | 无 | 保持 OpenAPI、api-client、repository 测试同步。 |
| M2 验收、双签、上架入库存 | US-M2-003 / US-M2-004 / US-M2-005 | `/api/v1/inbound/receiving-orders/{id}/inspect`、`/sign`、`/putaway` | `backend/crates/api/src/inbound.rs`；`backend/crates/api/src/wave3_handlers.rs` | `backend/crates/api/src/wave3_repository.rs`；`receiving_inspections`、`receiving_putaways`、`inventory_batches` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 | 无 | 扩字段时先补用户故事字段表和 OpenAPI，再补 repository 测试。 |
| M1 系统字典中心 | US-M1-011 | `backend/crates/api/src/lib.rs` system-dictionary OpenAPI | `backend/crates/api/src/system_dictionary_handlers.rs` | `backend/crates/api/src/system_dictionary.rs`；`backend/migrations/202606280001_system_dictionary.sql` | `backend/crates/api/tests/system_dictionary_postgres.rs`；`backend/crates/api/src/system_dictionary_tests.rs` | 部分覆盖 | 首批后端覆盖字典表、`document_type` 预置项、参数 schema、货主覆盖和基础幂等；M-QL 审批、H2-005 事件、导入导出、影响预览和 PC 管理页仍未实现。 | 后续分组补 M-QL 审批、H2-005 事件、导入导出、M2/M4 运行时字典校验和真实前端截图。 |
| H1 三层菜单管理 | US-H1-007 | `backend/crates/api/src/lib.rs` admin-menu OpenAPI；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | `backend/crates/api/src/admin_menu_handlers.rs` | `backend/crates/api/src/admin_menu.rs`；`backend/crates/api/src/admin_menu_repository.rs`；`backend/migrations/202607050003_h1_admin_menu.sql` | `backend/crates/api/tests/admin_menu_postgres.rs`；`apps/web-admin/self-checks/app-shell-layout-self-check.mjs` | 部分覆盖 | 已覆盖草稿、发布版本、按钮权限点、权限过滤、幂等、OpenAPI、api-client、前端 H1 菜单管理页和前端壳已发布菜单消费；角色授权矩阵页面和 9002 真实截图归后续证据收口。 | 后续补角色授权矩阵、真实用户权限组合 E2E、9002 截图归档和发布回滚操作证据。 |
| M-CG 单据号生成后端第一切片 | US-CG-001 / US-CG-002 | `backend/crates/api/src/lib.rs` code-generator OpenAPI；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | `backend/crates/api/src/document_numbering_handlers.rs`；`backend/crates/api/src/document_numbering_repository.rs`；`backend/crates/api/src/document_numbering.rs`；`backend/crates/api/src/wave4_repository_part1.rs` | `backend/migrations/202607020001_mcg_document_numbering.sql`；`backend/migrations/202607130004_m4_outbound_document_type.sql`；`document_number_rules`、`document_number_counters`、`document_number_allocations`；`outbound_orders.document_type`；`idempotency_request`；`audit_event` | `backend/crates/api/tests/document_numbering_postgres.rs`；`backend/crates/api/tests/wave4_postgres.rs`；`apps/web-admin/self-checks/mcg-document-numbering-slice-self-check.mjs`；`apps/web-admin/self-checks/m4-outbound-create-api-self-check.mjs`；`prototypes/e2e/web-admin-mcg-real.spec.ts`；`prototypes/e2e/web-admin-m4-real.spec.ts`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-order-created.png`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-order-detail.png` | 部分覆盖 | 已覆盖 no-gap 发号、计数器、生成记录、调用方事务接口、规则列表、规则 upsert、规则启停、读写权限、L11 幂等、M1 单据类型字典校验、M2 ASN 和 M4 出库订单、M4 列表/创建/详情同事务接入、OpenAPI/api-client、临时 PostgreSQL 真实浏览器 E2E 和截图；配置审批、规则变更审计展示、M4 其他动作、M3/其他业务模块接入及正式版证据仍未实现。 | 后续补配置审批、审计展示、M4 其他动作、M3/其他创建单据接入和正式版发布证据。 |
| H4 企业微信参数设置与本地测试 | US-H4-002 / US-H4-003 / US-H4-004 | `POST /api/v1/wechat-notify/settings/test`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | `backend/crates/api/src/wechat_notify.rs`；`backend/crates/api/src/wechat_notify_service.rs` | `backend/crates/domain/src/h4.rs`；`backend/migrations/202607090005_h4_wechat_settings.sql`；`backend/migrations/202607100002_h4_system_admin_permissions.sql` | `backend/crates/api/tests/h4_wechat_notify_postgres.rs`；`prototypes/e2e/web-admin-h4-dev.spec.ts`；`just openapi-check` | 部分覆盖 | 已覆盖参数完整性、URL、启用状态、普通用户记录隔离、指定审批人回写、幂等、OpenAPI 和管理端错误展示；未接 provider 时发送记录明确为失败，不冒充企业微信外网送达。 | 企业微信真实发送、外部回调签名和联调证据按延期故事单独收口。 |
| H6 状态机定义注册与转换校验 | US-H6-001 | `backend/crates/api/src/openapi_paths/extensions.rs`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | `backend/crates/api/src/state_machine.rs`；`backend/crates/api/src/bin/wms_api.rs` | `backend/crates/domain/src/master_dictionary.rs`；代码内不可变状态机定义；`backend/migrations/202607100001_h6_state_machine_permission.sql`；`backend/migrations/202607100003_system_admin_permission_sync.sql` | `cargo test --manifest-path backend/Cargo.toml -p wms-api state_machine`；`cargo test --manifest-path backend/Cargo.toml -p wms-api --test h6_state_machine_postgres`；`check_scope_gap_discovery --strict --module H6` | 已覆盖 | 无 | 后续业务模块接入状态执行时，再补事务原子性、H2 审计和领域事件发布测试。 |
| H9 打印模板引擎 | US-H9-001 / US-H9-002 / US-H9-003 / US-H9-004 / US-H9-005 | `backend/crates/api/src/lib.rs` print-template OpenAPI；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | `backend/crates/api/src/print_template_handlers.rs`；`backend/crates/api/src/print_template.rs` | `backend/crates/api/src/print_template.rs`；`backend/migrations/202607050002_h9_print_template.sql`；`backend/migrations/202607060001_h9_print_template_type_dictionary.sql`；`backend/migrations/202607070001_h9_print_template_runtime.sql`；字段库、字段库版本、字段定义表、模板主数据、模板版本、打印记录、模板类型字典 | `backend/crates/api/tests/print_template_postgres.rs`；`backend/crates/api/tests/system_dictionary_postgres.rs`；`pnpm --dir apps/web-admin run build`；`check_scope_gap_discovery --strict --module H9` | 部分覆盖 | 已覆盖模板类型字典、字段库发布、版本不可改写、幂等重放、审计写入、模板版本、hiprint JSON、字段绑定校验、必填字段缺失、打印记录、跨货主不可回退、OpenAPI/api-client 和 PC hiprint 入口；9002 真实截图和更多业务单据真实数据仍需补证据。 | 后续补 9002 真实截图、M2/M4/标签真实数据接入、动作 E2E、依赖安全复核和静默打印客户端后续边界。 |
| M3 库存状态、批次和幂等基础 | US-M3-001 / US-M3-002 / US-M3-003 / US-M3-004 | `backend/crates/api/src/lib.rs` inventory OpenAPI | `backend/crates/api/src/inventory.rs`；`backend/crates/api/src/wave3_handlers.rs` | `backend/crates/api/src/wave3_repository.rs`；`inventory_status_changes`；`idempotency_request` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 | 无 | 保持 L11 幂等和 owner scope 测试。 |
| M-QL 质量联系单后端主链 | US-QL-001 / US-QL-002 / US-QL-003 / US-QL-004 | `PUT /api/v1/quality-liaisons/types/{type_code}`；`POST /api/v1/quality-liaisons`；`GET /api/v1/quality-liaisons/{id}`；`POST /api/v1/quality-liaisons/{id}/approval-callback`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | `backend/crates/api/src/quality_liaison_handlers.rs`；`backend/crates/api/src/quality_liaison.rs`；`backend/crates/api/src/quality_liaison/actions.rs` | `backend/crates/domain/src/quality_liaison.rs`；`backend/migrations/202607150010_mql_quality_liaison.sql`；`quality_liaison_types`；`quality_liaison_orders`；`h4_approval_records` | `backend/crates/api/tests/quality_liaison_postgres.rs`；`just openapi-check` | 部分覆盖 | 已覆盖类型/H4 模板与指定审批人配置、M-CG 发号、手工创建、详情、H4 审批记录同事务创建、同意/拒绝回写、审批意见、权限、货主隔离、审计、幂等，以及审批通过后同事务创建 M-SA 销毁报损单和失败回滚。尚缺预置类型默认值、角色/货主审批规则、业务模块自动触发、真实企业微信签名回调与超时提醒、其余处置动作、ERP 档案补录闭环、查询统计、PC/PDA 页面及外部证据。 | 按 US-QL-001~005 继续补自动触发、外部 H4、ERP、其余联动、查询统计和端侧证据；完整前保持故事延期。 |
| M-SA 报损报溢后端主链 | US-SA-001 / US-SA-002 / US-VR-006 | `POST/GET /api/v1/stock-adjustments/loss-orders*`；`POST/GET /api/v1/stock-adjustments/surplus-orders*`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | `backend/crates/api/src/stock_adjustment_handlers.rs`；`backend/crates/api/src/stock_adjustment.rs`；`backend/crates/api/src/stock_adjustment/surplus.rs`；`backend/crates/api/src/stock_adjustment/quality_liaison.rs` | `backend/crates/domain/src/stock_adjustment.rs`；`backend/migrations/202607150008_msa_stock_loss.sql`；`backend/migrations/202607150009_msa_stock_surplus.sql`；`stock_adjustment_orders`；`stock_adjustment_execution_records`；`stock_adjustment_erp_feedback_outbox` | `backend/crates/api/tests/stock_adjustment_postgres.rs`；`backend/crates/api/tests/stock_adjustment_postgres/surplus.rs`；`backend/crates/api/tests/quality_liaison_postgres.rs`；`just openapi-check` | 部分覆盖 | 已覆盖报损/报溢手工与 ERP 创建、M-CG 发号、ERP 外部号并发去重、M-QL 审批通过创建销毁报损单、报损/报溢/销毁 M-VR 三档策略、双人/H4 门禁、M2 报溢库位规则、M3 库存与容量原子变更、召回销毁标记解除、审计、幂等、权限及 ERP outbox；尚缺真实 H4 外部回调、outbox 投递 worker/ERP 回执、PDA 扫码离线链及真机证据。 | 补 H4 外部回调与 ERP outbox worker，再在 ADR-0027 解禁后补 PDA 真机验收；完整前继续保持故事延期。 |
| M4 出库订单、波次、拣选、复核、发货 | US-M4-001 / US-M4-002 / US-M4-003 / US-M4-004 / US-M4-006 | `backend/crates/api/src/lib.rs` outbound OpenAPI | `backend/crates/api/src/wave4_handlers.rs` | `backend/crates/api/src/outbound.rs`；`backend/crates/api/src/wave4_repository.rs`；`backend/migrations/202606040001_wave4_outbound_tables.sql`；`backend/migrations/202607130004_m4_outbound_document_type.sql` | `backend/crates/api/tests/wave4_postgres.rs`；`prototypes/e2e/web-admin-m4-real.spec.ts`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-order-created.png`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-order-detail.png`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-wave-created.png`；`apps/web-admin/.e2e-artifacts/m4-real/screenshots/outbound-wave-detail.png` | 部分覆盖 | M4 后端已支持独立 `document_type`、M1 出库字典校验、M-CG 同事务自动编号、新建波次确认订单校验和库存分配、波次列表和详情查询；管理端已将出库订单列表、刷新、新建、详情、波次列表、刷新、创建和详情接入真实 API，并用临时 PostgreSQL 浏览器 E2E 验证自动单号、列表回显、详情响应、库存分配和截图，但容量/路径规则与后续动作仍未接入完整真实 API。 | 前端继续复用现有 Wave4 API，补容量/路径/下发/取消、校验/复核/发货/退货动作 API、单据类型筛选和各动作真实截图。 |
| M4 追溯码出库上报 | US-M4-006 / US-TC-005 / US-TC-006 | `/api/v1/traceability/outbound-reports` | `backend/crates/api/src/wave4_handlers.rs` | `backend/crates/api/src/wave4_repository.rs`；`backend/crates/api/src/traceability_code.rs` | `backend/crates/api/tests/wave4_postgres.rs` | 已覆盖 | 无 | 真实平台 evidence 归入 Wave4 外部依赖 gate。 |
| H1/H2 认证、审计、不可篡改证据 | US-H1-001 / US-H2-001 / US-H2-002 / US-H2-003 | `/api/v1/auth/*`、`/api/v1/audit/events` | `backend/crates/api/src/auth_handlers.rs`；`backend/crates/api/src/audit.rs` | `backend/crates/api/src/auth_service.rs`；`backend/crates/api/src/auth_repository.rs`；`backend/migrations/202606020001_audit_event.sql` | `backend/crates/api/tests/audit_postgres.rs` | 已覆盖 | 无 | 保持审计表 append-only，不允许 UPDATE / DELETE。 |
| M5/M10/H5 外部协作与价值增值闭环 | US-M5-001 / US-M10-001 / US-H5-001 | `backend/crates/api/src/lib.rs` cold-chain、tms、express OpenAPI | `backend/crates/api/src/wave5_handlers.rs` | `backend/crates/api/src/wave5_repository.rs`；`backend/migrations/202606050001_wave5_value_added_tables.sql` | `backend/crates/api/tests/wave5_postgres.rs` | 部分覆盖 | 仓库内闭环和测试已覆盖，真实 TMS、硬件、快递或冷链设备 evidence 仍依赖外部系统。 | 按 `docs/runbooks/wave-5-hardware-evidence.md`、`docs/runbooks/wave-5-tms-evidence.md` 和 Wave6 evidence gate 采集真实引用。 |

## 6. 测试证据 RTM

| 范围 | 需求来源 | 验证命令 | 证据对象 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|
| H-AL 告警定义 | US-AL-001 | `cargo test ... --test alert_definition_postgres --test alert_definition_repository_postgres --test alert_definition_change_postgres`；`pnpm --dir apps/web-admin run test:e2e:hal-real` | PostgreSQL 仓储/API/M-QL 原子测试；Playwright 报告与真实截图 | 已覆盖 | 无 | 变更字段、审批动作或 GSP 规则时同步扩展三组 PostgreSQL 测试与真实浏览器用例。 |
| H-AL 生命周期、升级与看板 | US-AL-002 / US-AL-003 / US-AL-004 | `cargo test ... --test alert_lifecycle_postgres --test alert_escalation_postgres --test alert_dashboard_postgres`；`pnpm --dir apps/web-admin run test:e2e:hal-real` | PostgreSQL 条件/状态/升级/权限/缓存/导出测试；Playwright 三个菜单页真实流程与截图 | 部分覆盖 | US-AL-003/004 已覆盖；US-AL-002 仅缺真 PDA 离线重放和企业微信点击确认外部证据。 | 本地回归继续成组运行；外部证据未满足前不关闭 US-AL-002。 |
| T1 治理门禁 | US-H3-001 / US-H3-002 | `just gov-t1` | `scripts/governance/governance_checks.py` | 已覆盖 | 无 | 新增治理脚本时同步 smoke、T1 和 gate-rules。 |
| 系统字典文档对齐 | US-M1-011 / US-M2-002 / US-M4-001 / US-M4-012 | `python3 scripts/governance/check_system_dictionary_alignment.py --json` | `scripts/governance/check_system_dictionary_alignment.py` | 已覆盖 | 无 | 后续实现 OpenAPI / 前端 / 后端时扩展脚本检查生成物和代码引用。 |
| Web 设计 RTM | US-M2-002 / US-M4-001 | `python3 scripts/governance/check_web_design_rtm.py --json` | `docs/*-web-design-plan.md` | 已覆盖 | 无 | 设计方案新增 RTM 类型时同步 `check_web_design_rtm.py`。 |
| 项目级 RTM | US-H3-001 / US-H3-002 | `python3 scripts/governance/check_project_rtm.py --json` | 本文 | 已覆盖 | 无 | 任何 `部分覆盖` 行必须补缺口说明和补齐路径。 |
| 入库/库存后端闭环 | US-M2-002 / US-M2-003 / US-M2-005 / US-M3-004 | `cargo test -p wms-api --test wave3_postgres` | `backend/crates/api/tests/wave3_postgres.rs` | 已覆盖 | 无 | 后端行为变更时补跑对应 Postgres 测试。 |
| 出库后端闭环 | US-M4-001 / US-M4-003 / US-M4-004 / US-M4-006 | `cargo test -p wms-api --test wave4_postgres` | `backend/crates/api/tests/wave4_postgres.rs` | 已覆盖 | 无 | 前端接入真实 API 后补 UI 层动作测试。 |
| 审计不可篡改 | US-H2-001 / US-H2-002 | `cargo test -p wms-api --test audit_postgres` | `backend/crates/api/tests/audit_postgres.rs` | 已覆盖 | 无 | 保持 append-only 和 hash chain seal 测试。 |
| M-CG 单据号生成 | US-CG-001 / US-CG-002 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --test document_numbering_postgres`；`pnpm --dir apps/web-admin exec tsc --noEmit`；`pnpm --dir prototypes exec playwright test --config=playwright-web-admin-mcg-real-config.ts`；`just openapi-check` | `backend/crates/api/tests/document_numbering_postgres.rs`；`backend/crates/api/src/lib.rs`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts`；`apps/web-admin/self-checks/mcg-document-numbering-slice-self-check.mjs`；`prototypes/e2e/web-admin-mcg-real.spec.ts` | 部分覆盖 | 后端/API、M1 字典绑定、PC 前端、真实数据 E2E 和截图已有证据；配置审批、审计展示、PDA、M2/M3/M4 创建单据接入和正式版外部证据仍未完成。 | 后续补配置审批、审计展示、业务接入及正式版发布证据；新增 API 时继续保持 OpenAPI/api-client 同步。 |
| H4 企业微信参数本地测试 | US-H4-002 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --test h4_wechat_notify_postgres`；`pnpm --dir apps/web-admin run test:e2e:h4-dev`；`just openapi-check` | `backend/crates/api/tests/h4_wechat_notify_postgres.rs`；`prototypes/e2e/web-admin-h4-dev.spec.ts`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | 部分覆盖 | 仅覆盖参数完整性、启用状态和前端错误展示的本地测试子切片，不声明企业微信外部网络连通。 | 真实企业微信凭据与外网联调证据归外部集成门禁。 |
| H6 状态机定义注册与转换校验 | US-H6-001 | `cargo test --manifest-path backend/Cargo.toml -p wms-api state_machine`；`cargo test --manifest-path backend/Cargo.toml -p wms-api --test h6_state_machine_postgres`；`just openapi-check`；`python3 scripts/governance/check_quality_matrix.py --json`；`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H6 --json` | `backend/crates/api/src/state_machine.rs`；`backend/crates/api/src/openapi_tests.rs`；`backend/crates/api/tests/h6_state_machine_postgres.rs`；`backend/migrations/202607100001_h6_state_machine_permission.sql`；`backend/migrations/202607100003_system_admin_permission_sync.sql`；`governance/quality-matrix.toml`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts` | 已覆盖 | 无 | 当前仅证明三类定义经同一 H6 API 可读取和校验；状态转换执行能力启动时补 Postgres 事务测试、H2 审计测试和业务模块 E2E。 |
| H9 打印模板引擎 | US-H9-001 / US-H9-002 / US-H9-003 / US-H9-004 / US-H9-005 | `cargo test --manifest-path backend/Cargo.toml -p wms-api --test print_template_postgres`；`cargo test --manifest-path backend/Cargo.toml -p wms-api --test system_dictionary_postgres`；`just openapi-check`；`pnpm --filter @wms/web-admin build`；`python3 scripts/governance/check_admin_page_query_panel.py` | `backend/crates/api/tests/print_template_postgres.rs`；`backend/crates/api/tests/system_dictionary_postgres.rs`；`backend/crates/api/src/lib.rs`；`shared/openapi/openapi.json`；`packages/api-client/src/schema.ts`；`apps/web-admin/src/pages/print-template/H9PrintTemplatePage.tsx` | 部分覆盖 | 模板类型字典、字段库发布、版本不可改写、OpenAPI/api-client 同步和 PC 树状字段库列表页已有验证；前端动作测试、浏览器打印预览、9002 截图和专项截图证据未覆盖。 | H9 后续切片补动作测试、浏览器打印预览截图、hiprint 设计器和真实截图证据。 |

## 7. 合规风险 RTM

| 范围 | 需求来源 | 合规/风险来源 | 控制措施 | 证据对象 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|---|
| GSP 字段级追溯 | US-M1-001 / US-M2-003 / US-M4-006 / US-TC-001 | [compliance/gsp-field-traceability.md](compliance/gsp-field-traceability.md) | 字段名、故事字段表、合规字段状态三向核对 | `check_gsp_field_traceability.py` | 已覆盖 | 无 | 字段变更先改故事字段表和合规字段矩阵。 |
| GSP 条款级追溯 | US-H2-001 / US-M2-003 / US-M4-006 / US-M5-001 | [compliance/README.md](compliance/README.md) | 条款到故事、测试、审计证据映射 | `docs/compliance/*.md` | 部分覆盖 | 软件可控条款已覆盖；剩余 🟡 是冷链外部系统协作和特殊药品后续波次实施边界。 | 按 `docs/compliance/README.md` §4 和对应章节把外部协作 evidence 归档到 Wave6 证据链。 |
| 审计追加不可篡改 | US-H2-001 / US-H2-002 / US-H2-003 | 审计表只能 INSERT | DB trigger + hash chain seal | `backend/migrations/202606020001_audit_event.sql`；`audit_postgres.rs` | 已覆盖 | 无 | 禁止把审计修正改成 UPDATE / DELETE。 |
| 写操作幂等 | US-M2-002 / US-M4-001 / US-M4-006 | 重复提交、弱网重试、PDA 离线补传 | `Idempotency-Key` + `idempotency_request` | `wave3_postgres.rs`；`wave4_postgres.rs` | 已覆盖 | 无 | 新增写接口必须补 L11 幂等测试。 |
| 系统字典受控变更 | US-M1-011 | 字典项会影响流程、批号策略、货主覆盖和历史单据解释 | 受控字典走 M-QL 审批、H2 审计、版本生效时间、影响预览和 H2-005 缓存刷新 | `check_system_dictionary_alignment.py`；`system_dictionary_postgres.rs`；后续审批 / 事件测试 | 部分覆盖 | 首批后端已覆盖基础审计、参数校验和 fail closed；M-QL 审批、H2-005 事件、影响预览和真实前端证据仍待补。 | 后续补 M-QL 审批、H2-005 事件、导入导出、M2/M4 字典运行时校验和真实前端截图。 |
| 状态图不可变与非法跳转拦截 | US-H6-001 | 业务单据状态越权跳转、状态语义漂移、生产状态图被临时修改 | H6 状态机定义代码内不可变；转换校验 API 拒绝未登记状态和非法边；修改状态图必须走代码、OpenAPI 和矩阵审查 | `backend/crates/api/src/state_machine.rs`；`docs/domain/user-stories-h6-state-machine.md`；`governance/quality-matrix.toml` | 已覆盖 | 无 | 后续状态执行切片补 H2 审计、事务原子性和领域事件发布证据。 |
| 打印模板版本和字段库治理 | US-H9-001 / US-H9-002 / US-H9-003 / US-H9-004 / US-H9-005 | GSP 纸质留存、历史打印可追溯、敏感字段脱敏、模板版本不可改写 | H9 字段库发布版本、模板版本、打印记录、货主覆盖和 H2 审计追踪 | `backend/crates/api/tests/print_template_postgres.rs`；`backend/migrations/202607050002_h9_print_template.sql`；`backend/crates/api/src/lib.rs`；`apps/web-admin/src/pages/print-template/H9PrintTemplatePage.tsx` | 部分覆盖 | 字段库发布版本不可改写、幂等、审计、读取契约和 PC 字段库列表已有验证；模板主数据、打印记录、权限细分、真实打印证据和截图仍未覆盖。 | H9 后续切片补模板版本不可改写测试、敏感字段脱敏测试、跨货主权限测试、打印记录证据和真实截图。 |
| 仓储层 SQL 货主隔离 | US-H1-001 / US-H1-002 | 多货主 / 3PL 数据隔离 | 仓储层租户表 SQL 必须写入或过滤 `owner_id` | `check_owner_scope_sql.py`；repository 测试与代码 review | 已覆盖 | 无 | 维持 T1 owner scope SQL 静态扫描；非 repository 生产 SQL 必须迁入仓储层或扩展门禁范围。 |
| 冷链温控异常 | US-M5-001 / US-M5-002 / US-M5-003 | 温湿度越界、批次隔离、外部设备证据 | 冷链事件接入 + 库存隔离 + 审计 | `wave4_postgres.rs`；`wave5_postgres.rs` | 部分覆盖 | 仓库内温控异常处置和审计已覆盖；真实采集设备、TMS 或冷链平台证据仍未由本仓库生成。 | 按 `docs/runbooks/wave-5-hardware-evidence.md` 和 `docs/runbooks/wave-5-tms-evidence.md` 采集真实设备/外部系统 evidence。 |

## 8. Review 规则

| 变更类型 | 必须检查 |
|---|---|
| 新增用户故事 | §3 故事总 RTM 增加一行；相关维度矩阵至少一行引用该故事 |
| 新增前端页面或按钮 | 前端体验 RTM 和对应 `*-web-design-plan.md` 字段/动作/状态/证据 RTM |
| 新增后端 API | 后端实现 RTM 记录 API、handler/service、domain/repository/migration、测试 |
| 新增合规或风险控制 | 合规风险 RTM 记录来源、控制措施和证据 |
| 新增治理脚本 | 接入 `governance_checks.py`、`gate-rules.toml`、smoke 测试和本文测试证据 RTM |

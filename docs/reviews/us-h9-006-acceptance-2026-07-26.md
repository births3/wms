# US-H9-006 验收记录

- 故事：`US-H9-006 随货同行单归集与截单`
- 验收基线：2026-07-26 当前工作区（线路冻结、截单计划、原子归集、管理端和真实 E2E）
- 验收层级：`S3`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用，本故事不执行物理打印
- 验收日期：2026-07-26
- 整体结论：`PASS`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 订单进入即冻结唯一线路 | V1 / V2 / V3 | 真实 PostgreSQL 创建订单，断言地址有效期唯一、冻结快照不随后续线路变化；M4 页面从真实客户地址创建订单 | `outbound_order_creation_freezes_the_effective_address_route`；M4 受控仓库/客户/地址选择 | PASS | - |
| AC-2 周计划、例外日期和授权人工截单 | V1 / V2 / V3 | 创建并发布结构化周计划；生产进程每分钟执行计划截单且跳过未配置计划的边界；页面执行有原因的人工截单 | `print_orchestration_job.rs`；`scheduled_cutoff_uses_exception_time_and_concurrent_runs_create_one_group`；`cutoff-plans.png`；`cutoff-result.png` | PASS | - |
| AC-3 客户→线路→货主加仓库优先级与同级防重叠 | V1 / V2 / V3 | 真实 PostgreSQL 发布三层计划并解析；同级同对象重叠返回受控冲突；页面创建并发布线路层计划 | `cutoff_plan_publish_rejects_same_level_overlap_and_resolves_customer_first`；`cutoff-plans.png` | PASS | - |
| AC-4 同事务冻结订单集合并由 M-CG no-gap 发号 | V2 / V3 | 并发截单只创建一个组；事务中分配 `print_document_category:delivery_note`，失败不消耗编号；后续订单不回写原组 | H9 PostgreSQL 8 个测试；`cutoff-result.png` | PASS | - |
| AC-5 货主+仓库+地址硬边界，可跨 ERP 订单组 | V1 / V2 / V3 | 混合地址截单受控拒绝且不发号；同边界多订单归入同一组；页面展示真实 WMS/ERP 单号、客户、中文地址和冻结线路 | `manual_cutoff_rejects_mixed_address_before_allocating_a_number`；`pending-orders.png` | PASS | - |
| AC-6 H2 审计与幂等重放 | V1 / V2 | 路线、计划发布和截单重复请求返回原资源；截单、计划和线路动作查询 H2 追加审计 | `manual_cutoff_freezes_one_boundary_numbers_audits_and_replays`；`route_binding_publish_is_idempotent_and_rejects_address_time_overlap` | PASS | - |
| L4 错误路径 | V1 / V2 | 无有效线路、非已确认订单、跨边界、同级重叠、无规则和缺权限均返回受控错误 | `h9_print_orchestration_postgres.rs` | PASS | - |
| L6 并发 | V2 | 两个自动截单执行器竞争同一时点，只生成一个组和一个编号 | `scheduled_cutoff_uses_exception_time_and_concurrent_runs_create_one_group` | PASS | - |
| L8 权限与货主隔离 | V1 / V2 / V3 | 读写权限分离；无维护权限 HTTP 返回 403；查询和写入均绑定当前货主 | `manual_cutoff_http_requires_orchestration_write_permission`；菜单权限迁移 | PASS | - |
| MENU-VISIBILITY | V3 | 关闭 dev-mock，经真实登录和“基础能力 → H9 打印能力 → 作业·随货同行单归集”进入页面；新浏览器上下文重新登录后再次进入并回读已发布线路 | 真实 Playwright spec；四张页面截图 | PASS | - |
| UI-SEMANTICS | V3 | 中文字段和状态；仓库、客户、地址使用受控选择；周计划使用日期/时间控件；人工截单使用确认弹窗 | `web-admin-h9-delivery-note-aggregation-real.spec.ts` | PASS | - |
| BUSINESS-CONTENT | V2 / V3 | V2/V3 复用 `OUT-H9-E2E-006`，API、页面和截图同时展示 ERP 单号、客户、中文地址、冻结线路、组号和截单原因 | 四张真实截图；E2E 字段断言 | PASS | - |

## 聚合验证

- V0：`node apps/web-admin/self-checks/h9-delivery-note-aggregation-self-check.mjs`
- V1 / V2：`CARGO_INCREMENTAL=0 cargo test --manifest-path backend/Cargo.toml -p wms-api --test h9_print_orchestration_postgres -- --test-threads=1`（8/8）
- V2 种子业务键：`OUT-H9-E2E-006`、`ERP-H9-E2E-006`、`LINE-H9-E2E-006`
- V3：一次性 PostgreSQL + `WMS_WEB_ADMIN_DEV_MOCK=0`，执行
  `pnpm --dir prototypes exec playwright test --config=playwright-web-admin-m1-real-config.ts e2e/web-admin-h9-delivery-note-aggregation-real.spec.ts`（1/1）
- OpenAPI：`just openapi-sync`；`just openapi-check`
- 前端：`pnpm --dir apps/web-admin run build`
- 截图：
  - `artifacts/screenshot-portal/real-web/h9-delivery-note-aggregation/pending-orders.png`
  - `artifacts/screenshot-portal/real-web/h9-delivery-note-aggregation/cutoff-result.png`
  - `artifacts/screenshot-portal/real-web/h9-delivery-note-aggregation/cutoff-plans.png`
  - `artifacts/screenshot-portal/real-web/h9-delivery-note-aggregation/plans-and-routes.png`

## 验收结论

- 已证明：六条 AC、L1-L11、真实 PostgreSQL、并发 no-gap、H2 审计、权限/幂等、真实菜单、中文业务内容和四张截图闭环。
- 未完成：无 US-H9-006 软件缺口。
- 范围声明：本记录不代表 H9 套打中心全部完成；US-H9-007～015 继续逐故事验收，真实 Windows Agent、打印机和纸盒证据不得由本故事截图抵扣。

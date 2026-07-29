# US-H9-007 验收记录

- 故事：`US-H9-007 归集维度规则配置`
- 验收基线：2026-07-27 当前工作区（受控字段目录、不可变规则版本、样本测试、发布/停用与截单快照）
- 验收层级：`S3`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用，本故事不执行物理打印
- 验收日期：2026-07-27
- 整体结论：`PASS`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 仅可选已登记标准字段、等值归组、维度有序 | V1 / V2 / V3 | 维度目录表 `h9_aggregation_field_catalog` 受控播种；创建草稿校验字段全部已登记；页面维度只能从目录下拉添加并上移/下移排序 | `202607260005_h9_aggregation_rules.sql`；`ensure_registered_dimensions`；`AggregationRuleDialog` | PASS | - |
| AC-2 禁止自由 SQL、脚本、正则和任意字段路径 | V0 / V1 | `AggregationFieldCode` 枚举在类型层面排除未登记字段；域模型仅有 `equals` 归组方式；self-check 断言弹窗无自由表达式输入口 | `wms-domain print_orchestration.rs`；`h9-delivery-note-aggregation-self-check.mjs` | PASS | - |
| AC-3 草稿/测试/发布版本；已引用发布版本不可改写 | V1 / V2 / V3 | 版本状态机 draft→tested→published→disabled；单发布版本唯一索引；发布/停用后内容改写被数据库触发器拒绝；归集组外键 RESTRICT 引用规则版本 | `rule_lifecycle_replays_audits_and_rejects_rewrite`（触发器 `h9_aggregation_rule_content_immutable`）；页面状态徽章与按钮门控 | PASS | - |
| AC-4 发布前样本订单展示命中规则、分组键和预计归集结果 | V2 / V3 | 真实样本订单测试返回可解释分组键（中文字段名=值）与分组订单；页面样本测试弹窗展示 2 组按发票号拆分结果 | `published_rule_tests_real_orders_and_freezes_cutoff_snapshot`；`rule-test-preview.png`（INV-H9-E2E-007 / INV-H9-E2E-008 两组） | PASS | - |
| AC-5 地址隔离为不可覆盖系统约束 | V1 / V2 / V3 | 分组键在仓库+送货地址硬边界内计算；跨规则分组截单返回 `AggregationRuleMismatch`；页面与弹窗展示硬边界不可覆盖文案 | `manual_cutoff` 混组拒绝断言；测试分组按 warehouse/address 先分组；页面固定提示文案 | PASS | - |
| AC-6 发布/停用/测试幂等重放并写 H2 审计；实例保存规则版本快照 | V1 / V2 | 同幂等键重放返回原版本；create/test/publish/disable 四动作各写一条 H2 审计；截单实例冻结 `aggregation_rule_snapshot`/`aggregation_group_key`，计划截单按发布规则键拆分 | `rule_lifecycle_replays_audits_and_rejects_rewrite`；`scheduled_cutoff_splits_one_address_by_published_rule_key`；快照列 SQL 断言 | PASS | - |
| L4 错误路径 | V1 / V2 | 草稿直接发布、重复测试非活动版本等返回 `AggregationRuleInvalidState`；版本不存在返回 NotFound；样本订单缺失返回 OrderNotFound | `h9_aggregation_rule_postgres.rs`（4/4） | PASS | - |
| L8 权限与货主隔离 | V1 / V3 | 端点要求 `h9.print_orchestration.read/write`；版本查询与全部动作绑定当前货主；菜单按钮权限 `create_rule/test_rule/publish_rule/disable_rule` 迁移登记 | handler 权限断言；`202607260005` 按钮权限迁移；页面 `canWrite` 门控 | PASS | - |
| UI-SEMANTICS | V3 | 中文维度名、状态徽章（草稿/已测试/已发布/已停用）、受控下拉与顺序调整、确认弹窗 | `web-admin-h9-delivery-note-aggregation-real.spec.ts` US-H9-007 用例 | PASS | - |
| BUSINESS-CONTENT | V2 / V3 | 样本订单使用种子真实业务键 `OUT-H9-E2E-007/008`、发票号 `INV-H9-E2E-007/008`，API、页面与截图展示同一分组键 | `rule-test-preview.png`；`rule-published.png`；E2E 字段断言 | PASS | - |

## 聚合验证

- V0：`node apps/web-admin/self-checks/h9-delivery-note-aggregation-self-check.mjs`（含 US-H9-007 规则断言）
- V1 / V2：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test h9_aggregation_rule_postgres`（4/4，含生产出库写入全部可配置归集字段）；
  `--test h9_print_orchestration_postgres` 回归不受影响
- V2 种子业务键：`OUT-H9-E2E-007`、`OUT-H9-E2E-008`、`INV-H9-E2E-007`、`INV-H9-E2E-008`（与 006 同仓库/客户/地址/线路边界）
- V3：一次性 PostgreSQL + `WMS_WEB_ADMIN_DEV_MOCK=0`，
  `just web-admin-m1-real-e2e`（12/12，含 US-H9-006 回归与 US-H9-007 新用例）
- OpenAPI：五个 aggregation 端点登记 `openapi_paths/print_orchestration.rs` + `openapi_doc.rs` + `openapi_tests.rs` 必含清单；`just openapi-sync` 已再生成 api-client
- 前端：`pnpm --dir apps/web-admin run build`
- 截图：
  - `artifacts/screenshot-portal/real-web/h9-aggregation-rules/rule-test-preview.png`
  - `artifacts/screenshot-portal/real-web/h9-aggregation-rules/rule-published.png`

## 验收结论

- 已证明：六条 AC、受控字段目录、版本不可改写、幂等/审计、规则快照冻结、计划截单按规则键拆分、真实菜单入口与中文业务内容截图闭环。
- 未完成：无 US-H9-007 软件缺口。
- 范围声明：本记录不代表 H9 套打中心全部完成；US-H9-010、012～015
  继续逐故事验收，US-H9-011 的 S4 硬件证据继续独立收口，真实 Windows Agent、
  打印机和纸盒证据不得由本故事截图抵扣。

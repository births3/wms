# US-H9-008 验收记录

- 故事：`US-H9-008 打印组套配置与就绪策略`
- 验收基线：2026-07-28 当前工作区（US-H9-009 已补齐 H-FILE 与分类 PDF）
- 验收层级：`S3`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用，本故事不执行物理打印
- 验收日期：2026-07-27
- H-FILE 缺口复审日期：2026-07-28
- 整体结论：`PASS`（缺口如实登记于下表与"缺口清单"节）

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 匹配顺序送货地址、客户、线路、货主+仓库默认；同级有效期重叠禁止发布 | V1 / V2 / V3 | 四层 scope CHECK + 解析 SQL 按层级降序取一；发布前同级同对象重叠 EXISTS 校验 + advisory lock；测试弹窗展示样本组解析层级 | `suite_resolution_prefers_address_customer_route_then_default`（逐层停用逐层命中 4/4）；`same_level_same_object_overlap_rejected_on_publish`；E2E `解析层级：客户（命中本版本）` | PASS | - |
| AC-2 草稿/测试/发布版本；已发布版本不可改写 | V1 / V2 / V3 | 状态机 draft→tested→published→disabled；`h9_print_suite_content_immutable` 与 `h9_print_suite_items_immutable` 触发器拒绝改写发布内容 | `suite_lifecycle_replays_audits_and_rejects_rewrite`（版本与打印项 UPDATE 均被触发器拒绝）；页面状态徽章与按钮门控 | PASS | - |
| AC-3 打印项维护分类、份数、顺序、逻辑输出槽、必需、就绪/失败策略；rendered 绑定模板版本，external_file 绑定已摄取文件 | V0 / V1 / V2 / V3 | 域校验（顺序 1..N 连续、份数 1..20、必需项禁 skip）；表 CHECK 按 source_mode 区分绑定必填；rendered 绑定校验模板版本已发布 | `validate_print_suite`；`binding_and_category_validation_is_mode_specific`（6 类拒绝 + DB CHECK 兜底）；新建组套弹窗逐项字段 | PASS | - |
| AC-4 分类使用 M1 字典 `print_document_category`，含编码/中文名/source_mode；首批随货同行单(rendered)、药检单/发票(external_file) | V1 / V2 / V3 | 迁移向既有受控字典补 `drug_inspection_report`/`invoice` 全局项（`params.source_mode=external_file`）；草稿创建校验分类已登记且 source_mode 匹配；`GET /print-document-categories` 供页面选择 | `202607270002_h9_print_suites.sql`；`binding_and_category_validation_is_mode_specific`；弹窗分类下拉（渲染/外部文件） | PASS | - |
| AC-5 发票完整=全部源订单被有效发票覆盖；药检完整=全部必需商品+批号被有效报告覆盖 | V1 / V2 / V3 | 服务端可计算就绪检查：按组订单发票号与 `outbound_order_lines` 商品+批号，对照 `h9_document_file_bindings` + `attachments` 的 ready 权威文件逐项判定并输出缺口原因 | `required_not_ready_applies_frozen_policy_and_completeness`（缺文件→缺口原因；H-FILE 写入→就绪）；E2E `发票（外部文件，必需）：就绪，绑定 1 个权威文件` | PASS | - |
| AC-6 引用 H-FILE 文件 ID/稳定来源；禁止临时外部 URL 当长期事实源 | V0 / V1 | 配置层 `external_file_ref` 受控校验：必须 `h-file:` 前缀、拒绝 http(s) URL；实例层保存权威文件 ID+版本+内容哈希，不落 URL | `validate_print_suite`；`binding_and_category_validation_is_mode_specific`；US-H9-009 `FileAttachmentService` 与下载代理 | PASS | - |
| AC-7 必需未就绪可配置仅挂起当前实例或暂停 Agent 队列；策略固化到实例 | V1 / V2 | 就绪策略 `wait_hold_instance|pause_agent_queue` 字段化到组套项并固化到实例项；实例 `hold_scope=instance|agent_queue`；实例项策略列由触发器保持不可改写 | `required_not_ready_applies_frozen_policy_and_completeness`（两种策略分别断言）；`instance_snapshot_and_policies_are_frozen`（策略改写被拒） | PASS | Agent 队列的实际暂停执行属 US-H9-010/012 运行时，本故事完成策略固化与标记 |
| AC-8 实例保存组套版本、规则版本、源单据快照；rendered 项存模板版本、external_file 项存权威文件 ID 与版本 | V1 / V2 / V3 | 截单事务内按解析结果创建实例：`suite_snapshot`/`aggregation_rule_version_*`/`source_documents` 固化，实例项存 `template_version_id` 或 `file_bindings`；无发布组套时只生成归集组 | `instance_snapshot_and_policies_are_frozen`；`suite_resolution_...`；E2E 实例先显示 `等待分类 PDF`，US-H9-009 准备成功后转 `待打印` | PASS | - |
| AC-9 测试、发布、停用支持幂等重放，记录版本/范围/操作者与 H2 审计 | V1 / V2 | 全部写端点要求 `Idempotency-Key`，同键同请求重放返回原结果；create/test/publish/disable 各写一条 H2 审计（含 diff）；实例创建写 `create_print_suite_instance` 审计 | `suite_lifecycle_replays_audits_and_rejects_rewrite`（replayed=true + 审计 4/4）；`store_idempotency_success`/`replay_idempotency` 复用 006 共享幂等基座 | PASS | - |
| L4 错误路径 | V1 | 草稿直接发布/重复停用→`H9_PRINT_SUITE_STATE_INVALID`；版本不存在→404；样本组不存在→`H9_DELIVERY_NOTE_GROUP_NOT_FOUND`；分类/绑定非法→422 | `h9_print_suite_postgres.rs`（6/6）；handler 错误映射 | PASS | - |
| L8 权限与货主隔离 | V1 / V3 | 端点沿用 `h9.print_orchestration.read/write`；全部查询与动作绑定当前货主；菜单按钮权限 `create_suite/test_suite/publish_suite/disable_suite` 迁移登记 | handler 权限断言；`202607270002` 按钮权限迁移；页面 `canWrite` 门控 | PASS | - |
| UI-SEMANTICS / BUSINESS-CONTENT | V3 | 中文分类名、层级、策略、状态徽章；真实业务键 `SHTX-E2E-H9-008-0001`、`OUT-H9-E2E-009/010`、`INV-H9-E2E-009/010`、`PROD-H9-E2E/BATCH-H9-E2E`、`HFILE-INV-E2E-009/010` | E2E US-H9-008 用例与三张截图 | PASS | - |

## 聚合验证

- V0：`node apps/web-admin/self-checks/h9-delivery-note-aggregation-self-check.mjs`（含 US-H9-008 组套断言块）→ ok
- V1 / V2：`cargo test -p wms-api --test h9_print_suite_postgres` → **9 passed; 0 failed**；
  回归 `--test h9_print_orchestration_postgres`（8/8）、`--test h9_aggregation_rule_postgres`（4/4）不受截单事务新增实例逻辑影响
- OpenAPI：七个组套端点（categories、versions GET/POST、test、publish、disable、suite-instances）登记
  `openapi_paths/print_orchestration.rs` + `openapi_paths.rs` 再导出 + `openapi_doc.rs`（paths+schemas）+
  `openapi_tests.rs` 必含清单；`just openapi-sync` 已再生成 api-client
- 前端：`pnpm --dir apps/web-admin exec tsc --noEmit` 通过；打印组套页签 + 新建/测试弹窗 + 组套实例网格
- V3：一次性 PostgreSQL `wms_test_h8a` + `WMS_WEB_ADMIN_DEV_MOCK=0`，
  `just web-admin-m1-real-e2e`（12/12，含 US-H9-006/007/011 回归与 US-H9-008 新用例）
- 截图：
  - `artifacts/screenshot-portal/real-web/h9-print-suites/suite-test-readiness.png`（样本组预检：解析层级 + 逐项就绪 + 权威文件绑定）
  - `artifacts/screenshot-portal/real-web/h9-print-suites/suite-published.png`
  - `artifacts/screenshot-portal/real-web/h9-print-suites/suite-instance.png`（截单后固化实例 V1 待打印）

## 后续故事边界

1. **就绪策略的运行时执行**：`pause_agent_queue` 在本故事固化为实例 `hold_scope='agent_queue'`
   标记；真正暂停 Agent 队列/放行后续实例的调度行为属 US-H9-010/012 队列与 Agent 运行时。
2. **实例后续状态机**：US-H9-009 已实现 `waiting_documents → queued` 的 PDF 守卫；
   `preparing/running/...` 与任务队列属 US-H9-010。
3. **组套实例创建入口**：按设计仅由截单事务自动创建；PDF 失败在同一实例和幂等键重试，
   不另建实例。

## 验收结论

- 已证明：九条 AC、四层解析优先级、同级重叠拒绝、发布不可改写（版本+打印项双触发器）、
  rendered/external_file 受控绑定、发票/药检完整性可计算判定、两种就绪策略固化、实例三重
  快照不可改写、幂等重放与 H2 审计、真实菜单入口与中文业务内容截图闭环。
- 范围声明：本记录不代表 H9 套打中心全部完成；US-H9-010、012～015 继续逐故事验收，
  真实 Windows Agent、打印机和纸盒证据不得由本故事截图抵扣。

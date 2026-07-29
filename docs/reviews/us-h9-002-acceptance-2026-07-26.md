# US-H9-002 字段库生成与元数据维护验收记录

- 故事：`US-H9-002`
- 验收基线：US-H9-002 本地分组提交（后端、管理端与验收治理提交）
- 验收层级：`S2`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用
- 验收日期：`2026-07-26`
- 整体结论：`PASS`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 从 H3 OpenAPI schema 生成草稿并记录字段来源 | V0 / V1 / V2 / V3 | 使用当前 `ApiDoc::openapi()` 生成 `CreateReceivingOrderRequest` 字段库，断言根字段、`lines[].product_code` 明细字段和 `allOf` 引用字段 | PostgreSQL 测试；真实 E2E spec；草稿截图 | PASS | - |
| AC-2 完整维护字段元数据 | V1 / V2 / V3 | 页面编辑后从真实 API 回读显示名称、分组、说明、示例、打印/敏感、脱敏/格式化、条码/二维码、明细标识和排序号 | PostgreSQL 测试；真实 E2E API 回读；草稿截图 | PASS | - |
| AC-3 草稿与发布状态；模板仅绑定发布版本 | V1 / V2 / V3 | 草稿绑定模板返回 `H9_FIELD_LIBRARY_NOT_PUBLISHED`；发布后页面状态及最新发布版本一致 | `print_field_library_postgres.rs`；发布截图 | PASS | - |
| AC-4 发布版本不可改写；字段变更新建版本并审计 | V1 / V2 | 更新已发布字段返回不可改写；第二次生成得到 v2 草稿且 v1 业务模块快照不变；查询 H2 审计动作与 before/after diff | `openapi_draft_metadata_publish_versioning_and_audit_are_closed` | PASS | - |
| AC-5 发布前校验当前 OpenAPI 字段路径 | V1 / V2 | 向草稿插入已移除路径，仓储层与 HTTP 契约均返回 `H9_FIELD_PATH_INVALID`，版本保持草稿 | PostgreSQL 测试 | PASS | - |
| L4 非法元数据格式规则 | V1 / V2 | 提交 `uppercase()`，仓储层拒绝且 HTTP 返回 `422 H9_FIELD_FORMAT_INVALID` | PostgreSQL 测试 | PASS | - |
| L8 维护和发布权限 | V2 / V3 | 系统管理员完成生成、维护和发布；仓库主管无管理按钮且 POST 返回 403；仅有发布权时不能生成或编辑草稿 | 权限迁移；PostgreSQL HTTP 测试；真实 E2E spec | PASS | - |
| L11 生成、更新和发布幂等 | V2 | 同一幂等键重放返回同一草稿、字段或发布版本，不重复写审计 | PostgreSQL 测试 | PASS | - |
| MENU-VISIBILITY 管理端真实菜单链路 | V3 | 关闭 dev-mock，重新登录后经“基础能力 → H9 打印能力 → H9 打印模板”进入页面 | 真实 E2E spec；两张页面截图 | PASS | - |
| UI-SEMANTICS 中文文案与控件语义 | V3 | 断言字段库、字段元数据、状态和动作均为中文；布尔元数据使用复选框，版本使用单选下拉 | 真实 E2E spec；两张页面截图 | PASS | - |

## 聚合验证

- V0：`python3 scripts/governance/check_quality_matrix.py --json`
- V0：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H9 --json`
- V0：`just openapi-check`
- V1 / V2：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test print_field_library_postgres --test print_template_postgres -- --test-threads=1`
- V1：`node apps/web-admin/self-checks/h9-print-template-tree-self-check.mjs`
- V1：`pnpm --dir apps/web-admin run build`
- V3：`just web-admin-m1-real-e2e`（创建并回收一次性 PostgreSQL；包含 `US-H9-002` 真实 Playwright）
- V3 截图：`artifacts/screenshot-portal/real-web/h9-print-templates/field-library-draft-metadata.png`
- V3 截图：`artifacts/screenshot-portal/real-web/h9-print-templates/field-library-published.png`
- V4：不适用；本故事不依赖 PDA、外部系统、打印机硬件或发布环境。

## 验收结论

- 已证明：五条 AC、L4/L5/L8/L11、真实 PostgreSQL 读写、当前 OpenAPI、关闭 dev-mock 的真实菜单和页面截图均已闭环。
- 未完成：无 US-H9-002 故事内缺口。
- 范围声明：本记录不代表 H9 模块完成；US-H9-003～015 继续逐故事验收。

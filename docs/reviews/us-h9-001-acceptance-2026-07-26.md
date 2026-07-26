# US-H9-001 打印模板类型字典验收记录

- 故事：`US-H9-001`
- 验收基线：US-H9-001 本地分组提交（实现提交与验收证据提交）
- 验收层级：`S2`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用
- 验收日期：`2026-07-26`
- 整体结论：`PASS`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 `print_template_type` 不替代 `document_type` | V0 / V2 / V3 | PostgreSQL 迁移与真实字典页面分别查询两个分类 | `202607060001_h9_print_template_type_dictionary.sql`；真实 E2E spec | PASS | - |
| AC-2 九类字段完整 | V0 / V2 / V3 | OpenAPI 同步、真实 API 排序断言、页面中文字段与受控控件断言 | `SystemDictionaryItem` / `UpsertSystemDictionaryItemRequest`；两张页面截图 | PASS | - |
| AC-3 六个预置类型 | V2 / V3 | PostgreSQL 测试断言编码与 `10..60` 顺序；浏览器断言六个编码 | `system_dictionary_postgres.rs`；真实 E2E spec | PASS | - |
| AC-4 启用类型必须绑定字段库 | V1 / V2 / V3 | 缺失和空白字段库均拒绝；HTTP 返回 `422 H9_FIELD_LIBRARY_REQUIRED` | PostgreSQL 测试；真实 E2E API 断言 | PASS | - |
| AC-5 全局默认与货主覆盖 | V2 / V3 | 创建货主覆盖、更新后回读、停用后不回退全局 | PostgreSQL 测试；`print-template-type-owner-override.png` | PASS | - |
| AC-6 创建、修改、停用写入 H2 审计 | V2 | 真实 PostgreSQL 查询同一资源的两条 upsert 审计和一条 disable 审计，并核对 diff | `print_template_type_create_update_disable_are_idempotent_and_audited` | PASS | - |
| L8 系统管理员维护、仓库主管只读 | V2 / V3 | 系统管理员完成全流程；仓库主管页面无维护动作且 PUT 返回 403 | 迁移权限；真实 E2E spec | PASS | - |
| L11 重复创建幂等 | V2 | 同一幂等键重复创建返回同一资源且只写一次创建审计 | PostgreSQL 测试 | PASS | - |

## 聚合验证

- V0：`python3 scripts/governance/check_quality_matrix.py --json`
- V0：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H9 --json`
- V1 / V2：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test system_dictionary_postgres --test print_template_postgres -- --test-threads=1`
- V1：`node apps/web-admin/self-checks/h9-print-template-tree-self-check.mjs`
- V1：`pnpm --dir apps/web-admin run build`
- V3：`just web-admin-m1-real-e2e`（创建并回收一次性 PostgreSQL；包含 `US-H9-001` 真实 Playwright）
- V3 截图：`artifacts/screenshot-portal/real-web/m1-system-dictionary/print-template-type-owner-override.png`
- V3 截图：`artifacts/screenshot-portal/real-web/h9-print-templates/template-type-tree.png`
- V4：不适用；本故事不依赖 PDA、外部系统、打印机硬件或发布环境。

## 验收结论

- 已证明：六条 AC、L4/L5/L8/L11、真实数据库、关闭 dev-mock 的两个前端页面及截图均已闭环。
- 未完成：无 US-H9-001 故事内缺口。
- 范围声明：本记录不代表 H9 模块完成；US-H9-002～015 继续逐故事验收。

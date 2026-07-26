# US-H9-003 模板设计与版本管理验收记录

- 故事：`US-H9-003`
- 验收基线：US-H9-003 本地分组提交（后端、管理端、真实 E2E 与治理提交）
- 验收层级：`S2`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用
- 验收日期：`2026-07-26`
- 整体结论：`PASS`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 hiprint 设计器与 WMS 治理边界 | V0 / V1 / V2 / V3 | 管理端动态加载 hiprint；保存走 WMS API；后端拒绝可执行 formatter/styler 类选项 | ADR-0036；PostgreSQL 安全测试；真实 E2E spec | PASS | - |
| AC-2 模板主数据字段完整 | V0 / V2 / V3 | 保存并回读模板编码、名称、类型、作用域、货主、启停、默认标记和备注 | PostgreSQL 测试；真实 E2E API 回读 | PASS | - |
| AC-3 模板版本快照完整 | V1 / V2 / V3 | 保存并回读 hiprint JSON、字段绑定、纸张、设计器版本和版本号 | PostgreSQL 测试；草稿截图 | PASS | - |
| AC-4 绑定已发布字段库且旧版本不被升级改写 | V1 / V2 / V3 | 草稿字段库拒绝；模板类型与字段库不匹配拒绝；字段库升级后旧模板版本仍绑定原版本 | `print_template_postgres`；真实 E2E 契约断言 | PASS | - |
| AC-5 修改生成草稿，独立发布后才生效 | V1 / V2 / V3 | 修改得到 v2 草稿时解析仍返回 v1；发布 v2 后解析切换；旧草稿不能越过最新草稿发布 | 生命周期 PostgreSQL 测试；草稿/发布截图 | PASS | - |
| AC-6 禁止物理删除并支持停用审计 | V1 / V2 / V3 | 数据库外键拒绝删除已发布/已打印版本；API 不提供删除入口；停用后不可解析并写入 H2 diff | 生命周期 PostgreSQL 测试；停用截图 | PASS | - |
| L4 错误契约 | V1 / V2 | 断言非法 JSON、可执行模板选项、重复编码、字段绑定不匹配和非最新草稿分别返回受控错误 | PostgreSQL 仓储层与 HTTP 测试 | PASS | - |
| L8 保存、发布、停启和预览权限 | V2 / V3 | 写权限不能发布；发布权限不能保存/停启；仓库主管无维护按钮、直接发布返回 403，但可预览 | HTTP PostgreSQL 测试；真实 E2E spec | PASS | - |
| L11 保存、发布和停启幂等 | V2 | 同一幂等键重放返回同一版本/状态且不重复写审计 | 生命周期 PostgreSQL 测试 | PASS | - |
| MENU-VISIBILITY 管理端真实菜单链路 | V3 | 关闭 dev-mock，经真实登录和“基础能力 → H9 打印能力 → H9 打印模板”进入页面 | 真实 E2E spec；三张页面截图 | PASS | - |
| UI-SEMANTICS 中文文案与动作分离 | V3 | 断言保存新草稿、发布、版本历史、停用/启用和只读预览为独立中文动作 | 真实 E2E spec；三张页面截图 | PASS | - |
| DEPENDENCY-GOVERNANCE hiprint 工程复核 | V0 / V1 / V3 | 核对包许可证声明、上游活跃度、OSV 直接漏洞查询、浏览器声明和 Chromium 真实 E2E；后端阻断字符串函数执行 | ADR-0036；PostgreSQL 安全测试；真实 E2E | PASS | 不属于本故事 V4；正式发布门禁见 ADR-0036 |

## 聚合验证

- V0：`python3 scripts/governance/check_quality_matrix.py --json`
- V0：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H9 --json`
- V0：`just openapi-check`
- V1 / V2：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test print_field_library_postgres --test print_template_postgres -- --test-threads=1`
- V1：`node apps/web-admin/self-checks/h9-print-template-tree-self-check.mjs`
- V1：`pnpm --dir apps/web-admin run test:e2e:h9-dev`
- V1：`pnpm --dir apps/web-admin run build`
- V3：`just web-admin-m1-real-e2e`（创建并回收一次性 PostgreSQL；包含 `US-H9-003` 真实 Playwright）
- V3 截图：`artifacts/screenshot-portal/real-web/h9-print-templates/template-version-draft.png`
- V3 截图：`artifacts/screenshot-portal/real-web/h9-print-templates/template-version-published.png`
- V3 截图：`artifacts/screenshot-portal/real-web/h9-print-templates/template-disabled.png`
- V4：不适用；本故事不依赖 PDA、外部系统、打印机硬件或发布环境。

## 验收结论

- 已证明：六条 AC、L4/L5/L8/L11、真实 PostgreSQL、关闭 dev-mock 的真实菜单、模板版本动作和截图均已闭环。
- 未完成：无 US-H9-003 软件故事内缺口；正式发布依赖复核按 ADR-0036 单独收口，不以开发环境证据替代。
- 范围声明：本记录不代表 H9 模块完成；US-H9-004～015 继续逐故事验收。

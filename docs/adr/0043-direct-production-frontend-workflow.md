# ADR-0043：直接生产前端开发与开发 Mock 走查

- 状态：Accepted
- 决策日期：2026-07-26
- 决策人：项目主人
- 关联：ADR-0006、ADR-0021、ADR-0022、ADR-0028、ADR-0029

## 背景

项目已进入生产管理端持续开发阶段。继续为每个页面维护独立原型、原型矩阵和视觉 baseline，
会产生两套页面编排与重复走查成本。项目主人明确决定：整个项目不再制作原型图，业务确认直接
使用生产前端和开发 Mock 数据。

## 候选方案

1. 继续原型先行：隔离性强，但要维护 `prototypes/` 与 `apps/` 两套页面，重复成本最高。
2. 直接在生产前端开发，以开发 Mock 走查：只维护一套页面，仍可在真实 API 完成前确认流程。
3. 等后端全部完成后再做前端：不会有 Mock 偏差，但业务反馈过晚，返工范围更大。

## 决策

采用方案 2：

1. 新页面直接写入 `apps/web-admin` 或经批准的其他生产应用，不再新增原型页。
2. 业务走查通过生产应用的开发模式完成；Mock 必须由 Vite dev mock 或可替换的数据入口提供，
   页面不得使用裸 `fetch`。
3. 开发 Mock 只证明界面和交互，不得作为 API、后端、数据库、权限、审计、幂等或发布完成证据。
4. 新页面继续使用 `@wms/ui`、`QueryPanel`、`DataGrid`、真实菜单接线和 Playwright 截图证据。
5. 现有 `prototypes/`、原型矩阵和 baseline 作为历史资产保留，本决策不执行批量删除；不再把
   原型批准作为新故事恢复或生产开发的前置条件。

本 ADR 取代 ADR-0021、ADR-0022 中的原型专用流程，以及 ADR-0029 的原型先行工作流。
`@wms/ui` 分层和组件规范继续由 ADR-0028、`docs/frontend-coding-standards.md` 约束。

## 后果

- 正面：页面只实现一次，菜单、查询、Mock、E2E 和生产结构同步接受业务反馈。
- 负面：开发 Mock 与正式契约可能漂移，必须在质量矩阵中明确保持故事未完成。
- 风险：Mock 交互被误认为生产闭环。缓解方式是页面明确标识开发 Mock，正式 API 未接入前
  不把相关故事从 `deferred_stories` 移入完成矩阵。

## 实施约束

- 新页面必须直接登记生产菜单、页面查询分类、已发布菜单 dev mock、页面 self-check 和真实
  Playwright 截图。
- 开发 Mock 不得进入生产持久化语义；正式写入仍需 OpenAPI、权限、H2 审计和幂等测试。
- 前端验收截图必须由连接真实 HTTP 服务和 PostgreSQL 测试库的 `*-real.spec.ts` 在业务断言
  与刷新回读之后生成；开发 Mock、静态响应和 Playwright 业务路由拦截只用于走查。
- 不新增 `prototypes/src/pages/*`、原型 manifest 或原型 baseline。
- 历史原型清理属于独立、可回滚的治理任务，不在业务页面开发中顺手删除。

## 参考

- [ADR-0006：TDD + 11 层测试](0006-tdd-and-test-layers.md)
- [ADR-0028：组件库抽离](0028-component-library-extraction.md)
- [前端编码规范](../frontend-coding-standards.md)
- [质量矩阵方法](../governance/quality-matrix-method.md)

---
name: wms-quality-matrix-governance
description: WMS 全链路质量矩阵治理技能。用户要求建立或维护测试/质量矩阵、检查新增用户故事/页面/API/字段是否进入矩阵、按 S0-S3 分层策略补齐维度、根据 issue/Bug/review 漏检迭代检查维度、修复 check_quality_matrix 失败，或说“缺口闭环 模块/页面/issue”“补齐缺失功能”“验收模块是否完整”时使用；缺口闭环默认必须推动功能实现，不只是登记或延期。
---

# WMS Quality Matrix Governance

把 WMS 用户故事、页面、接口、字段、后端、数据库、权限、审计、测试、证据和治理脚本纳入同一张可检查矩阵。

## 先读

- `AGENTS.md`
- `docs/governance/quality-matrix-method.md`
- `.agents/skills/wms-loop-engineering/SKILL.md`
- `.agents/skills/wms-execution-retrospective/SKILL.md`
- `docs/agent-loop-engineering.md`
- `governance/quality-matrix.toml`
- 目标故事文件、页面文件、OpenAPI path 或后端模块

## 固定事实源

- 机器事实源：`governance/quality-matrix.toml`。
- 展示页：`docs/governance/quality-matrix.md`，只由脚本生成。
- 检查脚本：`scripts/governance/check_quality_matrix.py`。
- 范围缺口：`scripts/governance/check_scope_gap_discovery.py`；模块验收用 `--strict --module <模块>`。
- MkDocs 入口：`mkdocs.yml` 治理分组。

## 工作流

短触发语：`缺口闭环 <模块/页面/issue>`。等价于先严格发现范围缺口，再用 `wms-worktree-subagent` 补实现，随后用 `wms-review-fix-commit` 复审提交；若发现漏检，用 `wms-execution-retrospective` 反哺规则。

闭环分两类，未说明时默认选功能闭环：

- 功能闭环：发现未实现故事、页面、按钮、弹窗、API、后端或测试时，必须进入开发任务；只登记矩阵不算完成。
- 登记闭环：只有用户明确说“只登记 / 暂缓 / 延期 / 先入矩阵”时，才允许用 `deferred_stories` 收口。

0. 新增维度、漏检复盘、批量补齐或多轮修复时，先按 `wms-loop-engineering` 定义目标、检查和停止条件。
1. 先运行 `python3 scripts/governance/check_quality_matrix.py --json`，确认当前矩阵是否干净。
2. 根据任务判断是否需要新增或修改矩阵行：
   - 新增 / 修改用户故事。
   - 新增 / 修改管理端页面、PDA 页面、OpenAPI path、数据库字段、权限、审计或测试。
   - issue、Bug、review 暴露出前后端未对齐、字段遗漏、页面布局遗漏、脚本漏检。
3. 每个矩阵条目以用户故事 ID 为主键；首版强门禁范围为 M1/M2/M3/M4。
4. 每个故事必须写完整维度：`requirement`、`fields`、`frontend`、`api`、`backend`、`database`、`security`、`audit`、`tests`、`evidence`、`docs`、`governance`。
5. 维度状态只允许 `verified` 或 `not_applicable`；`not_applicable` 必须写原因，不能用来掩盖未完成。
6. 由 `types` 自动推导测试层；不要手工压低 L1-L11 覆盖要求。
7. 改完事实源后运行 `python3 scripts/governance/check_quality_matrix.py --write-doc` 生成展示页。
8. 跑范围缺口：日常 `python3 scripts/governance/check_scope_gap_discovery.py --json`；模块验收或用户问“是否补齐”时加 `--strict --module <模块>`。
9. 严格模式发现缺口时，先分类：
   - 已实现未登记：补矩阵并生成展示页。
   - 未实现：拆成最小开发任务，优先用 `wms-worktree-subagent` 实现，再补矩阵和证据。
   - 范围过大或必须业务确认：停止并列出待实现清单、建议拆分和风险，不能把延期登记当完成。
10. 写入 `deferred_stories` 前必须有本轮用户明确确认；没有确认时只能报告“待实现/待确认”，不能用延期关闭功能缺口。
11. 最终汇报必须分开写：已实现、已登记、待实现、经确认延期。缺少“已实现”证据时，不能说“闭环完成”。
12. 验证：`check_quality_matrix.py --json`、`check_scope_gap_discovery.py --json`、相关 pytest、接线改动跑 dispatch/smoke、最后 `just gov-t1`。

批量口径：用户说“推进 N 个故事”默认表示 N 个故事达到“故事完成”退出条件，并从 `deferred_stories` 迁入 `stories`；分类、登记、补一部分实现或记录阻塞都不能抵扣 N。执行中必须按“已完成 X/N”报告真实进度，外部证据阻塞的故事保留未完成状态，不得用另一个故事静默替换。

## 验收退出条件

- 故事完成：十二维度闭环，故事类型推导的测试层全部覆盖；写入故事必须有真实 PostgreSQL 测试，有页面必须有真实数据 E2E。
- 模块完成：运行 `check_quality_matrix.py --complete-module <模块>`，该模块仍有任一 `deferred_stories` 时禁止宣称完成。
- 发布完成：PDA、硬件、外部系统、性能和灰度发布必须使用真实环境证据，不能以 mock、localhost 或静态文件替代。
- 验收深度按故事类型自动取最高层级：S1 查询展示、S2 普通写入、S3 库存/并发/关键路径、S4 PDA/硬件/外部系统/发布；不得手工降级。

## 页面分类分区联动

新增管理端页面时，同时使用 `wms-page-query-governance`：

- 列表型：页面上方 `QueryPanel`，主体 `DataGrid`。
- 双栏目录型：左侧分类 / 目录 / 树，右侧明细列表或编辑区。
- 配置型：按配置域分区，展示影响范围、启停状态和审计入口。
- 详情弹窗型：按订单信息、商品信息、批号信息、流程信息、审计信息上下分区。

页面分类必须反映到矩阵的 `frontend`、`fields`、`evidence` 和 `governance` 维度。

## 标准线与自我发现

质量标准线分两级：

- 硬失败：已进入质量矩阵的故事、页面或 API 与真实菜单 / 文档 / 接线不一致。
- 发现型缺口：同一活跃模块中还有未登记故事、未覆盖菜单页、用户故事要求的按钮 / 弹窗 / 流程未进入矩阵。

发现型缺口不能在最终汇报中消失；“补齐/闭环/验收/所有模块”任务必须升级为严格模式，并按功能闭环处理。

前端页面闭环的最小证据：

- `frontend_pages` 声明的页面必须同时出现在 `apps/web-admin/src/App.tsx` 的 `menuSections`、`defaultMenuTree`、`renderAdminView` 可达路由和 `apps/web-admin/dev-mocks/admin-menu-dev-mock.ts` 已发布菜单种子中。
- 带 `frontend_interaction` 且声明了 `frontend_pages` 的故事，必须在质量矩阵写 `e2e_checks`；页面级 self-check 只证明静态接线，不能替代真实浏览器验收。
- `governance/menu-e2e-screenshot-policy.toml` 基线外的新增菜单页必须写真实 Playwright 命令和 `e2e_screenshots = [{ page, spec, screenshot }]`，同时把 spec 与 `artifacts/screenshot-portal/real-web/**/*.png` 产物路径放入 `evidence_refs`；运行 E2E 生成截图后才能标记页面闭环。
- `legacy_pages` 只允许覆盖规则启用前页面，不能因本轮实现扩大；补齐历史页面证据时从基线移除。
- 页面级 self-check 至少覆盖菜单入口、默认菜单树、已发布菜单 dev mock、路由渲染、公共查询 / 表格组件、真实后端或 dev mock 数据入口，防止“菜单有了但页面不可达 / Dev mock route not found / 没有可运行证据”。

## 运行反馈迭代

出现三页以上同类遗漏、前端有页面但后端/API/权限未跟上、issue/review 暴露脚本漏检、新业务概念影响多模块时，先补矩阵或脚本，再批量修实现。顺序：定义目标 → 补事实源 → 补脚本 → 跑失败 → 修实现 → 生成展示页 → 复跑治理 → 记录漏检规则。

## 自进化

当 `check_quality_matrix.py`、页面自检、真实后端、dev mock、E2E、issue 评论或 review 暴露出“矩阵通过但运行仍失败”时，必须触发 `wms-execution-retrospective`，并把漏检转成下次可复用的检查规则。

最小闭环：归因到矩阵、脚本、页面/API 自检、真实后端、dev mock、证据或 skill 规则；先修当前事故；规则不足就改本 `SKILL.md`；可脚本化就补脚本/self-check，不可脚本化就写入方法文档或 runbook。复跑 `check_quality_matrix.py --json`、`check_scope_gap_discovery.py --json`、新增检查、`git diff --check`、`just gov-t1`。

前端页面登记为 `frontend=verified` 时，还要确认页面数据入口可运行：页面调用的 API 必须在真实后端路由或 `apps/web-admin/vite.config.ts` dev mock 路由中至少一边可达；dev mock 模式下要有 self-check 或 `curl 9002` 证据，防止 `Dev mock route not found`。

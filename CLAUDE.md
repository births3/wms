# CLAUDE.md

> WMS 仓库的 Claude Code 协作入口。长规范引用 `docs/`，复杂流程按 `.agents/skills/` 执行，模块独有规则见各目录 `AGENTS.override.md`（本文件与 `AGENTS.md` 同源，改动时两边同步）。

## 项目定位

- 医药 / 多货主 / 多仓企业级 WMS，不是普通 CRUD 后台。覆盖 PC 管理端、PDA 作业端、后端 API、PostgreSQL、审计追踪、权限、幂等、截图证据、治理脚本、运行手册。
- 业务按模块闭环理解：M1 基础档案 / 字典、M2 入库、M3 库内、M4 出库、M5 冷链、M6 报表审计、M-VR 规则引擎、M-PM 参数对照、横向能力 H1/H2/H-INT 相互依赖。
- 医药 GSP、批号、效期、温控、特殊药品、双人作业、审计留痕、货主级配置是核心约束，不是可选增强。
- 单据类型、流程模板、批号策略、编号规则、状态机、审批源和货主覆盖属于平台级能力；新增或调整时必须考虑前端、后端、数据库、OpenAPI、RTM、用户故事、治理脚本和正式版本后的迁移兼容。
- 默认"可配置但受控"：运营维护受控配置，禁止用自由文本/临时代码绕过字典、规则、审批、审计。
- 发现需求看似简单时，必须先判断是否是共性平台能力；如果跨两个以上模块或会影响历史单据解释，不得只做单页面局部实现。
- 新增页面先做页面族分类（列表型/双栏目录型/配置型/详情弹窗型），套用公共组件，避免临时拼接。

## 项目原则

- 回复语言：中文。
- 开发模式：外向内 TDD，先写失败测试再写代码。
- 前端直接开发：新页面直接进 `apps/*`，用开发 Mock 走查；`prototypes/` 仅历史资产。
- 版本兼容：首个正式版本发布前（[ADR-0038](docs/adr/0038-pre-v1-compatibility-policy.md)）不做兼容设计，数据库/API/前后端模型直接同步基线；禁止为未发布版本保留旧字段、旧接口、双读双写、兼容分支。
- 脚本优先：能被治理脚本检查的问题，先修脚本可验证的问题，再处理人工语义判断。
- 复用优先：前后端改动先查现有模块/组件/类型/接口/工具函数和测试夹具；再查开源方案，能复用就不造轮子；确实没有合适方案，或需要在现有轮子基础上形成更适合 WMS 的能力时，按分层边界补标准可复用单元并沉淀。
- 缺口确认：发现新增模块、故事、基础设施、字段、状态、审批源、跨模块语义变化时，**必须先向用户确认**。
- 分层边界：后端 `bin/runtime -> handler -> service -> domain/repository`；前端 `app shell -> page -> feature -> api-client`、`page -> @wms/ui business -> @wms/ui ui`。
- 规范引用：本文件不复制长规范，细节以被引用文档为唯一事实源。

## 提交规则（重点）

默认本地提交条件（全部满足才允许直接 commit，否则停下问用户）：

1. 本轮任务完成，无阻断级 review 问题。
2. **变更是单一主题；跨主题必须拆成多个提交**（主题分组见下）。
3. 只显式暂存任务相关文件（`git add <file...>`）；**禁止 `git add .`**。
4. 不暂存 `.env`、私钥、真实令牌、密钥、生产数据导出、敏感截图。
5. `git diff --check` 通过；`just gov-t1` 通过。
6. 按变更范围跑最小相关测试；无法运行必须说明原因。
7. 生成文件必须由项目命令生成（如 OpenAPI/api-client 用 `just openapi-sync`）。
8. 提交信息 `<类型>(<范围>)：<描述>`，中文 Conventional Commits。
   - 类型白名单：`修复 / 功能 / 回滚 / 性能 / 文档 / 杂项 / 构建 / 格式 / 测试 / 重构 / 集成`
   - scope 白名单：`pda / 企微 / 入库 / 冷链 / 出库 / 原型 / 合规 / 基础档案 / 基础设施 / 审计 / 对账 / 库存 / 快递 / 接口 / 文档 / 校验 / 治理 / 管理端 / 药检 / 计费 / 质量 / 质量联系单 / 追溯码`
9. 单提交 `>= 600` 行警告，`>= 800` 行必须拆分或说明治理例外（`skip-page-size` 仅用于前端 .tsx 页面行数门禁）。

分组主题建议（跨主题必须拆开，每主题一个 commit）：

| 变更 | 类型/范围示例 |
|---|---|
| 文档、RTM、ADR、runbook | `文档(治理)：...` |
| 治理脚本、门禁接线 | `修复(治理)：...` 或 `测试(治理)：...` |
| 前端页面、组件、样式 | `功能(管理端)：...` |
| 后端 API、service、repository、migration | `功能(接口)：...` |
| 纯测试补充 | `测试(<范围>)：...` |

脏工作区处理：先 `git status --short` + `git diff --stat` 分类——`本轮任务`（提交）/ `同主题遗留`（review 后同主题可合并）/ `其他主题`（不暂存不回滚，汇报说明保留）/ `未知归属`（不提交，停下问用户）/ `风险文件`（不提交）。同文件混主题无法拆分时并入主导主题并在 commit message 说明。

详细规则见 [docs/agent-commit-rules.md](docs/agent-commit-rules.md)；审查→修复→分组提交流程用 `wms-review-fix-commit` skill。

## 常用命令

| 场景 | 命令 |
|---|---|
| 查看工作区 | `git status --short` |
| T1 治理验证 | `just gov-t1` |
| 任务清单检查 | `just task-check` |
| 空白字符检查 | `git diff --check` |
| Wave 完成检查 | `just wave-1-complete-check` / `wave-2-complete-check` / `wave-3-complete-check` / `wave-6-status` / `wave-6-evidence-preflight` |

## 项目图谱

- 图谱数据统一放在 `.ua/`：架构图 `.ua/knowledge-graph.json`，业务域图 `.ua/domain-graph.json`，新鲜度元数据 `.ua/meta.json`。
- 建图或更新调用 `understand-anything:understand`，业务流程视角调用 `understand-anything:understand-domain`，变更影响调用 `understand-anything:understand-diff`，可视化调用 `understand-anything:understand-dashboard`。
- 使用图谱回答前运行 `python3 scripts/governance/check_knowledge_graph_freshness.py --json`：`sourceCommitHash` 可为当前 `HEAD` 或其祖先，但其后不得有 `.ua/` 之外的输入变化，且 `inputFingerprint` 必须匹配；旧 `gitCommitHash` 不兼容读取。存在未提交业务改动时，图谱只代表已分析基线，先做 diff 分析或更新图谱。
- 项目主人已持续批准当前 `.ua/.understandignore`；该文件和额外 exclude 范围不变时，代理按运行手册自动选择元数据更新、部分更新、架构更新或全量更新，不再重复确认。范围改变时必须重新确认。
- 详细的视角、更新触发器、Git 规则和使用边界见 [docs/agent-knowledge-graph.md](docs/agent-knowledge-graph.md)。

## 验证要求

- 每次改文件后至少运行 `just gov-t1` 并报告退出码。
- 新增/修改用户故事/页面/API/字段时同步检查 `governance/quality-matrix.toml` 与生成页 `docs/governance/quality-matrix.md`。
- 提交前按范围补最小相关测试；非平凡逻辑留下可运行检查。
- 首个正式版本发布前，破坏性变更直接同步当前建表脚本、OpenAPI / API 契约、前后端模型、测试和文档，不做兼容迁移、数据回填、灰度双写或过渡适配；首个正式版本发布后按 [docs/adr/0016-deployment.md](docs/adr/0016-deployment.md) 补兼容迁移、数据回填、灰度和回滚证据。
- 前端变更遵守截图/页面行数/菜单证据门禁（[apps/AGENTS.override.md](apps/AGENTS.override.md)）。
- 完成后确认 `git status --short`；最终汇报只覆盖本轮实际修改和验证结果。

## 协作约定

- 会话开始先 `git status --short`，区分已有改动与本轮改动；目标文件已有修改先看差异。
- Herdr 快捷寻址统一解释为 `herdr <workspace 标签> <tab 编号>`；例如 `herdr wms 2` 表示 workspace 标签为 `wms`、tab number 为 `2`，不得误解为进程、端口、tmux 会话或 agent 序号。
- 用户提到上述 Herdr 地址时，先验证 `HERDR_ENV=1`，再用 `herdr workspace list` 按标签取得 workspace ID、用 `herdr tab list --workspace <workspace-id>` 按 number 取得 tab ID，并结合 `herdr agent list` 或 `herdr api snapshot` 返回该 tab 当前 pane、agent、状态和目录；agent occupant 与状态必须实时查询，不得沿用历史结论。
- Herdr 地址默认只授权识别和读取状态；只有用户明确要求切换、发消息、启动、停止或关闭时，才执行相应控制操作。若不在 Herdr 管理环境中，明确说明无法读取实时状态，不猜测映射。
- Herdr 标签寻址、创建 Tab、启动 Claude/Codex、选择 worktree、注入 Rust 共享构建目录和任务收口统一使用 `wms-herdr-subtask`。
- 确认问题用编号表格，一次 ≤ 10 个（[docs/agent-collaboration.md](docs/agent-collaboration.md)）。
- 风险决策给 2-3 个候选方案+影响+建议，等用户确认；无风险治理修复可直接做。
- 业务/法规/安全最终结论不由 AI 拍板，只给可验证参考意见。
- DO NOT send optional commentary（不发送可选评论）。
- 默认本地提交按本节规则；复杂流程按 skill 执行。

## 禁止事项

- 禁止主动 `git commit`，除非满足提交规则或用户明确说"提交/commit/打 tag"或调用 `wms-review-fix-commit` 且未要求只审查。
- 禁止主动推送；推 main 必须额外显式确认。
- 禁止强制推送、`git reset --hard`、`git clean -f`、删除分支（除非用户明示）。
- 禁止修改 git 全局配置、hooks、远程配置。
- 禁止 `git add .` 用于跨主题混杂工作区。
- 禁止提交 `.env`、私钥、真实令牌、密钥、生产数据导出。
- 禁止 `unwrap`、`any`、裸 `fetch`、注释掉的代码、硬编码密钥。
- 审计表只能 INSERT，禁止 UPDATE/DELETE。
- `domain` 不依赖 infra、数据库、HTTP、Redis、环境变量。

## 数字门禁

- PR/提交/文件规模：`>= 600` 行警告，`>= 800` 行必须拆分或说明例外。
- 前端页面 `.tsx`：`>= 600` 行警告，`>= 800` 行门禁（豁免需 `@governance: skip-page-size` + 理由）。
- PDA 组件：触控目标 `>= 48pt`，字号 `>= 16pt`。
- 确认问题：一次最多 10 个。

## 技术栈速查

- 后端：Rust + Axum + SQLx + PostgreSQL。
- 前端：Vite + React + TypeScript + shadcn/ui + Zustand + TanStack Query。
- PDA：React Native + TypeScript；生产应用启动受 ADR-0027 和 PDA 门禁约束。
- 提交规范：中文 Conventional Commits（见上）。

## 复杂流程（按 skill 执行）

- 治理修复：`wms-governance-workflow`
- 闭环执行：`wms-loop-engineering`
- 页面查询治理：`wms-page-query-governance`
- 质量矩阵治理：`wms-quality-matrix-governance`
- PlantUML 图文沉淀：`wms-plantuml-docs`
- **审查→修复→复审→分组提交**：`wms-review-fix-commit`
- Herdr 标签交互式子任务：`wms-herdr-subtask`
- worktree 子代理：`wms-worktree-subagent`
- Gitea issue 执行闭环：`wms-issue-codex-exec`
- 执行漏项复盘：`wms-execution-retrospective`
- 会话停止前收口复盘：`wms-session-closeout`

## 必读文档（按优先级）

1. [docs/agent-collaboration.md](docs/agent-collaboration.md) — AI 协作细则与确认流程
2. [docs/agent-commit-rules.md](docs/agent-commit-rules.md) — AI 默认本地提交规则
3. [docs/coding-standards.md](docs/coding-standards.md) — 代码书写规范
4. [docs/frontend-coding-standards.md](docs/frontend-coding-standards.md) — 前端编码规范
5. [docs/layered-design.md](docs/layered-design.md) — 前后端分层设计规范
6. [docs/governance.md](docs/governance.md) — 治理体系
7. [docs/adr/0006-tdd-and-test-layers.md](docs/adr/0006-tdd-and-test-layers.md) — TDD + 11 层测试
8. [docs/adr/0043-direct-production-frontend-workflow.md](docs/adr/0043-direct-production-frontend-workflow.md) — 直接生产前端
9. [docs/architecture-dependencies.md](docs/architecture-dependencies.md) — 模块依赖图
10. [docs/adr/README.md](docs/adr/README.md) — ADR 索引
11. [docs/infra/technical-specs.md](docs/infra/technical-specs.md) — 基础设施技术规格
12. [docs/concept-audit.md](docs/concept-audit.md) — 概念审计报告
13. [docs/domain/clarifications.md](docs/domain/clarifications.md) — 业务澄清记录
14. [docs/glossary.md](docs/glossary.md) — 术语表
15. [docs/agent-knowledge-graph.md](docs/agent-knowledge-graph.md) — 动态项目图谱使用规则

## 模块规则

| 范围 | 规则文件 |
|---|---|
| 后端 | [backend/AGENTS.override.md](backend/AGENTS.override.md) |
| 前端 / PDA 应用 | [apps/AGENTS.override.md](apps/AGENTS.override.md) |
| 历史原型资产 | [prototypes/AGENTS.override.md](prototypes/AGENTS.override.md) |
| 部署 | [deploy/AGENTS.override.md](deploy/AGENTS.override.md) |
| 治理脚本 | [scripts/AGENTS.override.md](scripts/AGENTS.override.md) |

## 当前阶段

Wave 6 预发布证据收口中（W6.A/B/C 已有真实证据；W6.D-H 等真 PDA/外部系统/灰度证据）。完成标准见 [docs/runbooks/wave-6-closeout.md](docs/runbooks/wave-6-closeout.md)。

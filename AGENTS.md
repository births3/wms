# AGENTS.md

> WMS 仓库的 AI 协作入口。这里只放生成内容前必须知道的项目原则、常用命令、验证要求、协作约定和禁止事项；长规范放 `docs/`，复杂流程放 `.agents/skills/`，模块独有规则放子目录 `AGENTS.override.md`。

## 文件分工

| 位置 | 放什么 | 当前状态 |
|---|---|---|
| `AGENTS.md` | 项目原则、常用命令、验证要求、协作约定、禁止事项 | 本文件 |
| `*/AGENTS.override.md` | 模块独有规则，例如后端、前端应用、原型、部署、治理脚本 | 见下方模块规则 |
| `.agents/skills/<skill>/SKILL.md` | 复杂流程，例如需求确认、治理修复、页面查询治理、质量矩阵治理、审计流程、PlantUML 图文沉淀、worktree 子代理流程、Gitea issue 执行闭环、会话停止前收口复盘 | 见 `.agents/skills/wms-governance-workflow/SKILL.md`、`.agents/skills/wms-loop-engineering/SKILL.md`、`.agents/skills/wms-page-query-governance/SKILL.md`、`.agents/skills/wms-quality-matrix-governance/SKILL.md`、`.agents/skills/wms-review-fix-commit/SKILL.md`、`.agents/skills/wms-plantuml-docs/SKILL.md`、`.agents/skills/wms-worktree-subagent/SKILL.md`、`.agents/skills/wms-issue-codex-exec/SKILL.md` 与 `.agents/skills/wms-session-closeout/SKILL.md` |
| `.codex/config.toml` | 模型、沙箱、MCP、钩子、审批默认值 | 本仓库不跟踪；`.gitignore` 标记为本机配置 |
| `.codex/rules/*.rules` | 命令允许 / 提示 / 禁止规则 | 本仓库不跟踪；`.gitignore` 标记为本机配置 |
| `docs/*.md` | 详细说明、经验手册、长规范、背景材料 | 见 [docs/agent-collaboration.md](docs/agent-collaboration.md) 与 [docs/agent-document-index.md](docs/agent-document-index.md) |

## 项目定位

- 本项目是面向医药 / 多货主 / 多仓运营的大型企业级 WMS，不是普通 CRUD 后台、小型库存工具或原型演示系统。
- 设计默认要覆盖 PC 管理端、PDA 作业端、后端 API、PostgreSQL 数据模型、审计追踪、权限、幂等、截图证据、治理脚本和运行手册。
- 业务必须按模块闭环理解：M1 基础档案 / 系统字典、M2 入库、M3 库内、M4 出库、M5 冷链、M6 报表审计、M-VR 规则引擎、M-PM 参数对照、横向能力 H1/H2/H-INT 等相互依赖。
- 医药 GSP、批号、效期、温控、特殊药品、双人作业、审计留痕和货主级配置是核心约束，不能当作可选增强。
- 单据类型、流程模板、批号策略、编号规则、状态机、审批源和货主覆盖属于平台级能力；新增或调整时必须考虑前端、后端、数据库、OpenAPI、RTM、用户故事、治理脚本和正式版本后的迁移兼容。
- 默认按“可配置但受控”的企业系统设计：运营可维护受控配置，不能用自由文本或临时代码绕过字典、规则、审批和审计。
- 发现需求看似简单时，必须先判断是否是共性平台能力；如果跨两个以上模块或会影响历史单据解释，不得只做单页面局部实现。
- 新增页面必须先做页面族分类和信息分区：列表型、双栏目录型、配置型、详情弹窗型分别套用对应公共组件和展示结构，避免临时拼接页面。

## 项目原则

- 回复语言：中文。
- 开发模式：外向内 TDD，先写失败测试再写代码。
- 版本兼容：首个正式版本发布前按 [ADR-0038](docs/adr/0038-pre-v1-compatibility-policy.md) 不做兼容设计；数据库、API、前后端模型直接同步到当前基线，禁止为未发布版本保留旧字段、旧接口、双读双写、兼容分支或过渡适配层。
- 脚本优先：能被治理脚本检查的问题，先修脚本可验证的问题，再处理人工语义判断。
- 复用优先：前后端改动先查现有模块、组件、类型、接口、工具函数和测试夹具；再查 GitHub、成熟开源方案或已安装依赖，能复用就不造轮子；确实没有合适方案，或需要在现有轮子基础上形成更适合 WMS 的能力时，再按分层边界补标准可复用单元，并沉淀为后续可复用的组件、工具函数、服务或治理脚本。
- 缺口确认：发现新增模块、故事、基础设施、字段、状态、审批源、跨模块语义变化时，必须先向用户确认。
- 分层边界：后端 `bin/runtime -> handler -> service -> domain/repository`；前端 `app shell -> page -> feature -> api-client` 与 `page -> @wms/ui business -> @wms/ui ui`。这些英文为代码分层名。
- 规范引用：本文件不复制长规范，细节以被引用文档为唯一事实源。

## 常用命令

| 场景 | 命令 |
|---|---|
| 查看工作区 | `git status --short` |
| T1 治理验证 | `just gov-t1` |
| 任务清单检查 | `just task-check` |
| 空白字符检查 | `git diff --check` |
| Wave 1 完成检查 | `just wave-1-complete-check` |
| Wave 2 完成检查 | `just wave-2-complete-check` |
| Wave 3 完成检查 | `just wave-3-complete-check` |
| Wave 6 状态 | `just wave-6-status` |
| Wave 6 预检 | `just wave-6-evidence-preflight` |

## 验证要求

- 每次改文件后，至少运行 `just gov-t1` 并报告退出码。
- 新增或修改用户故事 / 页面 / API / 字段时，必须同步检查 `governance/quality-matrix.toml` 和生成页 `docs/governance/quality-matrix.md`。
- 提交前按变更范围补充最小相关测试；非平凡逻辑必须留下可运行检查。
- 首个正式版本发布前，破坏性变更直接同步当前建表脚本、OpenAPI / API 契约、前后端模型、测试和文档，不做兼容迁移、数据回填、灰度双写或过渡适配；首个正式版本发布后按 [docs/adr/0016-deployment.md](docs/adr/0016-deployment.md) 补兼容迁移、数据回填、灰度和回滚证据。
- 前端/原型变更还要遵守截图、视觉基线、页面行数等门禁，详见 [apps/AGENTS.override.md](apps/AGENTS.override.md) 与 [prototypes/AGENTS.override.md](prototypes/AGENTS.override.md)。
- 完成后确认 `git status --short`；最终汇报只覆盖本轮实际修改和验证结果。

## 协作约定

- 每次会话开始先看 `git status --short`，区分已有改动和本轮改动。
- 脏工作区中只改任务相关文件；目标文件已有修改时先看差异。
- 向用户提确认问题时使用编号表格，问题不超过 10 个，详见 [docs/agent-collaboration.md](docs/agent-collaboration.md)。
- 有风险决策给 2-3 个候选方案、影响和建议，等待用户确认；无风险治理修复可直接做。
- 业务/法规/安全最终结论不由 AI 拍板；AI 只给可验证参考意见。
- DO NOT send optional commentary。
- 默认本地提交按 [docs/agent-commit-rules.md](docs/agent-commit-rules.md)；满足条件时不再额外询问是否提交。
- 复杂流程按对应 `.agents/skills/*/SKILL.md` 执行；治理修复用 `wms-governance-workflow`，闭环执行用 `wms-loop-engineering`，页面查询治理用 `wms-page-query-governance`，质量矩阵治理用 `wms-quality-matrix-governance`，PlantUML 图文沉淀用 `wms-plantuml-docs`，审查修复后分组提交用 `wms-review-fix-commit`，worktree 子代理执行用 `wms-worktree-subagent`，Gitea issue 自动处理用 `wms-issue-codex-exec`，执行漏项复盘迭代用 `wms-execution-retrospective`，会话停止前收口复盘用 `wms-session-closeout`。

## 禁止事项

- 禁止主动 `git commit`，除非满足 [docs/agent-commit-rules.md](docs/agent-commit-rules.md)，或用户明确说“提交/commit/打 tag”，或明确调用 `wms-review-fix-commit` / “review 技能”且未要求只审查不提交。
- 禁止主动推送；推 main 分支必须额外显式确认。
- 禁止强制推送、`git reset --hard`、`git clean -f`、删除分支，除非用户明示。
- 禁止修改 git 全局配置、hooks、远程配置。
- 禁止 `git add .` 用于跨主题混杂工作区；必须显式暂存文件。
- 禁止提交 `.env`、私钥、真实令牌、真实密钥、生产数据导出。
- 禁止 `unwrap`、`any`、裸 `fetch`、注释掉的代码、硬编码密钥。
- 审计表只能 INSERT，禁止 UPDATE/DELETE。
- `domain` 不依赖 infra、数据库、HTTP、Redis、环境变量。

## 数字门禁

- PR / 提交 / 文件规模：`>= 600` 行警告，`>= 800` 行必须拆分或说明例外。
- 前端页面 `.tsx`：`>= 600` 行警告，`>= 800` 行门禁；豁免需 `@governance: skip-page-size` 和理由。
- PDA 组件：触控目标 `>= 48pt`，字号 `>= 16pt`。
- 确认问题：一次最多 10 个。

## 技术栈速查

- 后端：Rust + Axum + SQLx + PostgreSQL。
- 前端：Vite + React + TypeScript + shadcn/ui + Zustand + TanStack Query。
- PDA：React Native + TypeScript；生产应用启动受 ADR-0027 和 PDA 门禁约束。
- 提交规范：`<类型>(<范围>)：<描述>`，中文 Conventional Commits，详见 [docs/governance.md](docs/governance.md#32-conventional-commits)。

## 必读文档（按优先级）

1. [docs/agent-collaboration.md](docs/agent-collaboration.md) — AI 协作细则与确认流程
2. [docs/agent-commit-rules.md](docs/agent-commit-rules.md) — AI 默认本地提交规则
3. [docs/coding-standards.md](docs/coding-standards.md) — 代码书写规范
4. [docs/frontend-coding-standards.md](docs/frontend-coding-standards.md) — 前端编码规范
5. [docs/layered-design.md](docs/layered-design.md) — 前后端分层设计规范
6. [docs/governance.md](docs/governance.md) — 治理体系
7. [docs/adr/0006-tdd-and-test-layers.md](docs/adr/0006-tdd-and-test-layers.md) — TDD + 11 层测试
8. [docs/adr/0029-frontend-as-prototype-workflow.md](docs/adr/0029-frontend-as-prototype-workflow.md) — 前端原型先行工作流
9. [docs/prototypes/prototype-to-production.md](docs/prototypes/prototype-to-production.md) — 原型转生产清单
10. [docs/prototypes/matrix-e2e-screenshot-gate.md](docs/prototypes/matrix-e2e-screenshot-gate.md) — Matrix E2E 截图门禁
11. [docs/architecture-dependencies.md](docs/architecture-dependencies.md) — 模块依赖图
12. [docs/adr/README.md](docs/adr/README.md) — ADR 索引
13. [docs/infra/technical-specs.md](docs/infra/technical-specs.md) — 基础设施技术规格
14. [docs/concept-audit.md](docs/concept-audit.md) — 概念审计报告
15. [docs/domain/clarifications.md](docs/domain/clarifications.md) — 业务澄清记录
16. [docs/glossary.md](docs/glossary.md) — 术语表

## 模块规则

| 范围 | 规则文件 |
|---|---|
| 后端 | [backend/AGENTS.override.md](backend/AGENTS.override.md) |
| 前端 / PDA 应用 | [apps/AGENTS.override.md](apps/AGENTS.override.md) |
| 原型 | [prototypes/AGENTS.override.md](prototypes/AGENTS.override.md) |
| 部署 | [deploy/AGENTS.override.md](deploy/AGENTS.override.md) |
| 治理脚本 | [scripts/AGENTS.override.md](scripts/AGENTS.override.md) |

## 当前阶段

Wave 6 预发布证据收口中。W6.A / W6.B / W6.C 已有真实证据并通过验证器；W6.D-H 仍等待真 PDA、外部系统、硬件和预发布灰度发布证据。完成标准见 [docs/runbooks/wave-6-closeout.md](docs/runbooks/wave-6-closeout.md)。

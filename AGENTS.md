# AGENTS.md

> WMS 仓库的 AI 协作入口。这里只放生成内容前必须知道的项目原则、常用命令、验证要求、协作约定和禁止事项；长规范放 `docs/`，复杂流程放 `.agents/skills/`，模块独有规则放子目录 `AGENTS.override.md`。

## 文件分工

| 位置 | 放什么 | 当前状态 |
|---|---|---|
| `AGENTS.md` | 项目原则、常用命令、验证要求、协作约定、禁止事项 | 本文件 |
| `*/AGENTS.override.md` | 模块独有规则，例如后端、前端应用、原型、部署、治理脚本 | 见下方模块规则 |
| `.agents/skills/<skill>/SKILL.md` | 复杂流程，例如需求确认、治理修复、审计流程、worktree 子代理流程 | 见 `.agents/skills/wms-governance-workflow/SKILL.md`、`.agents/skills/wms-loop-engineering/SKILL.md`、`.agents/skills/wms-review-fix-commit/SKILL.md` 与 `.agents/skills/wms-worktree-subagent/SKILL.md` |
| `.codex/config.toml` | 模型、沙箱、MCP、钩子、审批默认值 | 本仓库不跟踪；`.gitignore` 标记为本机配置 |
| `.codex/rules/*.rules` | 命令允许 / 提示 / 禁止规则 | 本仓库不跟踪；`.gitignore` 标记为本机配置 |
| `docs/*.md` | 详细说明、经验手册、长规范、背景材料 | 见 [docs/agent-collaboration.md](docs/agent-collaboration.md) 与 [docs/agent-document-index.md](docs/agent-document-index.md) |

## 项目原则

- 回复语言：中文。
- 开发模式：外向内 TDD，先写失败测试再写代码。
- 脚本优先：能被治理脚本检查的问题，先修脚本可验证的问题，再处理人工语义判断。
- 复用优先：前后端改动先查现有模块、组件、类型、接口、工具函数和测试夹具；没有现成能力时，再按分层边界补标准可复用单元。
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
- 提交前按变更范围补充最小相关测试；非平凡逻辑必须留下可运行检查。
- 正式版发布前，数据库结构、OpenAPI / API 契约、前后端数据模型等破坏性变更或新增必填字段，必须按 [docs/adr/0016-deployment.md](docs/adr/0016-deployment.md) 补兼容迁移、数据回填、灰度和回滚证据；开发阶段直接修改建表脚本或契约不能作为正式发布方案。
- 前端/原型变更还要遵守截图、视觉基线、页面行数等门禁，详见 [apps/AGENTS.override.md](apps/AGENTS.override.md) 与 [prototypes/AGENTS.override.md](prototypes/AGENTS.override.md)。
- 完成后确认 `git status --short`；最终汇报只覆盖本轮实际修改和验证结果。

## 协作约定

- 每次会话开始先看 `git status --short`，区分已有改动和本轮改动。
- 脏工作区中只改任务相关文件；目标文件已有修改时先看差异。
- 向用户提确认问题时使用编号表格，问题不超过 10 个，详见 [docs/agent-collaboration.md](docs/agent-collaboration.md)。
- 有风险决策给 2-3 个候选方案、影响和建议，等待用户确认；无风险治理修复可直接做。
- 业务/法规/安全最终结论不由 AI 拍板；AI 只给可验证参考意见。
- 复杂流程按对应 `.agents/skills/*/SKILL.md` 执行；治理修复用 `wms-governance-workflow`，闭环执行用 `wms-loop-engineering`，审查修复后分组提交用 `wms-review-fix-commit`，worktree 子代理执行用 `wms-worktree-subagent`。

## 禁止事项

- 禁止主动 `git commit`，除非用户明确说“提交/commit/打 tag”，或明确调用 `wms-review-fix-commit` / “review 技能”且未要求只审查不提交。
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
2. [docs/coding-standards.md](docs/coding-standards.md) — 代码书写规范
3. [docs/frontend-coding-standards.md](docs/frontend-coding-standards.md) — 前端编码规范
4. [docs/layered-design.md](docs/layered-design.md) — 前后端分层设计规范
5. [docs/governance.md](docs/governance.md) — 治理体系
6. [docs/adr/0006-tdd-and-test-layers.md](docs/adr/0006-tdd-and-test-layers.md) — TDD + 11 层测试
7. [docs/adr/0029-frontend-as-prototype-workflow.md](docs/adr/0029-frontend-as-prototype-workflow.md) — 前端原型先行工作流
8. [docs/prototypes/prototype-to-production.md](docs/prototypes/prototype-to-production.md) — 原型转生产清单
9. [docs/prototypes/matrix-e2e-screenshot-gate.md](docs/prototypes/matrix-e2e-screenshot-gate.md) — Matrix E2E 截图门禁
10. [docs/architecture-dependencies.md](docs/architecture-dependencies.md) — 模块依赖图
11. [docs/adr/README.md](docs/adr/README.md) — ADR 索引
12. [docs/infra/technical-specs.md](docs/infra/technical-specs.md) — 基础设施技术规格
13. [docs/concept-audit.md](docs/concept-audit.md) — 概念审计报告
14. [docs/domain/clarifications.md](docs/domain/clarifications.md) — 业务澄清记录
15. [docs/glossary.md](docs/glossary.md) — 术语表

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

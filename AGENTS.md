# AGENTS.md

> AI 编码助手协作指引。具体规范通过引用获取，本文件不重复内容。

## 本文件的书写规范

- **只写引用和速查约束**，不写具体实现细节
- 具体规范的唯一真相源是被引用的文档，本文件不复制内容
- 新增规范文档时，在"必读文档"段追加引用
- 约束变更时同步更新"核心约束"段
- 本文件修改必须随对应规范文档的 PR 一起提交
- 保持极简：AI 助手应在 30 秒内读完本文件，再按需深入引用文档

## 必读文档（按优先级）

1. [docs/coding-standards.md](docs/coding-standards.md) — 代码书写规范（Rust / TS / 跨端 / 禁止清单）
2. [docs/governance.md](docs/governance.md) — 治理体系（5 类 + 4 Tier + Baseline + 文档四层管理）
3. [docs/adr/0006-tdd-and-test-layers.md](docs/adr/0006-tdd-and-test-layers.md) — TDD + 11 层测试
4. [docs/architecture-dependencies.md](docs/architecture-dependencies.md) — 模块依赖图（11 业务 + 3 横向 + 5 波次）
5. [docs/adr/README.md](docs/adr/README.md) — 所有架构决策索引

## 其他文档索引

| 文档 | 用途 |
|------|------|
| [docs/adr/0001-tech-stack.md](docs/adr/0001-tech-stack.md) | 技术栈选型决策 |
| [docs/adr/0002-monorepo-structure.md](docs/adr/0002-monorepo-structure.md) | 仓库结构决策 |
| [docs/adr/0003-governance-model.md](docs/adr/0003-governance-model.md) | 治理模型决策 |
| [docs/adr/0004-phase-roadmap.md](docs/adr/0004-phase-roadmap.md) | 波次路线决策 |
| [docs/retros/wave-0-retro.md](docs/retros/wave-0-retro.md) | Wave 0 回顾 |
| [ROADMAP.md](ROADMAP.md) | 长期路线（波次状态） |
| [TODO.md](TODO.md) | 当前 Wave 任务 |
| [CHANGELOG.md](CHANGELOG.md) | 版本变更记录 |
| [governance/gate-rules.toml](governance/gate-rules.toml) | 门禁触发规则 |
| [governance/baselines/README.md](governance/baselines/README.md) | Baseline 机制说明 |

## 核心约束（速查）

- 开发模式：outside-in TDD（先写失败测试再写代码）
- 后端：Rust + Axum + SQLx + PostgreSQL
- 前端：Vite + React + TypeScript + shadcn/ui + Zustand + TanStack Query
- PDA：React Native + TypeScript
- 提交规范：Conventional Commits（`<type>(<scope>): <subject>`）
- 禁止：`unwrap` / `any` / 裸 fetch / 注释掉的代码 / 硬编码密钥
- 审计表只能 INSERT，禁止 UPDATE/DELETE
- domain 不依赖 infra

## 当前阶段

Wave 0 治理骨架已完成。下一步：Wave 1 横向底座（H1 权限 / H2 审计 / H3 OpenAPI）。

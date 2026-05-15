# AGENTS.md

> AI 编码助手协作指引。具体规范通过引用获取，本文件不重复内容。

## 必读文档（按优先级）

1. [docs/coding-standards.md](docs/coding-standards.md) — 代码书写规范（Rust / TS / 跨端 / 禁止清单）
2. [docs/governance.md](docs/governance.md) — 治理体系（5 类 + 4 Tier + Baseline）
3. [docs/adr/0006-tdd-and-test-layers.md](docs/adr/0006-tdd-and-test-layers.md) — TDD + 11 层测试
4. [docs/architecture-dependencies.md](docs/architecture-dependencies.md) — 模块依赖图
5. [docs/adr/README.md](docs/adr/README.md) — 所有架构决策索引

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

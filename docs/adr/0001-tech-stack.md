# ADR-0001: 技术栈选型

- 状态：Accepted
- 日期：2026-05-15
- 决策者：项目发起人
- 关联：`docs/governance.md`、`ROADMAP.md`
- 修订：2026-08-20 数据库大版本钉为 PostgreSQL 18（部署镜像 `postgres:18`；v1 前按 ADR-0038 直接改基线）

---

## 背景

wms 是一个**医药冷链 GSP 合规仓储管理系统**，需求特点：

- **数据正确性极高**：库存、批号、效期错一条都可能违反 GSP，不能"差不多就行"
- **并发可观**：多 PDA 现场作业、温湿度高频采集、监管平台对接
- **长期维护**：业务模块多（11 个），生命周期至少 5 年
- **三端协同**：PC 管理后台 + PAD 主管端 + PDA 仓库作业端
- **合规可追溯**：审计追踪不可篡改，监管接口规范严格
- **个人起步、未来可能扩团队**：选型既要能一个人推进，也要在团队进入时不返工

技术栈是项目的地基，本 ADR 一次性把整套栈固化下来，避免后期反复推翻。

---

## 候选方案

### 后端语言/框架

| 候选 | 优点 | 缺点 |
|------|------|------|
| Python + FastAPI | 开发快、生态广、AI 能力强 | 运行时类型风险、并发性能一般、长期维护弱 |
| Java + Spring Boot | 企业级成熟、人才多 | 启动慢、内存大、模板代码多、Rust 时代略显笨重 |
| Go + Gin/Echo | 简单、并发好 | 错误处理冗长、泛型晚熟、GSP 这种强领域建模不擅长 |
| **Rust + Axum** | 类型安全极强、性能极佳、async 生态成熟、编译期捕获大量 bug | 学习曲线陡、编译慢、招人难 |

### ORM / 数据库访问

| 候选 | 优点 | 缺点 |
|------|------|------|
| **SQLx** | 编译期校验 SQL、异步原生、轻量、SQL 控制力强 | 不是真 ORM，复杂关系查询要手写 |
| SeaORM | ActiveRecord 风格、迁移工具齐全 | 抽象偏重、SQL 不够直观 |
| Diesel | 类型最强 | 同步为主、async 体验差 |

### 数据库

| 候选 | 优点 | 缺点 |
|------|------|------|
| **PostgreSQL 18** | 类型丰富、JSON/JSONB、扩展生态（TimescaleDB、PostGIS）、事务可靠；当前部署基线 `postgres:18` | 运维成本中 |
| MySQL | 部署简单、人才多 | 数据类型/约束弱于 PG，复杂业务吃亏 |
| SQLite | 零运维 | 并发写差，不适合服务端长期 |

### 前端构建工具

| 候选 | 优点 | 缺点 |
|------|------|------|
| **Vite** | 极快、生态成熟、shadcn/ui 默认推荐 | 不内置 SSR（本项目无需） |
| Next.js | 全栈一等公民、SSR/SSG | 本项目后端是 Rust，Next 的全栈能力浪费 |
| Rsbuild | Rust 底层、极快 | 较新，生态/案例不足 |
| CRA | 历史选择 | 已停止维护 |

### UI 组件库

| 候选 | 优点 | 缺点 |
|------|------|------|
| Ant Design | 后台开箱即用 | 风格固定、定制成本高、bundle 偏大 |
| **shadcn/ui** | 复制即用、Tailwind 驱动、可深度定制、现代 | 需自己拼组合，初期慢一点 |
| MUI | Material 风格 | 主题改造工作量大 |

### 状态管理

| 候选 | 优点 | 缺点 |
|------|------|------|
| **Zustand** | 极轻、hook 风格、TS 友好 | 弱约束（团队大需自律） |
| Redux Toolkit | 强约束、DevTools 强 | 样板多、与 TanStack Query 功能重叠 |

### 数据请求

| 候选 | 选择理由 |
|------|---------|
| **TanStack Query** | 服务端状态事实标准；缓存、重试、失效、乐观更新一站搞定 |

### 三端定位

| 端 | 选择 | 理由 |
|----|------|------|
| PC 管理端 | Vite + React + TS + shadcn/ui | 主战场 |
| PAD 主管端 | 与 PC 共用代码 + Tailwind 响应式 | 复用最大化，避免双倍维护 |
| PDA 仓库作业端 | **React Native**（独立 app） | 仓库现场必须离线、需扫码硬件、需蓝牙打印；纯 Web/PWA 不够稳 |

### 接口契约

| 候选 | 选择理由 |
|------|---------|
| **utoipa（Rust）→ openapi.json → openapi-typescript（前端）** | 一处定义两端同步；后端改字段前端 tsc 立刻报错 |
| 手写两端类型 | 不可持续 |

### 配套基础

| 项 | 选择 |
|----|------|
| 异步运行时 | Tokio |
| 错误处理 | thiserror（库错误）+ anyhow（应用错误） |
| 日志/追踪 | tracing + tracing-subscriber |
| 包管理（前端） | pnpm |
| 路由（前端） | React Router |

---

## 决策

整套技术栈如下：

### 后端

- **语言/框架**：Rust + Axum
- **ORM**：SQLx
- **数据库**：PostgreSQL 18（部署镜像 `postgres:18`）
- **异步运行时**：Tokio
- **错误处理**：thiserror + anyhow
- **日志/追踪**：tracing + tracing-subscriber
- **API 文档**：utoipa 生成 OpenAPI

### 前端（PC + PAD 共用）

- **语言**：TypeScript（strict 模式）
- **构建**：Vite
- **UI 框架**：React
- **组件库**：shadcn/ui（基于 Radix + Tailwind CSS）
- **状态管理**：Zustand（客户端状态） + TanStack Query（服务端状态）
- **路由**：React Router
- **包管理**：pnpm
- **API 客户端**：从 OpenAPI 自动生成（openapi-typescript）

### PDA 端（独立 App）

- **框架**：React Native
- **语言**：TypeScript
- **共享**：与 Web 端共用 `packages/api-client`、`packages/domain-types`

### 跨端契约

- **唯一真相源**：后端 utoipa 生成的 `shared/openapi/openapi.json`
- **生成时机**：每次后端 handler 变更，CI 自动跑 `gen-api`，前端类型同步更新

---

## 后果

### 正面

- **类型安全贯穿全栈**：Rust 类型 → OpenAPI → TS 类型，错配在编译期暴露
- **性能与正确性兼顾**：Rust 适合 GSP 这种"错一条都不行"的场景
- **三端代码复用最大化**：PC + PAD 一套代码；PDA 与 Web 共享类型与 API 客户端
- **合规友好**：强类型 + 显式错误 + 不变量约束，审计追踪可在类型层强制
- **现代且主流**：Axum、Vite、shadcn/ui、TanStack Query、Zustand 都是当前各领域第一梯队

### 负面

- **学习成本高**：Rust + RN + 全栈 TS strict 模式，初期产出慢
- **编译时间长**：Rust 全量编译慢，需后续配 sccache / mold linker
- **PDA 单独维护**：RN 是独立技术栈，需要单独的构建/发布流程
- **shadcn/ui 初期慢**：组件需要自己拼，相比 AntD 多一些手工成本
- **utoipa 注解负担**：每个 handler 都要写宏注解

### 风险

- **Rust 招人难**：未来扩团队是瓶颈；缓解措施：优先招 TS 全栈，Rust 部分由核心成员把控
- **SQLx 复杂查询**：跨 8+ 表的报表查询可能写起来痛苦；缓解：必要时引入 SeaORM 或写视图，作为后续 ADR
- **shadcn/ui 不是 npm 包**：升级靠重新复制，需建立自己的组件版本管理约定
- **监管平台对接**："码上放心"等政府接口可能用 SOAP/EDI 老协议，Rust 生态弱；缓解：必要时引入独立 Python/Java 适配层（独立部署，不污染主栈），后续 ADR 决策

---

## 实施约束

- **`#![forbid(unsafe_code)]`** 默认在所有 Rust crate 启用；如需 unsafe 必须单独 ADR
- **TS 禁用 `any`**，必要时用 `unknown` + 类型守卫
- **不允许绕过 OpenAPI 手写前端 API 类型**
- **PDA 任何写操作必须有离线队列处理**（具体方案后续 ADR）

---

## 附录：SQLx 实践规范（v0.2，2026-05-24，由 SPIKE-004 验证）

SPIKE-004 验证了 SQLx 0.8.6 在 wms 项目的 6 项核心假设全部成立（详见 `docs/spikes/spike-004-sqlx-offline.md`）。本附录把验证结论固化为编码规范。

### A. 工具链锁定

```toml
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid", "json", "migrate"] }
```

CLI：`cargo install sqlx-cli --no-default-features --features postgres,rustls --version "^0.8"`

### B. 编码模板

| 场景 | 写法 | 备注 |
|------|------|------|
| 普通 INSERT/UPDATE | `sqlx::query!("INSERT ... VALUES ($1, $2)", a, b).execute(pool).await?` | 编译期校验 SQL + 参数类型 |
| SELECT 多行 | `sqlx::query_as!(Item, "SELECT ... FROM items WHERE ...", arg).fetch_all(pool).await?` | Item 必须 `#[derive(sqlx::FromRow)]` |
| SELECT 单行 | `... .fetch_one(pool).await?` | 0 行返回 `RowNotFound` 错误 |
| SELECT 可选 | `... .fetch_optional(pool).await?` | 返回 `Option<T>` |
| SELECT COUNT | `sqlx::query_scalar!("SELECT COUNT(*) as \"count!: i64\" ...")` | **必须 `as "name!: type"`** 否则推导成 Option |

### C. 离线编译流（强制）

1. **dev 环境改 query!**：
   ```bash
   cd backend && export DATABASE_URL=$(cat .env | grep DATABASE_URL | cut -d= -f2)
   cargo sqlx prepare       # 重生 .sqlx/
   git add .sqlx/           # 入 git
   ```
2. **CI 校验**：`cargo sqlx prepare --check`（diff 触发：改 *.rs 必须同步更新 .sqlx/）
3. **CI build**：`cargo build --offline`（无需 DATABASE_URL）

`.sqlx/` 入 git，是 deterministic JSON（按 query 内容 hash 命名）。

### D. Migration 规范

- 路径：`backend/migrations/<timestamp>_<verb>_<noun>.sql`（如 `20260601000000_create_items.sql`）
- timestamp 格式：`YYYYMMDDHHMMSS`（14 位）
- 不写 down migration（PG 业务表 down 风险高于收益；Wave 4+ 评估）
- 使用 `sqlx migrate run` 或 `sqlx::migrate!()` 自动加载
- **schema 变更必须串行**（ROADMAP 节奏铁律 §7）

### E. 测试规范

```rust
#[sqlx::test]
async fn test_xxx(pool: PgPool) {
    // 自动：建临时 db + 跑 migrations + 测试结束 drop db
    // 不需要清理；不会污染其他测试
}
```

并发安全：每个 `#[sqlx::test]` 独立临时 db，可并行执行（cargo test 默认）。

### F. 集中维护原则

- **所有 `query!` / `query_as!` / `query_scalar!` 集中在单 crate**（如 `wms-infra`）
- 其他 crate 调用 `wms-infra` 暴露的 async 函数；**避免多 crate 各自有 .sqlx/**
- 防止：多 crate 改同一 query → .sqlx/ 文件冲突 / merge 噪声

### G. 治理脚本（Wave 1 W1.A 启动时新增）

`scripts/governance/check_sqlx_prepared.py`（T2 治理）：
- diff 触发：改 `*.rs` 中含 `query!` / `query_as!` / `query_scalar!` 的文件 → 必须同步更新 `.sqlx/`
- CI 实测：`cargo sqlx prepare --check`（spike-004 实测 0.37s）

### H. justfile 入口（Wave 1 W1.A 启动时加）

```makefile
db-up:
    sudo docker compose -f deploy/docker-compose.dev.yml up -d postgres

db-migrate:
    cd backend && sqlx migrate run

db-prepare:
    cd backend && cargo sqlx prepare

db-prepare-check:
    cd backend && cargo sqlx prepare --check

db-reset:
    cd backend && sqlx database drop -y && sqlx database create && sqlx migrate run
```

---

## 参考

- 治理总文档：`docs/governance.md`
- 仓库结构决策：`docs/adr/0002-monorepo-structure.md`
- 治理模型决策：`docs/adr/0003-governance-model.md`
- 阶段路线决策：`docs/adr/0004-phase-roadmap.md`
- **SQLx 实践验证**：`docs/spikes/spike-004-sqlx-offline.md`

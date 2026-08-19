# ADR-0002: 仓库结构（Monorepo + Cargo Workspace + pnpm Workspace）

- 状态：Accepted
- 日期：2026-05-15
- 决策者：项目发起人
- 关联：ADR-0001（技术栈）、`docs/governance.md`

---

## 背景

wms 项目涉及：

- **后端**：Rust，按 GSP 业务上下文分多个 crate（master-data / inbound / inventory / outbound / quality / cold-chain / billing / compliance / audit / api / app / infra / shared-kernel / pda-bff）
- **前端 PC + PAD**：一套 Vite + React 代码（响应式）
- **前端 PDA**：独立 React Native app，但需要复用 TS 类型与 API 客户端
- **跨端契约**：OpenAPI（后端生成 → 前端消费）
- **治理脚本**：Python，独立目录
- **大量文档**：governance、ADR、领域、合规、API

**核心问题**：这些产物放一个仓库还是多个？后端用单 crate 还是多 crate？前端各端怎么共享代码？

---

## 候选方案

### 整体仓库形态

| 候选 | 优点 | 缺点 |
|------|------|------|
| 多仓（backend / web / pda 各一个） | 权限分割、构建独立 | 跨仓改动需多 PR、契约同步难、版本对齐难、CI 复杂 |
| **单 monorepo** | 单 PR 跨端改动、契约一处生成、版本天然对齐、文档与代码同位 | 仓库大、CI 需按变更范围切分 |

### 后端 crate 组织

| 候选 | 优点 | 缺点 |
|------|------|------|
| 单 crate（mod 组织） | 起步快、无 workspace 配置 | 编译全量重编、依赖隔离靠自律 |
| **Cargo Workspace（多 crate）** | 增量编译快、强制依赖隔离（domain ⊥ infra）、未来可独立发布 | 起步配置成本中、Cargo.toml 多 |

ADR-0001 之前讨论倾向"单 crate 起步、未来再拆"，但本项目限界上下文（11 个业务模块）已经在 ROADMAP 明确，**边界清晰，没必要等"自然浮现"**。直接用 workspace 反而更省事，避免后期大规模重构。

### 前端代码组织

| 候选 | 优点 | 缺点 |
|------|------|------|
| 单 package（apps + 子目录） | 简单 | 跨 app 共享代码靠相对路径，不规范 |
| **pnpm Workspace（apps/* + packages/*）** | 共享代码以包形式管理、版本与依赖清晰、与 RN 互通 | 需 pnpm-workspace.yaml 配置 |

### PDA 端是否同仓

| 候选 | 优点 | 缺点 |
|------|------|------|
| 同仓（apps/pda-mobile） | 共享类型 / API client、单 PR 跨端改动、git 历史完整 | RN 构建工具与 Web 不完全兼容（Metro vs Vite）、初学者易混淆 |
| 独立仓 | 各自纯净 | 类型同步成本、双倍 CI、双倍 ADR |

**选同仓**。RN 与 Web 工具链分离不是新问题，pnpm workspace + Metro 配置可以共存（参考成熟案例：Expo 项目、Shopify 等）。

---

## 决策

采用**单 monorepo + Cargo Workspace + pnpm Workspace 三合一**结构。

### 顶层目录

```
wms/
├── README.md                  # 项目总览
├── ROADMAP.md                 # 长期路线
├── TODO.md                    # 当前迭代
├── CHANGELOG.md               # 变更日志（git-cliff 生成）
├── justfile                   # L1-L4 治理入口
├── lefthook.yml               # Git hooks
├── pnpm-workspace.yaml        # pnpm 工作空间
├── .gitignore .editorconfig .gitattributes
│
├── docs/                      # 所有项目文档
│   ├── governance.md
│   ├── architecture.md        # （后续创建）
│   ├── adr/                   # 架构决策记录
│   ├── domain/                # 各限界上下文领域文档
│   └── compliance/            # GSP 合规映射
│
├── backend/                   # Rust 后端（Cargo Workspace 根）
│   ├── Cargo.toml             # workspace 定义
│   ├── Cargo.lock
│   ├── rustfmt.toml
│   ├── clippy.toml
│   ├── crates/
│   │   ├── api/               # HTTP 入口（Axum router、中间件）
│   │   ├── app/               # 应用服务层（用例编排、事务边界）
│   │   ├── domain/            # 领域层（按业务上下文分子 crate 或子模块）
│   │   ├── infra/             # 基础设施（持久化、消息、外部、审计）
│   │   ├── shared-kernel/     # 跨上下文共享（值对象、ID、错误）
│   │   └── pda-bff/           # PDA 专用 BFF
│   ├── migrations/            # SQLx 迁移
│   └── tests/                 # 跨 crate 集成测试
│
├── apps/                      # 前端应用
│   ├── web-admin/             # PC + PAD 共用（Vite + React）
│   └── pda-mobile/            # PDA 端（React Native）
│
├── packages/                  # 前端共享包
│   ├── api-client/            # OpenAPI 自动生成的 TS 客户端
│   ├── domain-types/          # 共享业务类型
│   ├── ui/                    # 跨端 UI 基础（如有）
│   └── utils/
│
├── shared/                    # 跨语言共享资源
│   └── openapi/               # 后端生成的 openapi.json
│
├── scripts/                   # 项目级脚本
│   └── governance/            # 治理脚本（Python）
│
└── governance/                # 治理资产
    ├── gate-rules.toml        # 门禁触发规则
    └── baselines/             # 历史债务锁定（每脚本一个 .json）
```

### 后端 Cargo Workspace 起步形态

第 0 周仅创建目录占位，不写代码。MVP 阶段开始落地的 crate（按 ROADMAP 第一阶段需要的最小集）：

```
backend/crates/
├── shared-kernel/    # 最先落地：ID、错误类型、值对象
├── domain/           # 起步可单 crate，模块内分上下文；规模大后再按上下文拆 crate
├── infra/            # SQLx 仓储实现
├── app/              # 应用服务层
└── api/              # Axum HTTP 入口
```

**关于 domain 是否拆多 crate**：

- 起步：domain 内用模块按上下文组织（`domain::inventory`、`domain::inbound` ……）
- 触发拆分条件（任一满足）：
  - 某个上下文代码量 > 5000 行
  - 跨上下文编译依赖产生瓶颈（cargo build 增量 > 30s）
  - 出现独立部署需求

拆分时单独写 ADR-00XX 备案。

### 前端 pnpm Workspace

```yaml
# pnpm-workspace.yaml
packages:
  - "apps/*"
  - "packages/*"
```

### 命名约定

- Rust crate：`wms-<scope>`（如 `wms-domain`、`wms-api`、`wms-shared-kernel`）
- npm 包：`@wms/<name>`（如 `@wms/api-client`、`@wms/domain-types`）
- 目录用短横线（`web-admin`），不用驼峰

### 依赖方向（强制不变量）

```
api  ──→  app  ──→  domain  ←──  shared-kernel
                       ↑
                     infra （只能被 app 通过 trait 调用，不能被 domain 调用）
pda-bff ──→ app
```

**红线**：

- `domain` 不依赖 `infra`、`api`、`app`
- `api` 不直接依赖 `infra`（只能通过 `app`）
- 跨业务上下文不互相直接依赖（必要时通过 domain event 或 `app` 编排）

由 `scripts/governance/check_layer_dependency.py` 强制（后续脚本任务）。

---

## 后果

### 正面

- **跨端改动一个 PR 完成**：后端字段改了，前端类型同步、PDA 同步都在一次提交里
- **OpenAPI 真正成为唯一真相源**：生成产物在 `shared/openapi/`，所有消费方都读它
- **治理脚本对全仓库有效**：一份 just / lefthook / 治理脚本管所有产物
- **限界上下文显式化**：后端 crate 名字 = 业务上下文名字，新人一眼看懂
- **依赖方向可机器校验**：层级关系写在 Cargo.toml，不靠口头约定
- **文档与代码同位**：`docs/domain/inventory.md` 紧挨 `backend/crates/.../inventory/`

### 负面

- **仓库体量大**：CI 必须做按 path 触发（pnpm-workspace 改动不应触发 cargo 全量构建）
- **新人上手门槛**：要同时理解 Cargo workspace + pnpm workspace + RN 工具链
- **构建工具混合**：Vite + Metro + Cargo 三套构建系统在一个仓库里，需要清晰文档说明各自职责
- **git 仓库会变大**：长期下来 git clone 时间增长；缓解：未来必要时启用 git LFS / partial clone

### 风险

- **RN 工具链与 monorepo 兼容性**：Metro 在 monorepo 中的模块解析需要额外配置；缓解：建 PDA 端时单独 ADR 记录配置方案
- **Cargo.lock 与 pnpm-lock 双锁**：两套锁文件冲突合并复杂；缓解：依赖升级 PR 单独提交，不与业务改动混合
- **过早分 crate 反而拖慢**：domain 在没稳定前就硬拆 crate 会反复调整；缓解：domain 起步单 crate 内分模块，达到拆分触发条件再拆

---

## 实施约束

- 后端 workspace 根 `Cargo.toml` 必须用 `[workspace]` + `resolver = "2"`
- 所有 crate 共享版本号字段，写在 workspace 根的 `[workspace.package]`
- 所有 crate 公共依赖写在 workspace 根的 `[workspace.dependencies]`，子 crate 用 `dep.workspace = true`
- 前端 `apps/*` 不直接互相依赖，必须通过 `packages/*` 共享
- `shared/openapi/openapi.json` 是生成产物但**入库**（保证前端在没启动后端时也能生成类型）
- 跨语言路径引用统一从仓库根算起（治理脚本与文档一致）

---

## 参考

- 治理总文档：`docs/governance.md`
- 技术栈决策：`docs/adr/0001-tech-stack.md`
- 治理模型决策：`docs/adr/0003-governance-model.md`
- 阶段路线决策：`docs/adr/0004-phase-roadmap.md`

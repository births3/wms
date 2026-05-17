# ADR 索引（Architecture Decision Records）

本目录存放 wms 项目的架构决策记录。

> 详细规则：见 `../governance.md` §3.5 ADR 制度

## 目录约定

- 文件名格式：`NNNN-<slug>.md`（4 位顺序号 + 短横线 + slug）
- 编号永不复用；删除的 ADR 仍占用编号
- 状态：`Proposed` / `Accepted` / `Deprecated` / `Superseded by ADR-XXXX`
- 必填段：背景 / 候选方案 / 决策 / 后果 / 参考

## 已 Accepted

| 编号 | 标题 | 状态 | 摘要 |
|------|------|------|------|
| [ADR-0001](0001-tech-stack.md) | 技术栈选型 | Accepted | Rust+Axum / SQLx / PostgreSQL / Vite+React+shadcn/ui / Zustand / TanStack Query / RN PDA / OpenAPI 契约 |
| [ADR-0002](0002-monorepo-structure.md) | 仓库结构 | Accepted | monorepo + Cargo workspace + pnpm workspace；依赖红线 domain ⊥ infra |
| [ADR-0003](0003-governance-model.md) | 治理模型 | Accepted | 5 类 + 4 Tier + Baseline + diff 触发；与 ADR-0006 集成 |
| [ADR-0006](0006-tdd-and-test-layers.md) | TDD + 11 层测试维度 | Accepted | outside-in 双层 TDD；L1-L11 测试维度；Tier×Layer 执行矩阵 |
| [ADR-0007](0007-roadmap-v03-boundary-alignment.md) | 波次路线 v0.3 边界对齐 | Accepted | M11 移除；码上放心归 M-TC；药监 EDI 由 ERP/H8 边界承接；当前路线入口 |

## 已取代

| 编号 | 标题 | 状态 | 取代方 |
|------|------|------|--------|
| [ADR-0004](0004-phase-roadmap.md) | 波次路线（v0.2 取代 v0.1 三阶段） | Superseded by ADR-0007 | [ADR-0007](0007-roadmap-v03-boundary-alignment.md) |

## 编号空缺与未来计划

| 编号 | 状态 | 说明 |
|------|------|------|
| ADR-0005 | 预留 | Git Worktree 工作流（命名 / target/ / node_modules / baseline 协调）。计划 Wave 1 启动前补写。 |
| ADR-0008+ | 未分配 | 后续按需新建（如限界上下文边界原则、消息队列选型 等） |

> 编号永不复用。空缺编号必须在此表登记原因。

## 模板

新建 ADR 时使用本模板（保存为 `NNNN-<slug>.md`）：

```markdown
# ADR-NNNN: <决策标题>

- 状态：Proposed
- 日期：YYYY-MM-DD
- 决策者：<人名>
- 关联：<引用相关 ADR、文档>

## 背景
（为什么要做这个决策；当时面临的问题/约束）

## 候选方案
1. 方案 A — 优点 / 缺点
2. 方案 B — 优点 / 缺点

## 决策
（选了哪个，为什么）

## 后果
- 正面：
- 负面：
- 风险：

## 实施约束
（落地时的强制规则）

## 参考
- ...
```

## 强制 ADR 的场景

下列变更**必须**走 ADR：

- 引入新框架 / 中间件 / 数据库 / 第三方服务
- 改变分层架构或限界上下文边界
- 跨上下文集成方式（同步调用 / 事件 / 编排）
- 安全 / 合规相关决策（审计、加密、监管对接）
- 不可逆决策（数据迁移、协议选定）

## 治理校验

`scripts/governance/validate_adr_index.py` 会自动检查：
- 文件名格式
- 编号唯一性
- 状态合法性
- 必填段完整性
- 本索引文件是否登记所有 ADR

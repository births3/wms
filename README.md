# wms — 医药冷链 GSP 合规仓储管理系统

> 三端协同（PC / PAD / PDA）的医药冷链 WMS，覆盖多货主、3PL、连锁、监管对接全流程。
> 以**治理优先 + TDD 全程驱动**为核心工程纪律，按依赖图分波次推进。

- 项目状态：**Wave 0.5 已完成，Wave 1 准入就绪**
- 当前阶段准备启动横向底座（H1 权限 / H2 审计 / H3 OpenAPI），按 outside-in TDD 推进

## 项目定位

完整功能图谱见 [ROADMAP.md](ROADMAP.md)，包含：

1. 基础信息与资质档案（多货主、供应商资质、商品 UDI）
2. 采购入库（PDA 收货 + 双人验收 + 上架）
3. 库存与质量管控（批次 / 效期 / FIFO / 养护 / 盘点）
4. 销售出库（订单 / 拣选 / 复核 / 随货同行单）
5. 冷链数据集成（接收外部冷链系统数据 / 冷链台账）
6. 报表与审计追踪（GSP 法定台账 / append-only 审计）
7. 零拣复核包装站（Put-to-Light / 保温箱配置）
8. 连锁药店专有（门店经营范围 / 自动补货 / 越库 / O2O）
9. 3PL 计费管理（仓储费 / 作业费 / 月结账单）
10. 运输协同（接收外部 TMS 调度结果 / 周转箱回收）
11. 追溯码与码上放心上报（药监 EDI 由 ERP 负责，WMS 不直连）

## 技术栈速览

详见 [ADR-0001](docs/adr/0001-tech-stack.md)

| 层 | 选型 |
|----|------|
| 后端 | Rust + Axum + SQLx + PostgreSQL + Tokio + tracing |
| Web（PC + PAD） | Vite + React + TypeScript + shadcn/ui + Zustand + TanStack Query + React Router |
| PDA | React Native + TypeScript（独立 app，复用 packages/api-client） |
| 跨端契约 | utoipa → OpenAPI → openapi-typescript（前端 TS 类型自动生成） |
| 包管理 | Cargo workspace（后端） + pnpm workspace（前端） |

## 仓库结构

详见 [ADR-0002](docs/adr/0002-monorepo-structure.md)

```
wms/
├── docs/                       # 项目文档
│   ├── governance.md           # 治理总文档
│   ├── architecture-dependencies.md  # 模块依赖图
│   ├── adr/                    # 架构决策记录
│   ├── domain/                 # 领域文档（按上下文）
│   └── compliance/             # GSP 合规映射
├── backend/                    # Rust 后端（Cargo workspace）
│   ├── Cargo.toml
│   ├── crates/                 # api / app / domain / infra / shared-kernel / pda-bff
│   └── migrations/
├── apps/
│   ├── web-admin/              # PC + PAD（Vite + React）
│   └── pda-mobile/             # PDA（React Native）
├── packages/                   # 前端共享包（api-client / domain-types / ui / utils）
├── shared/openapi/             # 后端生成的 OpenAPI（前端消费）
├── scripts/governance/         # 治理脚本（Python）
├── governance/                 # 治理资产
│   ├── gate-rules.toml         # 门禁触发规则
│   └── baselines/              # 历史债务 baseline
├── justfile                    # T1-T4 治理入口
├── lefthook.yml                # Git hooks
├── ROADMAP.md
├── TODO.md
└── CHANGELOG.md
```

## 工程纪律（必读）

### 1. 治理体系（详见 [docs/governance.md](docs/governance.md)）

- **5 大治理类别**：文档 / 代码 / 质量 / 流程 / 运行
- **4 Tier 执行**：T1 quick-check / T2 task-check / T3 preflight / T4 verify
- **Baseline 机制**：锁定历史债务、新增必须修复、已修复自动收缩
- **diff 触发**：改什么查什么

### 2. TDD + 11 层测试（详见 [ADR-0006](docs/adr/0006-tdd-and-test-layers.md)）

- **outside-in 双层 TDD**：外层 L3 业务流程驱动 + 内层 L1 单元红绿循环
- **11 层测试维度**：L1 单元 / L2 契约 / L3 业务流程 / L4 错误 / L5 数据一致 / L6 并发 / L7 性能 / L8 权限 / L9 兼容 / L10 可观测 / L11 幂等
- **必备维度判定**：写操作必须含 L4+L5+L8+L11

### 3. 波次驱动（详见 [ADR-0007](docs/adr/0007-roadmap-v03-boundary-alignment.md)）

- 核心业务模块、横向业务能力、横向技术能力全部生产化交付，按依赖图分 5 个波次
- 每波内可 worktree 并行（上限 3 个）
- 每波完成都"生产可用"，不是半成品 demo

## 快速开始

### 必备工具

```bash
python3 -V    # ≥ 3.10
git --version # ≥ 2.30
# 后续 Wave 启用：rustc/cargo（≥1.70）、node（≥20）、pnpm（≥8）、just、lefthook
```

### 检查环境

```bash
python3 scripts/governance/validate_environment.py
```

### 跑一次 Tier 1 quick-check

```bash
# 如已安装 just：
just quick-check

# 否则直接调度脚本：
python3 scripts/governance/governance_checks.py --tier T1
```

### 启用 Git hooks

```bash
# 安装 lefthook（参考 https://github.com/evilmartians/lefthook）
lefthook install
```

## 文档导航

| 文档 | 用途 |
|------|------|
| [docs/governance.md](docs/governance.md) | 治理总文档（"宪法"） |
| [docs/architecture-dependencies.md](docs/architecture-dependencies.md) | 模块依赖图（当前模块清单 + 5 波次） |
| [docs/adr/](docs/adr/README.md) | 架构决策记录索引 |
| [ROADMAP.md](ROADMAP.md) | 长期路线（波次） |
| [TODO.md](TODO.md) | 当前 Wave 任务 |
| [CHANGELOG.md](CHANGELOG.md) | 版本变更记录（自 Conventional Commits 生成） |

## 贡献规范摘要

详见 [docs/governance.md](docs/governance.md) §3。

- 分支：`feature/*` / `fix/*` / `chore/*` / `hotfix/*`；main 受保护
- 提交信息：[Conventional Commits](https://www.conventionalcommits.org/)
- 提交前自动跑 T1 quick-check（lefthook pre-commit）
- 推送前自动跑 T3 preflight（lefthook pre-push）
- PR 必须填自查清单（TDD outside-in / 必备维度 / 关联 ADR）
- 单 PR < 400 行；超过强制拆分

## 许可证

待定（项目当前为内部开发阶段）。

---

> 注：本系统涉及 GSP 合规、监管对接、企业级生产数据。任何代码变更都可能影响法规符合性。
> 请严格遵循治理体系与 TDD 纪律，错一条数据可能违法。

# wms 项目治理（Governance）

> 本文档是 wms 项目的"宪法"。所有规范、流程、决策机制都在此声明。
> 修改本文档必须经过 PR，并在文末"变更记录"中追加条目。

- 版本：v0.3（追加文档四层管理 + validate_doc_layers 脚本）
- 日期：2026-05-15
- 适用范围：整个 wms 仓库（backend、apps/*、packages/*、scripts/*、docs/*）

---

## 0. 项目定位

wms 是一个**医药冷链 GSP 合规仓储管理系统**，目标是支撑：

- 多货主 / 多仓 / 3PL 模式
- GSP 全链路合规（资质、批次效期、双人验收、不可篡改审计）
- PC + PAD + PDA 三端协同
- 冷链温湿度采集、监管平台对接

完整功能图谱见 `ROADMAP.md`，分阶段交付。

---

## 1. 治理理念

四条核心理念，凌驾于具体规则之上：

1. **治理即可执行脚本**，不是 PDF 文档。规则必须落到 `just` 命令、git hooks、CI 卡点。
2. **分层执行**，按场景给不同时长预算（T1 < 10s / T2 < 120s / T3 < 5min / T4 < 30min）。**速度即采纳率**。
3. **Baseline 机制**——锁定历史债务、新增必须修复、已修复自动收缩。**渐进式治理**。
4. **diff 触发**——改什么查什么。无差别全量扫描不可持续。

---

## 2. 治理体系全景

### 2.1 五大治理类别

| 类别 | 目标 | wms 起步状态 |
|------|------|--------------|
| 1. 文档治理 | 设计一致性 | 🚧 骨架 |
| 2. 代码治理 | 实现正确性 | 🚧 骨架 |
| 3. 质量治理 | 行为可靠性 | 🚧 待业务 |
| 4. 流程治理 | 协作高效性 | 🚧 骨架 |
| 5. 运行治理 | 系统稳定性 | 🚧 骨架 |

### 2.2 四个执行 Tier

> Tier T1-T4 是"什么时候跑"（时间维度），与 ADR-0006 的"测试层 L1-L11"（覆盖维度）正交。

| Tier | 时间预算 | 命令 | 触发时机 |
|------|---------|------|---------|
| T1 | < 10s | `just quick-check` | 写代码时随手跑、pre-commit |
| T2 | < 120s | `just task-check` | 任务结束、commit 前 |
| T3 | < 5min | `just preflight` | 推送前、pre-push、PR 创建前 |
| T4 | < 30min | `just verify` | 合并前、CI、发版前 |

各 Tier 包含的测试层（L1-L11）见 ADR-0006 §3。

### 2.3 治理脚本约定

- 路径：`scripts/governance/<check_name>.py`
- 语言：Python 3.10+
- 头部必须注明：用途、输入、输出、退出码、所属类别
- 必须支持 `--help` 和 `--json` 输出
- 公共逻辑放 `_baseline.py` / `_diff.py`，不重复实现
- 退出码语义：`0` 通过，`1` 发现违规，`2` 脚本自身错误

---

## 3. 规范层

### 3.1 仓库与分支

- 分支模型：`main` + `feature/*` + `fix/*` + `chore/*` + `hotfix/*`
- 起步阶段不引入 `develop`
- `main` 必须可发布；禁止直接推送、禁止 force push
- 所有合入走 PR + 自审 + CI 通过
- 合并策略：**Squash and merge**

### 3.2 提交信息（Conventional Commits）

格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

**type**：`feat` `fix` `docs` `style` `refactor` `perf` `test` `build` `ci` `chore` `revert`

**scope**（按业务上下文 + 基础设施）：
`master-data` `inbound` `inventory` `outbound` `quality` `cold-chain`
`billing` `compliance` `audit` `pda` `web-admin` `api` `infra` `governance` `docs`

**Breaking Change**：footer 写 `BREAKING CHANGE: <说明>`，对应 SemVer 主版本 +1。

### 3.3 代码风格与质量

#### Rust 端

| 项 | 工具 | 强制度 |
|----|------|--------|
| 格式化 | rustfmt | CI 卡 |
| Lint | clippy（`-D warnings`） | CI 卡 |
| 测试 | cargo test | CI 卡 |
| 文档注释 | cargo doc | 公开 API 必有 |
| 不安全代码 | `#![forbid(unsafe_code)]` | 必须；如需 unsafe 走 ADR |
| 错误处理 | thiserror（库）+ anyhow（应用） | clippy 检查；禁用生产路径 unwrap/expect |
| 模块可见性 | 默认 `pub(crate)` | review 把关 |

#### TypeScript 端

| 项 | 工具 | 强制度 |
|----|------|--------|
| 格式化 | Prettier | CI 卡 |
| Lint | ESLint（typescript-eslint 严格） | CI 卡 |
| 类型检查 | tsc strict | CI 卡；禁用 `any`，必要时 `unknown` |
| 测试 | Vitest + Playwright | CI 卡 |

#### 通用

- `.editorconfig` 统一缩进、LF 换行、UTF-8
- 行长 100
- 命名：Rust `snake_case`/`PascalCase`/`SCREAMING_SNAKE`；TS `camelCase`/`PascalCase`

### 3.4 PR 规范

- PR 标题与提交信息同格式
- PR 描述用模板（变更说明 / 关联 / 自查清单 / 测试方法 / 截图）
- 单 PR 改动 < 400 行（不含自动生成代码），超过强制拆分

### 3.5 文档规范

#### 3.5.1 文档四层管理

文档按**读者 × 稳定性 × 变更频率**分为 4 层，自上而下约束：

| 层 | 名称 | 管什么 | 稳定性 | 变更流程 | 位置 |
|----|------|--------|--------|---------|------|
| L1 | 决策层 | 不可逆技术决策 | 极高（写了不改，只 Deprecated） | 新增走 PR；不修改已 Accepted 的内容 | `docs/adr/` |
| L2 | 规范层 | 所有人必须遵守的规则 | 高 | 修改走 PR + 文末追加变更记录 | `docs/governance.md`、`docs/architecture-dependencies.md` |
| L3 | 设计层 | 业务模块设计细节 | 中（随业务迭代） | 代码 PR 改了行为必须同步更新 | `docs/domain/`、`docs/compliance/`、`docs/architecture.md` |
| L4 | 运营层 | 项目当前状态 | 低（频繁更新） | 随时更新，无需单独 PR | `README.md`、`ROADMAP.md`、`TODO.md`、`CHANGELOG.md` |

**层间约束规则**：

- 下层不能违反上层（代码不能违反 L3 设计，L3 不能违反 L2 规范，L2 不能违反 L1 决策）
- 如果下层需要违反上层 → **必须先修改上层**（走对应变更流程）
- 上层越稳定、变更成本越高；下层越灵活、变更成本越低
- 改了行为不改文档 = 撒谎；文档过时比没文档更糟

**文档治理脚本覆盖**：

| 脚本 | 校验什么 | 覆盖层 | 引入时机 |
|------|---------|--------|---------|
| `validate_adr_index.py` | ADR 编号唯一 / 状态合法 / 必填段 / 索引完整 | L1 | ✅ 已有 |
| `check_doc_links.py` | 所有 .md 相对链接目标存在 | 跨层 | ✅ 已有 |
| `validate_doc_layers.py` | 层间一致性（L2 引用的 ADR 存在；L3 文件与代码目录对应；L4 状态与 git 一致） | L1-L4 | ✅ 已有 |
| `validate_governance_consistency.py` | governance.md 引用的 ADR/规范都存在且状态有效 | L2 | Wave 2 |
| `validate_domain_glossary.py` | L3 文档术语与代码命名一致 | L3 | Wave 3 |
| `validate_gsp_traceability.py` | GSP 条款 → 功能 → 测试 映射完整 | L3 | Wave 3 |
| `check_changelog_freshness.py` | CHANGELOG 与最近 tag 同步 | L4 | Wave 4 |

#### 3.5.2 文档清单

| 文档 | 层 | 位置 | 内容 |
|------|---|------|------|
| docs/adr/NNNN-*.md | L1 | docs/adr/ | 架构决策记录 |
| docs/governance.md | L2 | docs/ | 本文档（治理"宪法"） |
| docs/architecture-dependencies.md | L2 | docs/ | 模块依赖图 |
| docs/architecture.md | L3 | docs/ | 系统架构总览（Wave 1 后创建） |
| docs/domain/*.md | L3 | docs/domain/ | 各限界上下文领域模型 |
| docs/compliance/*.md | L3 | docs/compliance/ | GSP 条款 → 功能映射 |
| README.md | L4 | 根 | 简介、快速开始、目录索引 |
| ROADMAP.md | L4 | 根 | 长期路线图（波次状态） |
| TODO.md | L4 | 根 | 当前 Wave 任务 |
| CHANGELOG.md | L4 | 根 | 自 Conventional Commits 生成 |
| docs/retros/wave-N-retro.md | L4 | docs/retros/ | 波次回顾 |

#### 3.5.3 ADR 制度

- 编号 4 位顺序号，**永不复用**
- 模板：背景 / 候选方案 / 决策 / 后果 / 参考
- 状态：Proposed / Accepted / Deprecated / Superseded by ADR-XXXX
- **强制 ADR 场景**：引入新框架/中间件、改变分层、跨上下文集成方式、安全合规、不可逆决策

### 3.6 测试规范（TDD + 11 层维度）

详细规则见 **ADR-0006**。本节仅给出概要：

- **开发模式**：业务行为采用 **outside-in TDD 双层循环**（外层 L3 业务流程驱动、内层 L1 单元红绿循环）
- **测试维度（11 层）**：L1 单元 / L2 API 契约 / L3 业务流程 / L4 错误处理 / L5 数据一致性 / L6 并发安全 / L7 性能 / L8 权限 / L9 兼容性 / L10 可观测性 / L11 幂等性
- **执行分配**：11 层按 Tier 分时机执行（T2 跑 L1+L2，T3 跑 L3-L5/L8/L11，T4 跑 L6/L7/L9/L10 + 全套 E2E），详见 ADR-0006 §3
- **测试金字塔比例**（参考）：单元 70% / 集成 20% / E2E 10%
- **覆盖率目标**：domain crate ≥ 80%（GSP 核心，硬指标）；其他 ≥ 60%
- **起步阶段**：CI 检测但不强卡，靠 baseline 渐进收紧
- **必备维度判定**：见 ADR-0006 §2.3（写操作必须含 L4+L5+L8+L11）
- **测试组织目录**：见 ADR-0006 §2.4，禁止自由命名（治理脚本依赖）

### 3.7 安全与敏感信息

- `.env` 不入库；`.env.example` 占位入库
- pre-commit 跑 `gitleaks`
- CI 跑 `cargo audit` + `pnpm audit`

#### 审计追踪强制项（GSP 核心）

- 所有库存数量变更必须经过 audit 层
- 审批操作记录原始痕迹（旧值/新值/操作人/时间/IP）
- 审计表 append-only，**禁止 UPDATE/DELETE**

### 3.8 依赖管理

- 引入新依赖必须 PR 描述理由（必要性 / 维护活跃度 / 许可证 / 漏洞）
- 禁止 GPL，允许 MIT / Apache-2.0 / BSD
- Cargo.lock 与 pnpm-lock.yaml 必须入库
- 每月一次依赖升级 PR（chore 类型）

### 3.9 环境与配置

- 环境分层：local / dev / staging / prod
- 配置项命名：`<MODULE>_<NAME>`
- **禁止**在代码里写 `if env == "prod"` 分支逻辑
- **禁止**配置项无默认值（密钥类除外）

---

## 4. 治理层（元规则）

### 4.1 决策机制

| 类型 | 例子 | 方式 |
|------|------|------|
| 战术 | 用 `match` 还是 `if let` | PR review |
| 战役 | 某模块领域模型设计 | 短文档 + 评审 |
| 战略 | 引入新技术栈、改架构 | **必须 ADR** |

### 4.2 角色（哪怕只有一个人）

| 角色 | 职责 |
|------|------|
| 架构师 | 维护 architecture.md / ADR；技术栈选型 |
| 领域专家 | 维护 docs/domain/*；业务规则正确性 |
| 合规负责人 | 维护 docs/compliance/*；GSP 条款映射 |
| 开发者 | 写代码、写测试、PR review |
| 运维 | CI/CD、部署、监控（可缺位） |

### 4.3 演进原则（红线）

1. **YAGNI**：不为想象需求写代码；抽象前先有 3 个具体用例
2. **分层不可绕过**：domain ⊥ infra；api 不直接调 SQLx
3. **删除优于注释**：弃用代码立即删，git 史会保留
4. **显式优于隐式**：错误显式返回，配置显式声明
5. **合规优先**：GSP 相关变更必须经合规自查
6. **可逆性优先**：migration 必须支持 down；不可逆决策走 ADR
7. **文档是代码的一部分**：行为变了不改文档 = 撒谎

### 4.4 不变量（红线，任何理由不可违反）

- 库存数据不允许直接 SQL 修改，必须经领域服务
- 审计日志表只能 INSERT
- 密钥不入 git
- main 分支必须可发布
- 任何 PR 必须 CI 绿
- GSP 关键功能必须有测试

### 4.5 起步阶段简化

| 项 | 完整版 | 起步版 |
|----|--------|--------|
| ADR | 多人评审 24h | 自审即生效，**必须写** |
| PR | 多人 review | 自审 + 走流程 |
| CI | GitHub Actions 全套 | 先本地 hook，第一版跑通后上 CI |
| 覆盖率 | 卡门槛 | 不卡，每周看趋势 |

**核心：流程一个不少，严格度可调。**

---

## 5. 工具链

| 治理对象 | 工具 |
|----------|------|
| 代码格式 | rustfmt + Prettier |
| 代码 lint | clippy + ESLint |
| 提交信息 | commitlint（或本仓自研脚本） |
| Git hooks | **lefthook** |
| 任务编排 | **just** |
| Secret 扫描 | gitleaks |
| 依赖漏洞 | cargo-audit + pnpm audit |
| 变更日志 | git-cliff |
| 治理脚本 | Python 3.10+ |

---

## 6. 与其他文档的关系

```
docs/governance.md（本文档，规则源头）
  ├─→ docs/coding-standards.md              （代码书写规范）
  ├─→ docs/adr/0001-tech-stack.md           （技术栈决策）
  ├─→ docs/adr/0002-monorepo-structure.md   （仓库结构决策）
  ├─→ docs/adr/0003-governance-model.md     （治理模型决策）
  ├─→ docs/adr/0004-phase-roadmap.md        （波次路线决策）
  ├─→ docs/adr/0006-tdd-and-test-layers.md  （TDD + 11 层测试）
  ├─→ docs/architecture-dependencies.md     （模块依赖图）
  ├─→ justfile                              （T1-T4 执行入口）
  ├─→ lefthook.yml                          （Git hooks 落地）
  ├─→ scripts/governance/*.py               （治理脚本实现）
  ├─→ governance/gate-rules.toml            （门禁触发规则）
  └─→ governance/baselines/*.json           （历史债务锁定）
```

---

## 7. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-15 | v0.1 | 初版骨架（第 0 周） |
| 2026-05-15 | v0.2 | L1-L4 改名 T1-T4；§3.6 测试规范引用 ADR-0006（TDD + 11 层）；文档关系图补 ADR-0006 与依赖图 |
| 2026-05-15 | v0.3 | §3.5 文档规范重写为"四层管理"（L1 决策 / L2 规范 / L3 设计 / L4 运营）；新增 validate_doc_layers.py 脚本 |

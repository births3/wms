# wms 项目治理（Governance）

> 本文档是 wms 项目的"宪法"。所有规范、流程、决策机制都在此声明。
> 修改本文档必须经过 PR，并在文末"变更记录"中追加条目。

- 版本：v0.6（统一行数治理阈值）
- 日期：2026-06-26
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

> **理念 4 落地说明**：diff 触发由 `task_check.py` 实现（基于 `governance/gate-rules.toml`），仅在 **T2 及以上** Tier 启用；**T1 仍为全量扫描**（因 T1 必须无脑可跑、不依赖 git history）。在 Wave 0 阶段 T2/T3/T4 仅累积 T1，diff 触发的真正价值会在 Wave 1+ 才显现（届时 backend / frontend 文件量大，全量扫描不可持续）。

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

### 3.2 提交信息（Conventional Commits 中文版）

格式：

```
<类型>(<范围>)：<描述>

<正文>

<脚注>
```

**类型（type）**：

| 类型 | 含义 |
|------|------|
| 功能 | 新功能 |
| 修复 | bug 修复 |
| 文档 | 仅文档变更 |
| 格式 | 代码格式（不影响运行） |
| 重构 | 重构（非功能非修复） |
| 性能 | 性能优化 |
| 测试 | 测试相关 |
| 构建 | 构建系统/依赖 |
| 集成 | CI 配置 |
| 杂项 | 其他杂项 |
| 回滚 | 回滚提交 |

**范围（scope）**：
`基础档案` `入库` `库存` `出库` `质量` `冷链` `计费` `合规` `审计`
`pda` `管理端` `接口` `基础设施` `治理` `文档`
`追溯码` `对账` `药检` `校验` `质量联系单` `企微` `快递`
`原型`

**示例**：
```
功能(入库)：ASN 状态机加入待校验状态

收货通知单创建后先经过校验规则模块自动校验，
校验通过后才进入"待收货"状态。

关联：US-M2-001, M-VR
```

**Breaking Change**：脚注写 `破坏性变更：<说明>`，对应 SemVer 主版本 +1。

#### 描述行（subject）规则

- 长度：**≤ 50 字符**（不含类型/范围前缀）
- 句式：动词开头（添加/修复/重构/...）
- 末尾不带句号
- 不写"我"等主语，使用客观语气

#### 正文（body）规则

- 复杂改动**必填**正文；琐碎改动可省略
- 必填触发条件（任一）：
  - 改动 ≥ 3 个文件
  - 涉及业务规则 / 状态机 / GSP 合规 / 审计
  - 引入新依赖
  - Breaking Change
- 正文内容：动机（为什么）+ 改动概要（做了什么）+ 影响范围（影响谁）
- 与描述行之间空一行，每行 ≤ 72 字符

#### 脚注（footer）规则

- 关联：`关联：US-M2-001, ADR-0006, M-VR`（一行多项以逗号分隔）
- 破坏性变更：`破坏性变更：<说明>`
- 协作：`Co-authored-by: 名字 <邮箱>`
- 关闭 issue：`Closes #123`（如使用 issue 跟踪）

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
- 单 PR 改动 < 800 行（不含自动生成代码）；≥ 600 行 warning，≥ 800 行强制拆分

#### 3.4.1 commit 粒度规则

合并策略是 **Squash and merge**（§3.1），因此：

- **本地 commit 可细可粗**：一个 PR 可包含多个本地 commit；squash 时合并为一个
- **推荐节奏**：每完成一个**原子任务**就 commit 一次（如修一个 bug、改一组关联文件、一次重构）
- **必须拆分**的情形（即使在同一 PR 内也分多个 commit）：
  - 跨 ≥ 2 个 scope（如同时改了"治理"和"基础档案"）
  - 跨"格式" / "重构" / "功能"等不同 type（重构和功能不要混提交，便于 reviewer 区分）
  - 单次累积变更 ≥ 800 行
  - Breaking Change 必须独立 commit
- **不建议拆分**的情形：
  - 同主题强关联的多个文件（如改一个故事 + 配套字段表 + 配套测试）
  - 同步重命名（rename 应一次完成）

#### 3.4.2 何时建议 commit

- 跑通 T1 治理脚本 + 关联测试 → 可 commit
- 单一主题完成 → 可 commit
- 长会话/长任务结束前 → **必须** commit（避免大量未提交变更累积）

#### 3.4.3 PR 与 commit 的关系

| 维度 | PR | commit（本地）|
|---|---|---|
| 粒度 | 一个完整功能/修复 | 一个原子改动 |
| 行数上限 | < 800 行（硬约束），≥ 600 行 warning | 无硬约束，但 ≥ 600 行应拆分 |
| 单 scope | **强制单一**（跨 scope 拆 PR）| 推荐单一（跨 scope 拆 commit）|
| 提交人 | review 通过后 squash | 实时提交 |

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
| `validate_adr_index.py` | ADR 编号唯一 / 状态合法 / 必填段 / 索引完整 | L1 | ✅ 已有（T1）|
| `check_doc_links.py` | 所有 .md 相对链接目标存在 + 附录跨模块引用 | 跨层 | ✅ 已有（T1）|
| `validate_doc_layers.py` | 层间一致性（L2 引用的 ADR 存在；L3 文件与代码目录对应；L4 状态与 git 一致）| L1-L4 | ✅ 已有（T1）|
| `check_user_story_structure.py` | 用户故事 As a/I want/So that 三件套结构 + 验收标准块（仅结构，不查语义）| L3 | ✅ 已有（T1）|
| `check_glossary_consistency.py` | 禁用同义词在文档中是否出现（基于 glossary.md 表格）| L3 | ✅ 已有（T1）|
| `check_approval_source_chain.py` | 库存状态变更故事必须声明审批源 | L3 | ✅ 已有（T1）|
| `check_config_center_consistency.py` | 故事使用 ⇄ M1-008 配置中心 ⇄ 故事默认值 三向一致 | L3 | ✅ 已有（T1）|
| `check_pda_story_completeness.py` | PDA 故事三件套（字段表 + 扫码顺序 + 离线声明）| L3 | ✅ 已有（T1）|
| `check_pda_production_gate.py` | ADR-0027 Accepted 前禁止启动 `apps/pda-mobile` 生产 app 文件 / workspace / lockfile / 依赖 / scripts；Accepted 后校验 Spike accepted evidence 与候选一致性 | L1/L4 | ✅ 已有（T1，PDA 新方案）|
| `check_gsp_field_traceability.py` | 70 GSP 字段在故事字段表中有实现（v25 字段追溯矩阵）| L3 | ✅ 已有（T1，原计划 Wave 3，提前实现）|
| `check_project_rtm.py` | 项目级 RTM 覆盖故事、前端、后端、测试、合规风险矩阵；故事引用有效；部分覆盖项必须写缺口和补齐路径 | L3 | ✅ 已有（T1）|
| `check_owner_scope_sql.py` | 从 migration 识别含 `owner_id` 的租户表，静态扫描仓储层 SQL 的 owner 写入与过滤谓词 | L3/L5 | ✅ 已有（T1）|
| `check_baseline_health.py` | baseline 数量单调下降 + 过期检测（防止滥用 baseline 抑制噪音）| 跨层 | ✅ 已有（T1，v0.4 加入）|
| `check_governance_coverage.py` | 所有 `check_*` / `validate_*` 治理脚本，以及被 gate-rules.toml 用作 evidence gate 的 `report_*` 脚本，必须被运行器覆盖，并纳入 smoke 或记录明确豁免 | 跨层 | ✅ 已有（T1，元治理）|
| `check_wave6_evidence_preflight.py` | Wave 6 evidence runbook / just 入口 / validator 链路完整性 | L4 | ✅ 已有（T1，Wave 6）|
| `validate_governance_consistency.py` | governance.md 引用的 ADR/规范都存在且状态有效 | L2 | Wave 2（占位）|
| `validate_domain_glossary.py` | L3 文档术语与代码命名一致 | L3 | Wave 3（占位）|
| `check_changelog_freshness.py` | CHANGELOG 与最近 tag 同步 | L4 | ✅ 已有（T1，Wave 5）|

#### 3.5.2 文档清单

| 文档 | 层 | 位置 | 内容 |
|------|---|------|------|
| docs/adr/NNNN-*.md | L1 | docs/adr/ | 架构决策记录 |
| docs/governance.md | L2 | docs/ | 本文档（治理"宪法"） |
| docs/architecture-dependencies.md | L2 | docs/ | 模块依赖图 |
| docs/architecture.md | L3 | docs/ | 系统架构总览（Wave 1 后创建） |
| docs/domain/*.md | L3 | docs/domain/ | 各限界上下文领域模型 |
| docs/compliance/*.md | L3 | docs/compliance/ | GSP 条款 → 功能映射 |
| docs/prototypes/*.md | L3 | docs/prototypes/ | 原型治理、组件注册、原型转生产清单 |
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

### 3.6.1 前端原型先行流程（ADR-0029）

Wave 1 起，涉及一线操作或复杂业务流程的前端页面采用"前端原型先行，确认后迁移生产"：

```text
用户故事
→ 概念审计
→ 模式提炼
→ 领域模型 / API 草案
→ 前端高保真原型
→ 业务走查确认
→ 领域模型 / API 契约冻结
→ outside-in TDD 生产实现
```

强制边界：

- `prototypes/` 是业务走查工具，允许 mock 数据和演示交互，不承载生产 API 调用。
- `packages/ui` 是原型与生产前端共享的组件库，禁止放页面级业务编排。
- `apps/web-admin` 是生产 PC/PAD 前端，必须通过 OpenAPI 生成的 API client 接口访问后端。
- 原型页不得直接复制成生产页；迁移必须通过 `docs/prototypes/prototype-to-production.md` checklist。
- 新增原型页仍需 Tabs.tsx / manifest.toml / baseline PNG 三同步，并通过 T1；PR 前按风险跑 T3 视觉回归。

### 3.7 安全与敏感信息

#### 不得入库的文件 / 内容（红线）

- 配置类：`.env` / `.env.local` / `.env.production`（占位用 `.env.example`）
- 密钥类：私钥（`.pem` / `.key` / `id_rsa*`）、token、证书私钥
- 凭据类：数据库密码、API key、OAuth client secret、JWT 签名密钥
- 客户/用户数据：真实生产数据库 dump、含 PII 的样本数据
- 大文件：> 5MB 的二进制 / 媒体文件（用 LFS 或 OSS）
- IDE 个人配置：`.idea/`、`.vscode/settings.json`（除非项目级配置共享）

如需在仓库提供示例，使用占位（如 `DATABASE_URL=postgres://USER:PASS@HOST/DB`），并在 `.gitignore` 加防御。

#### 自动化检测

- pre-commit 跑 `gitleaks`（`.gitleaks.toml` 可白名单已知占位）
- CI 跑 `cargo audit` + `pnpm audit`
- 发现疑似密钥 → 立即停止 commit，提示用户检查

#### 误提交后处理

1. 立即 revoke 该密钥/token
2. 用 `git filter-repo` 或 BFG 从历史中清除（**有风险操作，必须用户审批**）
3. force push 重写远程历史（**有风险操作，必须用户审批**）
4. 通知所有协作者重新 clone

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

### 4.6 Tier 启动 SOP（v0.4 加入）

每进入新 Wave 时，必须为 T2/T3/T4 添加对应治理脚本，让 Tier 体系不停留在 Wave 0 占位状态：

| Wave | 必须新增的治理脚本 | 注册到 | 对应 gate-rules 规则 |
|------|-----------------|------|------------------|
| Wave 1 | `check_layer_dependency.py`（Rust 分层）/ `check_unsafe_and_unwrap.py` | T2 | `backend/crates/**` |
| Wave 1 | `check_handler_test_coverage.py`（baseline 起步）| T2 | `backend/crates/api/src/**` |
| Wave 1 | `check_field_coding_standards.py`（字段命名/类型/加密/审计）| T1 | `docs/compliance/gsp-field-traceability.md` + `docs/domain/user-stories-*.md` |
| Wave 1 | `check_business_rules_registry.py`（业务规则字段引用）| T1 | `docs/compliance/gsp-business-rules-registry.md` + `docs/domain/user-stories-*.md` |
| Wave 1 | `check_openapi_in_sync.py` / `validate_openapi_artifacts.py` / `check_openapi_contract.py` | T2 | `shared/openapi/openapi.json` + `backend/crates/api/**` + `backend/crates/domain/**` + `packages/api-client/src/schema.ts` |
| Wave 3 | `check_audit_trail_coverage.py` / `check_idempotency_test.py` | T3 | `backend/crates/domain/src/inventory/**` |
| Wave 3 | `check_cold_chain_data_freshness.py` | T3 | `backend/crates/domain/src/cold_chain/**` |
| Wave 4 | `check_perf_baseline.py` / `check_api_compat.py` | T4 | （CI 全量，非 diff 触发）|
| Wave 4 | `check_observability_signals.py`（L10 可观测）| T4 | （CI 全量，非 diff 触发）|
| Wave 5 | `check_changelog_freshness.py` | T1 | `*.md`（变更前必跑）|

> **事实之源约定**：本表与 `governance/gate-rules.toml` 中的占位规则**必须保持一致**；以本表为权威源，gate-rules.toml 仅作为脚本侧实现承接。`task_check.py --strict` 模式（当前为 Wave 1+ 准备中）会强制检查未实现脚本。

**门禁强制**：
- `just wave-N-ready` 必须列出"应当新增的脚本"清单
- 以及在 `task_check.py --strict` 模式下，gate-rules.toml 中引用但未实现的脚本视为失败（CI 启用 --strict）
- 每 Wave 第一周完成补齐前，Wave 演进不通过

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
| 2026-05-17 | v0.4 | 治理体系审计修复（11 项）：(P0) task_check 加 --strict 模式；§3.5.1 表格 gsp-field 时机更正；§1 加 diff 触发理念落地说明；(P1) 新增 check_baseline_health.py（T1）+ Python 包依赖检查 + 路径硬编码迁出到 governance/check-data.toml + 12 个治理脚本 smoke + core_logic 测试（共 81 项）；(P2) §4.6 加 Tier 启动 SOP + wave-1-ready 加门 + just tier-timing 落地 + glossary 词边界检测扩展到 ASCII + structure 输出补语义说明 |
| 2026-05-17 | v0.4.1 | v0.4 review 二轮修复（5 项）：gate-rules.toml ↔ §4.6 Wave 时序统一（OpenAPI Wave 2 / handler Wave 1 / cold-chain Wave 3）；§4.6 表格加"对应 gate-rules 规则"列 + "事实之源约定"段；check_baseline_health 默认仅检测不改 working tree（--update-snapshot 才写）；task_check.py docstring 加 --strict 启用时机说明；governance/baselines/README.md 加治理元数据文件入库说明 |
| 2026-05-17 | v0.4.2 | v0.4.1 review 三轮修复（4 项 + 2 验证）：(P0) §7 补 v0.4.1 变更记录；(P1) 新增 check_governance_consistency.py（元检查 §4.6 ↔ gate-rules.toml 一致性）；(P2) wave-1-ready 加 baseline-health 初始化 + --strict 启用提醒；测试加 v0.4.1 行为回归；baselines/README.md 标题改为"治理债务与元数据" |
| 2026-05-24 | v0.5 | 接受 ADR-0029 前端原型先行工作流：§3.6.1 增加"用户故事 → 原型走查 → 契约冻结 → TDD 生产实现"流程；文档清单加入 `docs/prototypes/*.md`；原型转生产必须走 checklist |
| 2026-06-26 | v0.6 | 放宽行数治理阈值：PR 规模与文件规模统一为 ≥ 600 行 warning、≥ 800 行门禁 / 必须拆分；同步 AGENTS.md 速查约束和前端页面脚本阈值 |
| 2026-06-28 | v0.6.1 | 新增项目级 RTM 缺口说明约束与仓储层 SQL 货主隔离静态门禁；同步 T1、gate-rules 和 smoke 覆盖 |

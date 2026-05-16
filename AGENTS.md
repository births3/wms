# AGENTS.md

> AI 编码助手协作指引。具体规范通过引用获取，本文件不重复内容。

## 本文件的书写规范

- **只写引用和速查约束**，不写具体实现细节
- 具体规范的唯一真相源是被引用的文档，本文件不复制内容
- 新增规范文档时，在"必读文档"段追加引用
- 约束变更时同步更新"核心约束"段
- 本文件修改必须随对应规范文档的 PR 一起提交
- 保持极简：AI 助手应在 30 秒内读完本文件，再按需深入引用文档

## 设计流程（标准步骤）

> 每个阶段完成后进入下一个，不可跳过。

```
1. 用户故事编写
   → 从用户视角描述所有功能需求
   → 输出：docs/domain/user-stories-*.md

2. 概念审计（Concept Audit）★★
   → 用多视角镜头审视系统，发现认知盲区
   → 不是找"缺了哪个功能"，而是找"缺了哪个维度/概念"
   → 输出：新概念清单 → 和用户确认 → 补充到对应文档
   
3. 模式提炼（Pattern Extraction）★
   → 扫描所有故事，提取重复模式
   → 识别"多模块共用的能力"和"用户不操作但系统必须有的能力"
   → 输出：基础设施清单 → 和用户确认 → docs/infra/technical-specs.md
   
4. 领域模型设计
   → 实体/值对象/聚合根/状态机/领域事件
   → 输出：docs/domain/domain-model.md

5. 接口契约设计
   → API 定义/模块间事件契约
   → 输出：docs/api/ 或 OpenAPI spec

6. 代码实现（TDD）
   → outside-in：先写失败测试再写代码
```

### 概念审计的方法

详见 [docs/concept-audit.md](docs/concept-audit.md)（审计结果）。

方法速查：
- **多视角镜头**：用户/开发者/运维/安全/数据/时间/失败/边界 8 个视角
- **概念完备性**：对已有概念问"它的对立面/补集是什么"
- **类比迁移**：从成熟领域借概念

### 模式提炼的具体方法

| 方法 | 做什么 | 发现什么 |
|------|--------|---------|
| 词频统计 | 统计故事中重复出现的动词/名词 | 审计追踪(102次)→审计引擎 |
| 依赖分析 | 找被 ≥3 个模块依赖的能力 | 状态流转→状态机引擎 |
| 用户不可见分析 | 找"用户不操作但必须存在"的能力 | ERP 对接→防腐层 |
| 技术关注点分离 | 找混在业务故事里的技术需求 | 离线同步→PDA SDK |

## 需求回复格式模板

> AI 向用户提出需要确认的问题时，必须使用以下格式，方便用户快速回复。

### 格式

```markdown
| # | 问题 | 选项/说明 | 建议 |
|---|------|---------|------|
| 1 | 简短问题描述 | A) 选项一 B) 选项二 C) 选项三 | 建议选 X |
| 2 | ... | ... | ... |
```

### 规则

1. **每个问题必须有编号**（方便用户用数字回复，如"1A 2B 3 可以"）
2. **有明确选项时列出选项**（A/B/C），不要开放式提问
3. **没有选项时给出建议默认值**（用户可以直接说"可以"）
4. **问题数量控制在 10 个以内**（超过 10 个分批问）
5. **按紧急程度排序**：🔴 必须现在决定 > 🟡 需要确认 > 🟢 用默认值即可
6. **相关问题分组**（用标题分隔）

### 用户回复方式

用户可以用以下任何方式回复：
- 数字 + 选项："1A 2B 3C"
- 数字 + 描述："1 两种都要 2 用默认"
- 批量确认："3-6 统一用你的建议"
- 单独展开："7 详细探讨"

## 发现缺口时的确认流程

> **核心原则：AI 不能自行决定新增模块/故事/基础设施，必须和用户确认。**

当 AI 在工作中发现以下情况时，**必须暂停并向用户确认**：

### 触发条件

1. **功能缺口**：发现某个业务场景没有对应的用户故事覆盖
2. **模块缺失**：发现需要新增一个独立模块（业务模块或基础设施模块）
3. **设计冲突**：发现两个已有故事/决策之间存在矛盾
4. **抽象机会**：发现多个模块有重复模式，可以抽象为公共能力
5. **技术决策**：需要做出影响架构的技术选择（如新增依赖、改变数据模型）
6. **范围变更**：某个故事的实现复杂度远超预期，可能需要拆分或简化

### 确认流程

```
1. 描述发现：清晰说明发现了什么问题/缺口
2. 分析影响：说明这个缺口影响哪些模块/流程
3. 提出选项：给出 2-3 个解决方案（含利弊）
4. 等待确认：不要自行选择方案，等用户决定
5. 记录决策：确认后记录到 clarifications.md
```

### 不需要确认的情况

- 修复脚本报错（error 级别）
- 修复术语违规
- 修复文件命名不合规
- 补充审计追踪（所有写操作都需要，这是已确认的规则）
- 代码层面的重构（不改变功能）

## 必读文档（按优先级）

1. [docs/coding-standards.md](docs/coding-standards.md) — 代码书写规范（Rust / TS / 跨端 / 禁止清单）
2. [docs/governance.md](docs/governance.md) — 治理体系（5 类 + 4 Tier + Baseline + 文档四层管理）
3. [docs/adr/0006-tdd-and-test-layers.md](docs/adr/0006-tdd-and-test-layers.md) — TDD + 11 层测试
4. [docs/architecture-dependencies.md](docs/architecture-dependencies.md) — 模块依赖图（11 业务 + 3 横向 + 5 波次）
5. [docs/adr/README.md](docs/adr/README.md) — 所有架构决策索引
6. [docs/infra/technical-specs.md](docs/infra/technical-specs.md) — 基础设施技术规格（H6 状态机 / H7 导入导出 / H8 ERP 防腐层 / H9 打印）
7. [docs/concept-audit.md](docs/concept-audit.md) — 概念审计报告（8 镜头扫描结果 + 数据量评估）
8. [docs/domain/clarifications.md](docs/domain/clarifications.md) — 业务澄清记录（42 项决策）
9. [docs/glossary.md](docs/glossary.md) — 术语表（54 个，含禁用词）

## 业务文档索引

| 文档 | 用途 |
|------|------|
| [docs/domain/user-stories-m1-master-data.md](docs/domain/user-stories-m1-master-data.md) | M1 基础档案（10 个故事） |
| [docs/domain/user-stories-m2-inbound.md](docs/domain/user-stories-m2-inbound.md) | M2 入库（8 个故事） |
| [docs/domain/user-stories-m3-inventory.md](docs/domain/user-stories-m3-inventory.md) | M3 库存（10 个故事） |
| [docs/domain/user-stories-m4-outbound.md](docs/domain/user-stories-m4-outbound.md) | M4 出库（11 个故事） |
| [docs/domain/user-stories-m5-cold-chain.md](docs/domain/user-stories-m5-cold-chain.md) | M5 冷链（3 个故事） |
| [docs/domain/user-stories-m6-audit-report.md](docs/domain/user-stories-m6-audit-report.md) | M6 报表（3 个故事） |
| [docs/domain/user-stories-m8-retail-chain.md](docs/domain/user-stories-m8-retail-chain.md) | M8 连锁（3 个故事） |
| [docs/domain/user-stories-m9-billing.md](docs/domain/user-stories-m9-billing.md) | M9 计费（3 个故事） |
| [docs/domain/user-stories-m10-tms-plus.md](docs/domain/user-stories-m10-tms-plus.md) | M10 TMS+（3 个故事） |
| [docs/domain/user-stories-m11-regulatory-edi.md](docs/domain/user-stories-m11-regulatory-edi.md) | M11 监管 EDI（3 个故事） |
| [docs/domain/user-stories-mte-task-engine.md](docs/domain/user-stories-mte-task-engine.md) | M-TE 任务引擎（11 个故事） |
| [docs/domain/user-stories-mrp-replenishment.md](docs/domain/user-stories-mrp-replenishment.md) | M-RP 补货（4 个故事） |
| [docs/domain/user-stories-mpk-packing-station.md](docs/domain/user-stories-mpk-packing-station.md) | M-PK 包装站（6 个故事） |
| [docs/domain/user-stories-mvr-validation-rules.md](docs/domain/user-stories-mvr-validation-rules.md) | M-VR 规则引擎（5 个故事） |
| [docs/domain/user-stories-mtc-traceability-code.md](docs/domain/user-stories-mtc-traceability-code.md) | M-TC 追溯码（6 个故事） |
| [docs/domain/user-stories-mql-quality-liaison.md](docs/domain/user-stories-mql-quality-liaison.md) | M-QL 质量联系单（5 个故事） |
| [docs/domain/user-stories-mcg-code-generator.md](docs/domain/user-stories-mcg-code-generator.md) | M-CG 编码生成（2 个故事） |
| [docs/domain/user-stories-msa-stock-adjustment.md](docs/domain/user-stories-msa-stock-adjustment.md) | M-SA 报损报溢（3 个故事） |
| [docs/domain/user-stories-mrc-reconciliation.md](docs/domain/user-stories-mrc-reconciliation.md) | M-RC 对账（4 个故事） |
| [docs/domain/user-stories-mdi-drug-inspection.md](docs/domain/user-stories-mdi-drug-inspection.md) | M-DI 药检单（4 个故事） |
| [docs/domain/user-stories-h4-wechat-notify.md](docs/domain/user-stories-h4-wechat-notify.md) | H4 企业微信（4 个故事） |
| [docs/domain/user-stories-h5-express.md](docs/domain/user-stories-h5-express.md) | H5 快递（5 个故事） |

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
- **发现缺口必须确认**：新增模块/故事/基础设施前必须和用户确认（见上方流程）

## 当前阶段

Wave 0 治理骨架已完成。下一步：Wave 1 横向底座（H1 权限 / H2 审计 / H3 OpenAPI）。

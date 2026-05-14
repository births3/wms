# ADR-0003: 治理模型（5 类 + 4 Tier + Baseline + diff 触发）

> **术语说明**：本 ADR 中"Tier T1-T4"是治理执行层级（时间维度，什么时候跑）。
> 与 ADR-0006 的"测试层 L1-L11"（覆盖维度，跑什么）正交。早期版本曾用 L1-L4 描述 Tier，
> 为避免与 ADR-0006 冲突，统一改为 T1-T4。

- 状态：Accepted
- 日期：2026-05-15
- 决策者：项目发起人
- 关联：`docs/governance.md`、ADR-0001（技术栈）、ADR-0002（仓库结构）

---

## 背景

wms 项目同时面对几个长期压力：

- **GSP 合规**：审计追踪、批次效期、双人验收等规则不能靠人工把关
- **三端协同**：后端 Rust + Web React + PDA RN，跨语言约束多
- **个人起步、未来扩团队**：规则必须从一开始就成文且自动化，否则人多后追加成本极高
- **模块多 / 周期长**：11 个业务模块、5 年以上生命周期，规则会随时间演进

如果治理只写在 PDF 或 Wiki 里，没人会看；写得太严，开发体验崩，规则被绕开；写得太松，债务雪崩。

**核心问题**：用什么治理模型，能在"严格"与"可持续"之间取得平衡？

---

## 候选方案

### 方案 A：传统重度治理（CMM/瀑布式）

需求规格 → 详细设计 → 编码规范 → 评审会议 → QA 测试 → 验收。

| 优点 | 缺点 |
|------|------|
| 文档齐全、责任清晰 | 一个人或小团队压根跑不动；流程慢；规则与代码脱节 |

### 方案 B：靠 Code Review + 文档约束

写规范文档，靠 PR review 把关。

| 优点 | 缺点 |
|------|------|
| 启动成本低 | 规则执行靠人自律；review 疲劳后必失效；新人难传承 |

### 方案 C：纯 CI 卡点

所有规则塞 CI，PR 不绿不能合。

| 优点 | 缺点 |
|------|------|
| 强约束 | 反馈慢（push 后才知错）；本地写代码体验差；CI 运行成本高 |

### 方案 D（采纳）：分层 + 可执行 + Baseline + diff 触发

把规则做成可执行脚本，按时间预算分 4 个 Tier（写代码 / 提交 / 推送 / 合并），按 git diff 触发最小检查集，用 baseline 锁定历史债务并自动收缩。

参考：成熟的中大型工程治理体系（如开源项目 / 大厂内部 monorepo 实践）的精炼。

| 优点 | 缺点 |
|------|------|
| 反馈最近端（写代码时就报错）；规则即代码、可演进；务实接受历史债 | 需投入维护治理脚本本身；脚本数量上限要克制 |

---

## 决策

采纳**方案 D**，记为 wms 治理模型。具体包含 4 个机制：

### 机制 1：5 大治理类别（What）

把所有治理规则按目标归到 5 类，避免规则散乱。

| 类别 | 目标 | 典型脚本（最终形态） |
|------|------|----------------------|
| 1. 文档治理 | 设计一致性 | check_doc_links / validate_adr_index / validate_gsp_traceability / validate_openapi_artifacts |
| 2. 代码治理 | 实现正确性 | check_layer_dependency / check_unsafe_and_unwrap / check_audit_trail_coverage / check_modularization_candidates |
| 3. 质量治理 | 行为可靠性 | check_handler_test_coverage / check_gsp_rule_test_traceability / check_test_data_isolation |
| 4. 流程治理 | 协作高效性 | task_check / validate_gate_trigger_rules / check_commit_convention |
| 5. 运行治理 | 系统稳定性 | validate_environment / validate_codebase / validate_runtime_schema / check_audit_log_integrity |

**约束**：每个新治理脚本必须明确归属某一类，写在脚本头部注释。**不允许跨类**（跨类一定能拆）。

### 机制 2：4 Tier 执行（When / How fast）

| Tier | 时间预算 | 命令 | 触发时机 | 包含范围 |
|------|---------|------|---------|---------|
| T1 | < 10s | `just quick-check` | 写代码、pre-commit | 格式、lint、提交规范 |
| T2 | < 120s | `just task-check` | 任务完成、提交前 | T1 + diff 触发的最小治理集 + L1 单元测试 + L2 静态契约 |
| T3 | < 5min | `just preflight` | 推送、PR 创建 | T2 + L3/L4/L5/L8/L11 + T3 治理脚本 |
| T4 | < 30min | `just verify` | 合并前、CI、发版 | T3 + L6/L7/L9/L10 + 完整 E2E + 合规追溯 + 性能基线 |

> 各 Tier 包含的测试层（L1-L11）详见 ADR-0006 §3。

**约束**：

- 每个治理脚本必须声明所属 Tier（一个或多个）
- 超出当前 Tier 时间预算的脚本必须降级或拆分
- 时间预算硬性，不允许长期超标（超标即视为脚本设计失败）

### 机制 3：Baseline 机制（务实演进）

每个支持 baseline 的脚本：

- 输出当前违规列表（文件:行 + 规则 ID）
- 与 `governance/baselines/<check_name>.json` 对比
- **新增违规 → fail**
- **已修复违规 → 自动从 baseline 移除**（写回，作为 PR 一部分）
- baseline 文件入库（治理资产，不是临时产物）

**baseline 条目必须包含**：
- `id`：违规位置标识
- `reason`：临时跳过的理由
- `added_at`：加入日期
- 可选 `expires_at`：到期日期（过期视为新违规）

**约束**：
- 不允许"全量加白名单"——baseline 是债务清单，不是免死金牌
- 季度评审：baseline 数量必须**单调下降**，否则触发治理回顾
- 关键不变量（审计 append-only / 密钥不入库 / domain 不依赖 infra）**禁止 baseline**

### 机制 4：diff 触发（最小检查集）

`task_check.py` 根据 `git diff` 文件路径，决定跑哪些脚本：

```toml
# governance/gate-rules.toml 节选（最终形态示意）

[[rules]]
match = "backend/crates/domain/**"
checks = ["check_layer_dependency", "check_unsafe_and_unwrap", "check_handler_test_coverage"]
tier = "T2"

[[rules]]
match = "docs/adr/**"
checks = ["validate_adr_index", "check_doc_links"]
tier = "T1"

[[rules]]
match = "shared/openapi/openapi.json"
checks = ["validate_openapi_artifacts", "check_openapi_contract"]
tier = "T3"
```

**约束**：
- 规则文件入库（`governance/gate-rules.toml`）
- 规则变更必须 PR + 自审
- 必须有"兜底规则"：未匹配的变更默认跑 T1 全套
- T4 不受 diff 触发约束（合并前必须全量）

---

## 治理脚本工程约束

为了让脚本本身不失控，定下统一约束：

1. **路径**：`scripts/governance/<check_name>.py`
2. **语言**：Python 3.10+（标准库优先；外部依赖必须列入仓库根 `requirements-governance.txt`）
3. **头部必填注释**：
   - 用途
   - 所属类别（5 类之一）
   - 所属 Tier（T1-T4）
   - 输入（环境变量 / CLI 参数）
   - 输出（人类可读 + `--json` 机器可读）
   - 退出码语义
4. **退出码**：
   - `0`：通过
   - `1`：发现违规（业务失败）
   - `2`：脚本自身错误（环境缺失、配置错误）
5. **公共逻辑**：放 `scripts/governance/_baseline.py`、`_diff.py`、`_report.py`，禁止脚本间互相 import
6. **必须支持 `--help`**
7. **必须支持 `--json`** 输出（机器可消费）
8. **不修改源码**（只读检查）；如要自动修复，单独命令 `--fix`，需显式调用

---

## 起步阶段（第 0 周）落地范围

只建以下 8 件，**业务相关治理脚本等模块开发时同步建**。

| # | 物件 | 用途 |
|---|------|------|
| 1 | `justfile` | T1-T4 入口（命令可暂时为占位） |
| 2 | `lefthook.yml` | pre-commit / commit-msg / pre-push 三钩子 |
| 3 | `scripts/governance/_baseline.py` | baseline 公共库 |
| 4 | `scripts/governance/_diff.py` | git diff 解析公共库 |
| 5 | `scripts/governance/validate_environment.py` | 工具版本检查（运行治理） |
| 6 | `scripts/governance/check_doc_links.py` | docs 链接（文档治理） |
| 7 | `scripts/governance/validate_adr_index.py` | ADR 编号与索引（文档治理） |
| 8 | `scripts/governance/check_commit_convention.py` | Conventional Commits（流程治理） |
| 9 | `scripts/governance/governance_checks.py` | T1-T4 调度入口 |
| 10 | `scripts/governance/task_check.py` | diff 触发调度（骨架） |
| 11 | `governance/gate-rules.toml` | 门禁规则（骨架） |
| 12 | `governance/baselines/` | baseline 占位 |

（实际是 12 件物件，归为 8 类设计，含公共库与调度。）

---

## 演进路径

| 阶段 | 治理脚本数量上限 | 重点 |
|------|------------------|------|
| 第 0 周 | ~10 | 文档/流程/运行治理骨架 |
| MVP（M1-M3） | ~20 | 加入代码治理（layer、unwrap、modularization） |
| 第二阶段（M4-M6） | ~30 | 加入质量治理（test coverage / GSP traceability） |
| 第三阶段（M7-M12） | ~40 | 加入合规专项（audit / cold-chain / compliance EDI） |

**上限是软指标**：超过上限必须先做"治理脚本本身的整合 / 退役"，再加新脚本。

---

## 后果

### 正面

- **规则即代码**：治理脚本入库、走 PR、有版本，新人 clone 仓库即获完整治理体系
- **反馈最近端**：T1 在写代码时就报错，避免推到 CI 才发现
- **务实接受历史债**：baseline 让"渐进改进"成为可能，不必一次性清零
- **可演进**：5 大类别给新规则归位、4 Tier 给时间预算红线，避免脚本失控膨胀
- **合规可机器化**：GSP 追溯不靠"评审会议"，靠 `validate_gsp_traceability.py`

### 负面

- **额外维护成本**：治理脚本本身是代码，需要测试、文档、版本管理
- **学习成本**：协作者需要理解 5 类 + 4 Tier + baseline 机制
- **过度治理风险**：管理者诱惑——"再加一个脚本"，可能膨胀失控
- **CI 时间膨胀**：T4 接近 30 分钟时，必须做按变更的并行优化

### 风险

- **baseline 滥用**：可能演变为"加白名单逃避修复"；缓解：季度评审 + 关键不变量禁用 baseline
- **脚本本身有 bug**：误报会摧毁开发者信任；缓解：每个脚本必须有自测；脚本变更走 PR
- **diff 规则覆盖不全**：变更类型未匹配规则导致漏检；缓解：兜底规则 + T4 全量
- **治理压过开发**：规则太多导致开发停滞；缓解：4 Tier 时间预算硬上限 + 数量上限 + 季度精简

---

## 实施约束

- **新增治理脚本** = 一次 PR + 必须填写：用途、类别、Tier、是否支持 baseline、是否进入 diff 规则
- **修改 gate-rules.toml** = 必须自审，不允许悄悄改
- **删除/退役治理脚本** = 也要 PR，必须说明原因（被替代 / 价值低 / 已无场景）
- **禁止在业务代码里"绕过治理"**（如加 `# noqa` / `#[allow(...)]` 大范围抑制）；必要的局部抑制必须附 issue 链接 + 到期日

---

## 与 TDD 测试体系的集成

本 ADR 定义"治理执行 Tier T1-T4"。
ADR-0006 定义"测试覆盖维度 L1-L11"。两者**正交**：

- Tier 回答"什么时候跑"（时间维度）
- Layer 回答"跑什么"（覆盖维度）
- 11 层测试被分配到 4 Tier 中执行（详见 ADR-0006 §3）

质量治理类的脚本（5 类中的"质量治理"）**全部**与 ADR-0006 配套：

- 测试目录布局检查
- 各 L 维度的测试覆盖检查
- 性能基线 / 兼容性 schema 历史
- 幂等性 / 可观测性 / 权限矩阵的存在性

具体脚本清单与建立波次见 ADR-0006 §4.1。

---

## 参考

- 治理总文档：`docs/governance.md`
- 技术栈决策：`docs/adr/0001-tech-stack.md`
- 仓库结构决策：`docs/adr/0002-monorepo-structure.md`
- 波次路线决策：`docs/adr/0004-phase-roadmap.md`
- TDD 与 11 层测试：`docs/adr/0006-tdd-and-test-layers.md`
- 架构依赖图：`docs/architecture-dependencies.md`

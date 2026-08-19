# Wave 0 Retrospective

- 日期：2026-05-15
- 范围：治理骨架搭建（从项目创建到首次 T1 全绿 + 钩子端到端验证）

---

## 1. 计划 vs 实际

| 计划 | 实际 | 差异 |
|------|------|------|
| 12 项任务 | 12 项全部完成 + 额外 4 项（依赖图、ADR-0006、ADR-0004 v0.2 重写、review 修复） | 范围扩大但合理 |
| 1 周 | 1 天（密集对话） | 快于预期 |
| 4 份 ADR | 5 份（追加 ADR-0006 TDD） | 用户新增需求 |
| 4 个治理脚本 | 4 个 + 2 个调度 + 2 个公共库 | 按 ADR-0003 设计 |

## 2. 做对了的

1. **先讨论后动手**：技术栈、治理模型、TDD 模式、波次路线全部在写代码前对齐
2. **治理即可执行**：4 个脚本 + 2 个调度 + lefthook 钩子端到端验证通过
3. **ADR 留痕**：所有重大决策（技术栈、仓库结构、治理模型、波次路线、TDD）都有 ADR
4. **依赖图先行**：11 模块 + 3 横向能力的依赖关系在写代码前画清楚
5. **Tier 时间预算健康**：T1=57ms / T2=127ms / T3=793ms / T4=1491ms，远低于上限

## 3. 做错了的 / 需要改进的

1. **估时不诚实**：给了"10 个月"无依据数字（已修复：改为"无法估时"）
2. **gate-rules 占位过多**：违反 YAGNI 精神（保留但标注为占位）
3. **`_diff.py` glob 自实现**：有边界 bug，应在 Wave 1 引入 `pathspec` 库替代
4. **pre-push 逻辑错误**：判断当前分支而非推送目标（已修复）
5. **gitleaks 原为可选**：密钥不入库是红线，不应软化（已修复为必需）
6. **ADR-0005 编号空缺无说明**：（已修复：索引加空缺记录）

## 4. 依赖图是否需要更新

**不需要**。Wave 0 不涉及业务模块，依赖图在 Wave 1 启动前仍有效。

## 5. ADR 状态

| ADR | 状态 | 是否需要修订 |
|-----|------|-------------|
| 0001 技术栈 | Accepted | 否 |
| 0002 仓库结构 | Accepted | 否 |
| 0003 治理模型 | Accepted | 否（已含 T1-T4 重命名 + TDD 集成） |
| 0004 波次路线 | Accepted v0.2 | 否 |
| 0005 Worktree | 预留 | Wave 1 启动前补写 |
| 0006 TDD + 11 层 | Accepted | 否 |

## 6. Baseline 复盘

Wave 0 无业务代码，无 baseline 文件生成。`governance/baselines/` 目录为空（仅 README）。
首个 baseline 预计在 Wave 2 的 `check_handler_test_coverage` 上线时产生。

## 7. Wave 1 准入条件检查

| 条件 | 状态 |
|------|------|
| 所有 Wave 0 ADR = Accepted | ✅ |
| T1 全绿 | ✅ |
| validate_environment 必需工具就绪 | ✅（含 gitleaks） |
| 首次 commit 通过 lefthook 钩子 | ✅ |
| Wave 0 retro 完成 | ✅（本文档） |
| ADR-0005 Worktree 工作流 | ⏸ 预留，Wave 1 启动前补 |

**结论**：Wave 1 准入条件基本满足。ADR-0005 可在 Wave 1 第一个 worktree 创建前补写。

## 8. 下一步

1. 补写 ADR-0005（Git Worktree 工作流）
2. 启动 Wave 1：H1 权限/多租户 + H2 审计追踪 + H3 OpenAPI 契约
3. 同步启动外部资质流程（药监局 / "码上放心"）
4. Wave 1 启动时给 `_baseline.py` / `_diff.py` 加 pytest
5. Wave 1 启动时引入 `pathspec` 替代自实现 glob

## 9. 节奏反思

Wave 0 在一天内完成，密度极高。这不是可持续节奏——后续 Wave 涉及真实业务设计 + TDD 红绿循环，每天产出会大幅下降。

**不要用 Wave 0 的速度作为后续 Wave 的基准**。

---

## 10. Wave 0 持续演进（2026-05-15 ~ 2026-05-18）

> 原 retro 写于首次 T1 全绿之时（2026-05-15）。Wave 0 退出前继续演进 3 天，产出大量补强工作；记录如下。

### 10.1 文档体系扩充

| 文档 | 起始 | 终态 | 变化 |
|---|---|---|---|
| 用户故事文件 | 16 个（早期版本）| 34 个（含拆分） | +112%（按主题拆分 m4/m3/m1/m2 + 新增多个故事） |
| 字段词典 §6 | 70 GSP 字段 | 163 字段（80 GSP + 40 business + 8 system + 15 config + 10 derived + 10 interface） | +133%（v3.1 性质类别扩展）|
| ADR | 5 个（0001-0006） | 6 个 + 1 个 Superseded（0001-0007） | +0007 v0.3 路线边界对齐 |
| 治理脚本 | 6 个（4 起步 + 2 调度） | 14 个 | +8（含 check_gsp_field_traceability / check_user_story_structure / check_glossary / check_approval_source / check_config_center / check_pda_completeness / check_baseline_health / check_governance_consistency / check_story_size）|
| AGENTS.md | 基础引导 | 加风险分级标准 + Git 操作引导 + 必读索引 | 形成"AI 协作宪法" |
| docs/concept-audit.md | 不存在 | 独立文档 | 8 镜头审计结果 + 数据量评估 |
| docs/domain/clarifications.md | 占位 | 47 项业务决策 | 含本次 #47 三项业务取舍 |
| governance.md | §3.1-§3.5 基础 | §3 全段补 commit/PR 拆分/secret 红线 | §3.2 描述/正文/脚注规则、§3.4 commit 粒度（vs PR）、§3.7 误提交后处理 |
| mkdocs 站点 | 未启 | site/ 全部生成 + serve 在 0.0.0.0:8000 | 浏览器可视化文档 |

### 10.2 业务问题清零

按用户故事审计（2026-05-16）发现的 K1-K15 + C1-C10 共 25 项：

- **22 项已修**：K1-K10（编号/角色/订单号/储存条件等）+ K11-K15（批号语义/校验异常/隔离免审/审批集中）+ C1（PDA 离线策略）+ C2（业务数据生命周期归档新故事）+ C4（打印失败通用策略）+ C6（盘点期间出库）+ C7（退货批号 ERP 校验）+ C8/C9/C3（已修复或合理）+ M1-005 客商开票信息补强（与 v3.1 财税链联动）+ 测试要求声明 27 个故事文件全覆盖
- **3 项 P2 后续**：C2 部分（已写归档故事，覆盖 C10）/ C5 冷链断链（二期）/ Wave 4-5 模块扩充（M9 计费、M8 连锁）

### 10.3 治理脚本进步

T1 从最初 8/8 → 现在 14/14。新增脚本针对：
- **故事内容质量**：check_user_story_structure（骨架）+ check_pda_story_completeness（PDA 三件套）+ check_story_size（拆分阈值）
- **字段闭环**：check_gsp_field_traceability（163 字段全覆盖）
- **业务规则一致性**：check_glossary_consistency（禁用词）+ check_approval_source_chain（审批源闭环）+ check_config_center_consistency（双向一致）
- **流程**：check_governance_consistency（治理脚本 ↔ gate-rules.toml 一致）+ check_baseline_health

### 10.4 AGENTS.md 演进

最重要的工程纪律突破：

1. **风险分级标准**：明确"🟢 无风险自主修复 / 🔴 有风险必确认"+ 边界判断方法 + 操作流程
2. **Git 操作引导**：6 条 AI 红线（绝不主动 commit/push/force/改 hooks）+ 5 条主动建议项 + 标准动作流程
3. **本文件即"AI 协作宪法"**：30 秒可读，所有规范都引用到具体文档而不在此重复

### 10.5 v0.3 / v3.1 期间补做了的事

1. legacy CSV 字段对比分析（识别 45 个真实新增概念，非 100+ 虚高）
2. 字段词典从 70 GSP 扩到 163 字段，加 11 项技术属性 + 7 性质类别
3. 故事文件按 20KB / 8 故事 / 15 AC 三阈值拆分（m4 → 3 / m3 → 2 / m1 → 2 / m2 → 2）
4. 跨故事约束统一加 27 个"测试要求"声明
5. K15 审批集中入口在 M-QL §8 落地，解决散落 16 文件 32 次问题
6. v3.1 #47 业务决策（C1/C6/C7）落地 + 入 clarifications.md

### 10.6 Wave 0 准入 Wave 1 状态（v3.1 终态）

| 条件 | 状态 |
|---|---|
| 所有 Wave 0 ADR = Accepted | ✅ |
| T1 14/14 全绿 | ✅ EXIT=0 |
| validate_environment 工具就绪 | ✅ |
| 字段词典 0 error | ✅ 163/163 |
| 故事结构 0 error | ✅ 34/34 |
| 跨文档链接 0 broken | ✅ |
| AGENTS.md 风险分级标准 | ✅ |
| Git 操作引导 | ✅ |
| 业务决策入 clarifications | ✅ #1-#47 |
| 首次 commit | ⏳ **待用户决策**（94 个累积变更需拆分） |

**结论**：v3.1 后 Wave 0 内容质量远超 v0.2 设计预期。**唯一阻塞 Wave 1 启动的事项是首次 commit 的拆分策略**（待用户决策）。

### 10.7 v3.1 节奏反思

- v0.2（2026-05-15）→ v3.1（2026-05-18）跨 4 天
- 期间累积 94 个未提交文件，跨多个主题
- **教训**：早期没建立"原子任务即 commit"的纪律，导致大量变更未提交累积
- **改进**：governance.md §3.4.2 已补"何时建议 commit"规则，AGENTS.md Git 引导明示"长任务结束前必须 commit"

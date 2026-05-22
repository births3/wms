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

## 风险分级标准（无风险自主修，有风险必确认）

> **核心规则**：**无风险的自主改，有风险的必须用户确认**。出现边界模糊时，**默认归入"有风险"**，不擅自决定。

### 🟢 无风险（自主修复，无需确认）

满足以下**全部**条件即为无风险：

1. **不改变业务语义**：修改不影响业务规则、合规边界、状态机迁移
2. **不引入新概念**：没有新 canonical 字段、新故事、新模块、新角色、新状态、新审批源、新枚举值
3. **不影响数据模型**：不增删字段、不改字段类型、不改约束（如 nullable/CHECK）
4. **可被治理脚本验证**：修改后跑 T1（含 `check_gsp_field_traceability` / `check_user_story_structure` / `check_doc_links`）必须 0 error
5. **可逆**：单文件单段改动，git revert 即可撤销

#### 典型场景清单

| 场景 | 例子 |
|----|----|
| **格式 / 编号 / 拼写** | 重复编号修复、错位的 markdown 项、链接错误、错别字 |
| **同义词 alias 补充** | 在已有 canonical 的 aliases 数组追加 legacy 列名（如 WHSE → 仓库 ID）|
| **已存在的事实标准集中说明** | 多个故事已采用同一策略但散落，把它集中到一处 + 其他故事引用（如 K15 审批集中入口） |
| **澄清性注释** | 在已一致的多处加一句注释让语义更清楚（如 K11 "批号校验=ERP 指定批号是否有库存"）|
| **从 H1/H5 等事实参考标准提取通用模式** | 如把 H5-003 打印失败处理提到通用 |
| **治理纪律全局声明** | 加"测试要求"到所有故事跨故事约束 |
| **删除已声明移除的残留** | v23/v25 已 strikethrough 但未清理的引用 |
| **修复脚本 error / 术语违规 / 文件命名** | 治理硬约束 |
| **补充审计追踪条款** | 所有写操作必须的规则，已是治理硬约束 |
| **代码重构不改功能** | rename / 抽函数 / 提取常量 |

### 🔴 有风险（必须用户确认）

满足以下**任一**条件即为有风险：

1. **业务规则取舍**：在多个合规且合理的方案中选择（如 C6 盘点期间出库 A/B/C 三选一）
2. **GSP 合规边界**：影响法规符合性的取舍（如 C7 退货批号保留策略）
3. **新增 canonical 字段**：哪怕字段定义清楚，也属于词典扩展（如 v3.1 新增 10 个 P0 字段需用户拍板）
4. **新增 / 移除 / 拆分故事**：故事本身的存在与否
5. **新增 / 移除 / 改写角色**：M1 角色清单的变更
6. **改状态机**：增删状态、改迁移条件
7. **改字段约束**：data_type / validation / nullable / encryption / audit_required
8. **改审批人 / 审批源 / 触发源**：影响审计追踪与合规
9. **跨模块语义改动**：影响 ≥ 2 个模块的实质性改动（不只是引用）
10. **引入新默认值**：如"离线超时 24 小时"这种数字默认值，没有既存依据
11. **决策有不确定性**：自己不敢拍胸脯说一定对的事
12. **可能误删用户内容**：批量正则替换、跨文件批量删除

#### 典型场景清单

| 场景 | 例子 |
|----|----|
| **业务流程取舍** | 盘点期间出库怎么处理 / 退货批号是否保留 / 离线冲突默认时长 |
| **新故事 / 拆故事** | H2 加数据归档故事 / 拆 US-M4-003 |
| **新字段 canonical** | 加 country_of_origin 等新字段进 §6 词典 |
| **GSP 合规边界** | 是否双人 / 是否审计 / 保留期年限 |
| **新角色 / 新状态** | M1 角色清单变更 / 状态机加新状态 |
| **配置项默认值定义** | 阈值、时长、配额等没明确依据的数字 |
| **跨模块改写** | 同一字段在多个故事的统一改名 |
| **大批量 git 操作** | 跨文件正则替换、reset --hard、push --force |
| **删除任何业务内容** | 哪怕被 strikethrough 也先确认 |

### 边界模糊时的判断方法

按以下顺序应用：

1. **能否被现有治理脚本兜底？** 跑 T1，0 error 才能改
2. **是否有"既存事实标准"作依据？** 多模块已采用同一策略 → 集中说明可自主；只有一处 → 不可推广为标准
3. **是否引入新数字 / 新枚举值？** 引入了 → 有风险（数字需要业务方依据）
4. **撤销成本如何？** 改一个文件 git revert 就能撤销 → 倾向无风险；要改回多个文件甚至数据 → 有风险

### 操作流程

**无风险任务**：
1. 直接改 → 跑治理脚本 → 给用户简短报告（修了什么 + 验证结果）

**有风险任务**：
1. 列出 2-3 个候选方案（含原文引用、利弊、GSP 关联、实施成本）
2. 给推荐 + 推荐理由
3. 等用户决策（默认推荐就回"全 A"）
4. 落实决策后跑治理脚本验证

## Git 操作引导

> **规范本体**：[docs/governance.md §3.1 仓库与分支](docs/governance.md#31) / [§3.2 提交信息](docs/governance.md#32-conventional-commits) / [§3.4 PR 规范](docs/governance.md#34-pr)。
> 本节只写**引导**与**AI 红线**，具体格式/类型/范围/示例去看上述章节。

### AI 在 git 上的红线（任何理由不可违反）

1. **绝不主动 commit**：必须用户明确说"提交"/"commit"/"打个 tag"等动作才能执行 git commit
2. **绝不主动 push**：push 必须用户明确指令；推 main 分支额外要求显式确认
3. **绝不 force push / reset --hard / clean -f / branch -D**：这类破坏性操作必须用户明示
4. **绝不修改 .gitconfig / hooks / 远程配置**：包括 `git config` 任何子命令、改 lefthook.yml 的 hook 行为
5. **绝不在 commit 里包含可疑文件**：`.env` / 私钥 / 密钥 / token 等先警告并停下
6. **绝不 git add .** 当工作区跨主题混杂时：先列出文件让用户确认范围，再分主题分别 stage

### AI 应该主动做的（不需用户确认）

1. **每次改文件后** → 跑治理脚本（T1）验证 → 报告 EXIT 码
2. **完成一个原子任务**（如修一个 bug、改一组关联文件）后 → **建议**用户考虑 commit，但不主动执行
3. **跨主题工作累积** → 在累积变更跨 ≥ 2 个主题或 ≥ 5 个文件时，**主动建议**用户拆分多个 commit
4. **看到未提交变更 > 400 行** → 主动提示"超过 PR 规范 400 行上限，建议拆分"
5. **每次会话开始** → 跑 `git status` 看是否有遗留未提交变更

### 用户说"提交"时 AI 的标准动作

按以下顺序执行：

1. 跑 `git status` 确认改动范围
2. 跑 `git diff --stat` 给用户看本次提交涉及哪些文件 + 行数
3. **如果跨多主题或 > 400 行** → 提示拆分建议，等用户确认怎么拆
4. **如果在单一主题且 < 400 行** → 起草符合 [docs/governance.md §3.2](docs/governance.md#32-conventional-commits) 的 commit message（中文 type + scope + subject + 正文 + 关联）
5. **展示 commit message 草稿给用户审阅**，等"确认"后再执行 `git commit`
6. 跑 lefthook pre-commit / commit-msg 钩子，把脚本输出贴回给用户
7. **不主动 push**，除非用户额外说"推上去"

### 用户说"推送 / push"时 AI 的标准动作

1. 确认目标分支（默认推当前分支到 origin）
2. 如果是 main 分支 → **强制要求**用户显式确认（governance.md §3.1 禁止直推 main）
3. 推送前提示运行 pre-push 钩子（T3 preflight）
4. 推送后报告结果，并提示是否需要 PR

### 提交信息起草要点（速查）

- **格式**：`<类型>(<范围>)：<描述>` — 中文版（governance.md §3.2 完整定义）
- **类型**：功能 / 修复 / 文档 / 格式 / 重构 / 性能 / 测试 / 构建 / 集成 / 杂项 / 回滚（11 类）
- **范围**：基础档案 / 入库 / 库存 / 出库 / 质量 / 冷链 / 计费 / 合规 / 审计 / pda / 管理端 / 接口 / 基础设施 / 治理 / 文档 / 追溯码 / 对账 / 药检 / 校验 / 质量联系单 / 企微 / 快递（22 个）
- **描述**：≤ 50 字，动词开头，不带句号
- **正文**：动机 + 改动概要 + 影响范围（可空，复杂改动必填）
- **脚注**：`关联：US-X-NNN, ADR-NNNN`、`破坏性变更：<说明>`、`Co-authored-by:` 等

## AI 协作产出审查标准

> 把 AI 当**实习生 + 速查工具**，不是把它当**架构师**。
> 关键原则：**AI 输出必须可被人类验证**，不能盲信。

### 9 条审查清单（每次 AI 产出后逐条勾选）

1. **数字有出处**：所有"12 个维度"、"50 个错误码"、"4.84 亿行"等数字必须能追到证据（脚本输出 / 文件读取 / 字典 §6 行数）
2. **决策有候选方案**：A/B/C 至少给 2 个候选，不能直接说"我推荐 X"
3. **GSP 引用具体到条款**：不能只说"GSP 合规"，必须 §5.67 / §7.103 这种具体编号
4. **风险有应对**：识别风险时必须给应对方案，不能停在"有风险"
5. **代码 / 字段 / 故事修改可回滚**：不能批量正则替换 + 不验证；每次改文件都跑治理脚本
6. **诚实标注未验证内容**：AI 推断的结论必须明示"推断 / 未验证"前缀
7. **不冒充权威**：法规条款解释 / 业务方决策 / 安全审计 → AI 给参考意见，不下定论
8. **跨文档一致性**：改 ADR 必同步 ADR README / AGENTS.md 索引；改字段词典必同步治理脚本
9. **每次会话末做哲学自检**：苏格拉底（真理解了吗）+ 斯多葛（必要 vs 多余）+ 经验主义（每个断言有证据吗）

### AI 产出反模式（红线）

| 反模式 | 例子 | 危害 |
|---|---|---|
| **数字虚高** | "约 100-145 字段"（实际 45）| 决策被误导 |
| **过度抽象** | 把简单 if-else 写成 12 层 trait | 维护成本爆炸 |
| **照搬不思考** | 直接复制 odoo Python 代码到 Rust | 语言特性不对 |
| **概念漂移** | "审批源" 在不同地方指不同概念 | 业务方对不上 |
| **忽略治理脚本** | 改字段不跑 check_gsp_field_traceability | 一致性断链 |
| **未读先答** | 不读 ADR-0008 就回答相关问题 | 给错答案 |
| **跨主题混提交** | 一次 commit 改 5 个无关主题 | review 困难 |

### PR 评审重点（针对 AI 产出）

人类 reviewer 重点检查：

1. **语义合理性** > 语法正确性（AI 写的代码语法基本对，但语义可能错）
2. **业务规则**：AI 是否真理解业务流程？还是机械堆 keyword？
3. **GSP 合规边界**：AI 容易在合规边界含糊（如"或可"、"可能需要"）
4. **测试是否真验证业务**：AI 写的测试可能仅 happy path，错误路径常缺
5. **是否漏了相关文档**：AI 改 H1 时容易漏改 H2-005（事件总线依赖）

### 何时不该用 AI

- **法规解释**（GSP 具体条款的法律含义）
- **架构决策最终拍板**（AI 给候选，人决策）
- **生产环境操作**（删数据 / 改 prod schema）
- **业务方未确认的取舍**（A/B/C 哪个对，业务方说了算）
- **安全 / 合规审计的最终结论**

## 修复 / 审计优先级（脚本第一，语义第二）

> 核心纪律：**能用脚本自动检查的问题，永远先于需要人工语义理解的问题修复**。
> 目的：把容易自动化的问题在 commit 前消灭干净；让人工审计聚焦在真正需要语义判断的地方，不被脚本噪音淹没。

### 三级优先级

| 级别 | 修复对象 | 验证方式 | 典型例子 |
|------|---------|---------|---------|
| **P0 最高** | 已有脚本可检查的违规 | `just gov-t1` 退出码 / warning 数 | 编号重复 / 模糊词 / 角色白名单 / 术语黑名单 / 文件命名 |
| **P1 次高** | 当前脚本未覆盖但**可以脚本化**的问题 | 先扩展脚本到能检查，再修复 | 跨文件引用失效 / 审批源链路 / 配置中心列表与默认值一致性 / 状态机闭合性 |
| **P2 最后** | 必须人工语义判断的问题 | 人工 review，记入 review 报告 | GSP 法规边界 / 业务流程合理性 / 跨模块语义不一致的取舍 |

### 工作流

```
发现问题
    ↓
能用现有脚本检查吗？
  ├─ 是 → P0：直接修复，跑 just gov-t1 验证
  └─ 否 → 能写脚本检查吗？
           ├─ 是 → P1：先扩展脚本（加到 governance_checks），再修复
           └─ 否 → P2：写入 review 报告，人工决策
```

### 实践要点

1. **遇到批量同类问题先想"是否可脚本化"**：例如 v3 后审计发现的"审批源链路"问题（M-QL/M-SA/M2-003 调用 M3 状态变更时未声明 approval_source），如果有 5 处以上，应该先写一个 `check_approval_source_chain.py` 脚本，再批量修复。
2. **P0 修复必须跑 T1 验证**：不能只看修改本身，要看脚本最终 0 error / 0 warning。
3. **P1 升 P0 是常态**：审计中发现的语义检查只要有规律，应该尽快脚本化。`check_user_story_structure.py` 就是从"模糊词/角色/审计/幂等"这些规律演化来的。
4. **P2 不可省略，但要稀缺**：人工审计时间贵，只用在真正需要业务/法规判断的地方。
5. **脚本误报先修脚本，不修文档**：v3 阶段把"幂等性缺失"的 77 个 warning 通过让脚本识别"跨故事约束兜底模式"消除，比让 77 个故事各加一行幂等声明更整洁。

### 何时新增检查脚本

| 触发条件 | 行动 |
|---------|------|
| 同一类问题在 ≥3 个文件出现 | 写脚本 |
| 单次审计发现的语义检查可表达为正则/AST/字符串规则 | 写脚本（即使只用一次） |
| 脚本发现"已知误报" 持续存在 | 修脚本（让脚本理解约定模式），不修文档 |
| 跨文件一致性需要反复人工对照 | 写脚本（如引用有效性、配置项一致性） |

> 推论：审计报告中的"建议修复"清单应该按这三级排序展示，不按问题严重度排序。

## 必读文档（按优先级）

1. [docs/coding-standards.md](docs/coding-standards.md) — 代码书写规范（Rust / TS / 跨端 / 禁止清单）
2. [docs/frontend-coding-standards.md](docs/frontend-coding-standards.md) — **前端编码规范**（项目结构 / 命名 / 组件接口 / Tailwind 风格 / PDA 触控基线 / 4 个治理脚本 / PR 自查清单）
3. [docs/governance.md](docs/governance.md) — 治理体系（5 类 + 4 Tier + Baseline + 文档四层管理）
4. [docs/adr/0006-tdd-and-test-layers.md](docs/adr/0006-tdd-and-test-layers.md) — TDD + 11 层测试
5. [docs/adr/0021-high-fidelity-prototype-strategy.md](docs/adr/0021-high-fidelity-prototype-strategy.md) — 高保真原型策略（shadcn/ui + Storybook 工具链）
6. [docs/adr/0022-prototype-component-spec.md](docs/adr/0022-prototype-component-spec.md) — 原型组件规范（三层架构 + cva + forwardRef + 文档头）
7. [docs/architecture-dependencies.md](docs/architecture-dependencies.md) — 模块依赖图（当前模块清单 + 5 波次）
8. [docs/adr/README.md](docs/adr/README.md) — 所有架构决策索引
9. [docs/infra/technical-specs.md](docs/infra/technical-specs.md) — 基础设施技术规格（H6 状态机 / H7 导入导出 / H8 ERP 防腐层 / H9 打印 / H10 备份恢复）
10. [docs/concept-audit.md](docs/concept-audit.md) — 概念审计报告（8 镜头扫描结果 + 数据量评估）
11. [docs/domain/clarifications.md](docs/domain/clarifications.md) — 业务澄清记录（42 项决策）
12. [docs/glossary.md](docs/glossary.md) — 术语表（54 个，含禁用词）

## 业务文档索引

| 文档 | 用途 |
|------|------|
| [docs/domain/user-stories-m1-master-data-product.md](docs/domain/user-stories-m1-master-data-product.md) | M1 基础档案（10 个故事） |
| [docs/domain/user-stories-m2-inbound-asn.md](docs/domain/user-stories-m2-inbound-asn.md) | M2 入库（10 个故事） |
| [docs/domain/user-stories-m3-inventory-query.md](docs/domain/user-stories-m3-inventory-query.md) | M3 库存（11 个故事） |
| [docs/domain/user-stories-m4-outbound-order.md](docs/domain/user-stories-m4-outbound-order.md) | M4 出库（11 个故事） |
| [docs/domain/user-stories-m5-cold-chain.md](docs/domain/user-stories-m5-cold-chain.md) | M5 冷链数据集成（3 个故事，**温控由外部冷链系统采集**） |
| [docs/domain/user-stories-m6-audit-report.md](docs/domain/user-stories-m6-audit-report.md) | M6 报表（3 个故事） |
| [docs/domain/user-stories-m8-retail-chain.md](docs/domain/user-stories-m8-retail-chain.md) | M8 连锁（2 个故事） |
| [docs/domain/user-stories-m9-billing.md](docs/domain/user-stories-m9-billing.md) | M9 计费（3 个故事） |
| [docs/domain/user-stories-m10-tms-plus.md](docs/domain/user-stories-m10-tms-plus.md) | M10 TMS+（4 个故事） |
| [docs/domain/user-stories-m11-regulatory-edi.md](docs/domain/user-stories-m11-regulatory-edi.md) | ~~M11 监管 EDI~~（v7 移除：码上放心迁 M-TC，药监 EDI 由 ERP 做）|
| [docs/domain/user-stories-mte-task-engine.md](docs/domain/user-stories-mte-task-engine.md) | M-TE 任务引擎（11 个故事） |
| [docs/domain/user-stories-mrp-replenishment.md](docs/domain/user-stories-mrp-replenishment.md) | M-RP 补货（5 个故事） |
| [docs/domain/user-stories-mpk-packing-station.md](docs/domain/user-stories-mpk-packing-station.md) | M-PK 包装站（6 个故事） |
| [docs/domain/user-stories-mvr-validation-rules.md](docs/domain/user-stories-mvr-validation-rules.md) | M-VR 规则引擎（5 个故事） |
| [docs/domain/user-stories-mtc-traceability-code.md](docs/domain/user-stories-mtc-traceability-code.md) | M-TC 追溯码（7 个故事） |
| [docs/domain/user-stories-mql-quality-liaison.md](docs/domain/user-stories-mql-quality-liaison.md) | M-QL 质量联系单（5 个故事） |
| [docs/domain/user-stories-mcg-code-generator.md](docs/domain/user-stories-mcg-code-generator.md) | M-CG 编码生成（2 个故事） |
| [docs/domain/user-stories-msa-stock-adjustment.md](docs/domain/user-stories-msa-stock-adjustment.md) | M-SA 报损报溢（3 个故事） |
| [docs/domain/user-stories-mba-batch-adjustment.md](docs/domain/user-stories-mba-batch-adjustment.md) | M-BA 批号调整（4 个故事，v21 库内业务）|
| [docs/domain/user-stories-mrc-reconciliation.md](docs/domain/user-stories-mrc-reconciliation.md) | M-RC 对账（4 个故事） |
| [docs/domain/user-stories-mdi-drug-inspection.md](docs/domain/user-stories-mdi-drug-inspection.md) | M-DI 药检单（4 个故事） |
| [docs/domain/user-stories-h4-wechat-notify.md](docs/domain/user-stories-h4-wechat-notify.md) | H4 企业微信（4 个故事） |
| [docs/domain/user-stories-h5-express.md](docs/domain/user-stories-h5-express.md) | H5 快递（5 个故事） |
| [docs/domain/user-stories-h-driver.md](docs/domain/user-stories-h-driver.md) | H-Driver 司机端（5 个故事，v15 W4.E）|
| [docs/domain/user-stories-h-store.md](docs/domain/user-stories-h-store.md) | H-Store 门店用户端（6 个故事，v15 W4.E）|
| [docs/domain/user-stories-h1-auth-tenant.md](docs/domain/user-stories-h1-auth-tenant.md) | H1 权限与多租户（6 故事，v18 Wave 1 W1.A）|
| [docs/domain/user-stories-h2-audit-trail.md](docs/domain/user-stories-h2-audit-trail.md) | H2 审计追踪 + 事件总线（6 故事，v18 Wave 1 W1.B；H2-005 升级为 H-EVT）|
| [docs/domain/user-stories-h3-contract.md](docs/domain/user-stories-h3-contract.md) | H3 跨端契约 OpenAPI（4 故事，v18 Wave 1 W1.C）|
| [docs/domain/user-stories-h-dock-management.md](docs/domain/user-stories-h-dock-management.md) | H-DOCK 月台预约管理（7 故事，v3.1，可启用开关，3PL/冷链优先仓启用）|
| [docs/domain/user-stories-h-alert.md](docs/domain/user-stories-h-alert.md) | H-AL 告警引擎（5 故事，v3.1，GSP 5.71 触发响应时间合规）|

## 其他文档索引

| 文档 | 用途 |
|------|------|
| [docs/adr/0001-tech-stack.md](docs/adr/0001-tech-stack.md) | 技术栈选型决策 |
| [docs/adr/0002-monorepo-structure.md](docs/adr/0002-monorepo-structure.md) | 仓库结构决策 |
| [docs/adr/0003-governance-model.md](docs/adr/0003-governance-model.md) | 治理模型决策 |
| [docs/adr/0004-phase-roadmap.md](docs/adr/0004-phase-roadmap.md) | 波次路线旧决策（已被 ADR-0007 取代） |
| [docs/adr/0007-roadmap-v03-boundary-alignment.md](docs/adr/0007-roadmap-v03-boundary-alignment.md) | 当前波次路线与边界对齐决策 |
| [docs/adr/0008-borrow-from-odoo.md](docs/adr/0008-borrow-from-odoo.md) | 借鉴 Odoo 的 9 个设计（mail.thread / stock.move / ir.sequence / manifest / GS1 + ir.rule / access.csv / TransientModel / state button），实施 Wave 0-4 |
| [docs/adr/0010-error-codes.md](docs/adr/0010-error-codes.md) | **错误码体系**（三段式 + 4 级严重度 + 50 错误码字典 + 治理脚本）|
| [docs/adr/0011-observability.md](docs/adr/0011-observability.md) | **可观测性方案**（OpenTelemetry + Prometheus + Loki + Grafana + KPI 清单 + SLO 告警）|
| [docs/adr/0012-bounded-contexts.md](docs/adr/0012-bounded-contexts.md) | **限界上下文与 Context Map**（24 BC + 8 种 DDD 集成模式 + 9 类 Shared Kernel）|
| [docs/adr/0013-config-secrets.md](docs/adr/0013-config-secrets.md) | **配置与 secrets 管理**（三层配置 + Vault + 90 天密钥轮换）|
| [docs/adr/0014-data-migration.md](docs/adr/0014-data-migration.md) | **数据迁移策略**（CDC + 双写 + 货主级灰度 + 4 维校验）|
| [docs/adr/0015-multi-end-rules.md](docs/adr/0015-multi-end-rules.md) | **多端业务规则放置**（A/B/C 三级 + OpenAPI 共享 schema）|
| [docs/adr/0016-deployment.md](docs/adr/0016-deployment.md) | **部署形态**（docker-compose / k8s 双轨 + Migration 4 步走）|
| [docs/error-codes.md](docs/error-codes.md) | **错误码字典**（v3.1 初版 50 项，单一事实之源）|
| [docs/retros/wave-0-retro.md](docs/retros/wave-0-retro.md) | Wave 0 回顾 |
| [docs/reviews/user-stories-audit-2026-05-16.md](docs/reviews/user-stories-audit-2026-05-16.md) | 用户故事 5 维度审计（116 故事 / 22 模块）|
| [docs/reviews/software-design-audit-2026-05-18.md](docs/reviews/software-design-audit-2026-05-18.md) | **软件设计 12 维度审计**（v3.1，识别 6 P0 / 4 P1 / 3 P2 共 13 项缺口；指引 Wave 1 启动前需补 5+ ADR）|
| [docs/compliance/README.md](docs/compliance/README.md) | **GSP 合规追溯矩阵总索引**（v11 建立，按章节拆分追溯条款 → 用户故事）|
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

### 前端组件红线（详见 [frontend-coding-standards.md](docs/frontend-coding-standards.md)）

- 三层架构：`components/ui/`（shadcn 原子）/ `components/business/`（业务复合）/ `pages/`（页面级）；依赖方向不可反向
- 业务复合组件必须 `React.forwardRef` + 继承 `HTMLAttributes` + `cn()` 合并 className + `displayName`
- 业务复合组件**禁止静态 inline style**（动态值用 `// 动态：理由` 注释豁免）
- 颜色用 CSS 变量（`bg-primary`），不直写 hex；间距/字号/圆角用 tailwind token
- 顶部文档头 5 项强制：用途 / 层级 / 关联故事 / Wave / @example
- size 三档对齐 shadcn：`sm | default | lg`（禁止 md）
- 状态枚举必须对齐 `docs/prototypes/component-registry.md §4.3`
- PDA 端组件触控 ≥ 48pt / 字号 ≥ 16pt（usability-baseline §2.1）
- **单页面 `.tsx` < 300 行**（≥ 300 警告，≥ 500 PR 门禁，加 `@governance: skip-page-size` 注释豁免）
- **流程类组件按决策树选型**（StepFlow 通用进度 / AuditTimeline 历史事件 / ApprovalFlow 审批 / DualSignPanel 双人签字特例）
- 新增 Layer 2 组件 PR 必须在 component-registry.md §3.1 注册

### 前端治理脚本（T1 自动跑）

| 脚本 | 校验 |
|---|---|
| `check_component_doc_header.py` | 顶部文档头 5 项字段齐全 |
| `check_component_no_inline_style.py` | 业务复合无静态 inline style（动态值豁免） |
| `check_component_props_classname.py` | Props 接口含 className + forwardRef + displayName |
| `check_component_registry_consistency.py` | 注册表 ↔ 实际目录一致（区分"已开发"/"待开发"） |
| `check_page_size.py` | 页面 < 300 行通过 / 300-499 警告 / ≥ 500 门禁 |
| `check_prototype_index_consistency.py` | 原型 index.toml 字段合法 |
| `check_prototype_story_sync.py` | 原型 ↔ 故事文件同步 |
| `check_prototype_freshness.py` | 原型走查时效（90 天） |
| `check_prototype_usability_baseline.py` | PDA 触控/字号基线 |
| `check_prototype_review_signoff.py` | 走查签字（T2，PR 阶段） |

## 当前阶段

Wave 0 治理骨架收尾中。完成 TODO.md 的退出条件后进入 Wave 1 横向底座（H1 权限 / H2 审计 / H3 OpenAPI）。

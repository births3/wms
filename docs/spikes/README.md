# 技术 Spike — 索引与流程

> Spike 是**带时间盒的技术验证**，用于在 Wave 1 启动业务实现之前，把高风险/未确定的技术问题暴露并定型。
> 不是产品代码、不入主干业务；产出是**结论 + 决策**（必要时升级为 ADR）。

---

## 1. 当前 Wave 0.5 Spike 清单（5 项）

| # | ID | 标题 | 关联 Wave 任务 | 状态 | 时间盒 | 关联 ADR |
|---|-----|------|-------------|------|------|---------|
| 1 | [SPIKE-001](spike-001-axum-jwt.md) | Axum + JWT + 多租户 middleware | W1.A 权限/多租户 | **accepted** | 2 天（用 ~3h） | ADR-0001 / 产出 ADR-0024（Proposed） |
| 2 | [SPIKE-002](spike-002-h2-append-only.md) | PostgreSQL append-only 审计 | W1.B 审计追踪 | **accepted** | 2 天（用 ~2h） | ADR-0001 / 产出 ADR-0025（Proposed） |
| 3 | [SPIKE-003](spike-003-utoipa-openapi-ts-pipeline.md) | utoipa → OpenAPI → TS 全链路 | W1.C OpenAPI 工具链 | **accepted** | 1.5 天（用 ~4h） | ADR-0001 / 产出 ADR-0026（Proposed） |
| 4 | [SPIKE-004](spike-004-sqlx-offline.md) | SQLx offline 编译模式 | 跨 W1.A/B（基础设施） | **accepted** | 1 天（用 ~1.5h） | ADR-0001 §SQLx 附录（已更新 v0.2） |
| 5 | [SPIKE-005](spike-005-rn-scanner.md) | RN 扫枪 + 离线队列 | Wave 1 PDA 离线 / W3.A | 起草 | 2 天 | ADR-0015 / 拟产出 ADR-0027 |

合计上限：8.5 天。**任何 Spike 超出时间盒 50% 必须 escalate**：要么承认走错路（reject + 重新设计），要么承认低估（写新 spike 接力，原 spike 关闭）。不允许"再多花 1 天试试"无限延期。

---

## 2. Spike 与 ADR 的关系

```
Spike（带时间盒的验证） → 结论
   ├─ accept   → 升级成 ADR（如本目录尚无对应 ADR）或在现有 ADR 加附录
   ├─ reject   → 候选作废，写明拒绝理由；如阻塞 Wave 计划须新建 spike
   └─ defer    → 当前 Wave 不做，记入 ROADMAP backlog 并指明启动条件
```

- Spike 文档**永久保留**在 `docs/spikes/`（即使被 ADR 取代）—— 它是决策的史料证据
- 决策一旦升级为 ADR，Spike 文档**只追加**"决策记录"段引用 ADR，不删原文
- Spike 失败（reject）也保留：避免后人重复试错

---

## 3. Spike 文档结构（强制）

每个 `spike-NNN-<slug>.md` 必须包含以下小节，缺一不可：

```markdown
# SPIKE-NNN: <标题>

- 状态：起草 / 进行中 / accepted / rejected / deferred
- 时间盒：<N 天 / N 小时>
- Owner：<姓名 / 工号>
- 起始：<日期>  完成：<日期>
- 关联 Wave 任务：<W1.A / W1.B / ...>
- 关联 ADR：<已有 ADR 编号；如需产出新 ADR 写"拟产出 ADR-XXXX"​>

## 1. 背景与问题
为什么要做这个 spike？不做会带来什么风险？

## 2. 验证假设
列出 H1/H2/H3 ...，每条必须可被一个具体实验证伪或确认。
不允许"看起来挺好用"这种不可证伪的假设。

## 3. 退出条件
| 状态 | 条件 |
| --- | --- |
| accept | 全部 H 假设确认，且不引入超出时间盒的额外 spike |
| reject | 任一 H 假设被确凿反例推翻，或方案存在硬阻塞 |
| defer | Wave 0.5 范围外的子问题；明确转入哪个 backlog |

## 4. 实施路径
分步骤；每步说明产出（代码位置 / 测试命令 / 配置文件）。
代码 spike 默认放 `spikes/<spike-id>/` 目录（与生产代码隔离）。

## 5. 风险与后备方案
列已知风险 + 如果 reject 之后用什么备选（必须能落地，不能写"再讨论"）。

## 6. 产出物清单
- 代码：spikes/<spike-id>/
- 决策：本文件 §7
- ADR（如需）：docs/adr/XXXX-<title>.md

## 7. 决策记录（spike 完成后填）
日期：__
结论：accept / reject / defer
关键发现：……
后续动作：……
```

---

## 4. 不允许的反模式

- ❌ "Spike 完不成所以扩到 Wave 1" —— 时间盒就是为了避免无限拖延，扩大要走 escalate
- ❌ "Spike 代码进主干业务" —— spike 代码隔离在 `spikes/<id>/`，主干业务从零写（spike 是经验，不是骨架）
- ❌ "Spike 跳过 ADR 直接开始 Wave 1" —— 任何被 accept 的 Spike 必须在 Wave 1 启动前完成 ADR 或 ADR 附录
- ❌ "口头 accept 不写决策" —— 决策记录是产出物的一部分，否则视作未完成

---

## 5. Spike 触发新 Spike 时的处理

如果某个 spike 验证过程中发现需要再开一个子 spike：

1. 当前 spike 标记 `defer` 或 `accept-with-followup`
2. 新建 `spike-NNN+1-<slug>.md`，在 §1 背景里链接父 spike
3. 父 spike §7 决策记录中明确指向子 spike

避免把多个不同问题塞进一个 spike，导致结论混淆。

---

## 6. Wave 0.5 退出条件中的 Spike 项

按 [ROADMAP.md](../../ROADMAP.md) Wave 0.5：

> **完成标准**：Storybook 可运行；P0 原型 ≥1 次走查 approved；Spike 结论记录到 `docs/spikes/`。

"Spike 结论记录到 `docs/spikes/`" 的具体判据：

- [ ] 5 个 Spike 全部进入 accepted / rejected / deferred 三态之一（不能停在"起草"或"进行中"）
- [ ] 每个 accepted 的 Spike 都有对应 ADR（新建或附录）
- [ ] 任一 rejected 都明确写出后备方案在哪个 spike / 哪个 ADR
- [ ] 任一 deferred 都已写入 ROADMAP backlog 并附启动条件

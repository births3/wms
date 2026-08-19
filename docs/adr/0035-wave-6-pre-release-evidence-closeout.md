# ADR-0035：Wave 6 预发布证据与外部依赖收口

- 状态：Accepted
- 决策日期：2026-06-04
- 决策人：项目主人
- 起草人：AI 助手
- 关联：ADR-0007 / ADR-0016 / ADR-0025 / ADR-0030 / ROADMAP.md / TODO.md / docs/runbooks/wave-1-runtime-evidence.md / docs/runbooks/wave-2-runtime-evidence.md / docs/runbooks/wave-3-pda-readiness.md / docs/runbooks/wave-4-external-dependencies.md / docs/runbooks/wave-5-hardware-evidence.md / docs/runbooks/wave-5-tms-evidence.md / docs/runbooks/wave-6-deploy-evidence.md / docs/runbooks/wave-6-evidence-preflight.md / docs/runbooks/wave-6-closeout.md

---

## 背景

原 ROADMAP 口径为 Wave 1-5 覆盖开发阶段：

- Wave 1 横向底座。
- Wave 2 业务底座 + schema。
- Wave 3 核心业务规则。
- Wave 4 完整闭环。
- Wave 5 增值模块全面铺开。

截至 2026-06-04，Wave 1-5 开发完成门禁已能分别用对应 completion check 证明。但多个真实环境证据按已确认口径后移为预发布 gate，包括：

- Wave 1 H2 真实 dev PostgreSQL 60M baseline + wrk 1k QPS × 1 小时 + 7 天封档。
- Wave 1 W1.D 真实 dev/staging 自动回滚。
- Wave 2 配置中心版 Feature Flag 真实 dev/staging runtime evidence。
- Wave 3 真 PDA + L7 性能/易用性证据。
- Wave 4 M-TC “码上放心”真实 dev/staging evidence。
- Wave 5 M-PK 电子秤 / 蓝牙打印机 / 面单打印真实硬件 evidence。
- Wave 5 M10 TMS+ 真实 dev/staging 推送、回调、失败重试和 audit_event 查询 evidence。
- 首次试运行投产必须使用 ADR-0016 灰度发布链路。

若继续把这些散落为“后续跟踪”，容易出现两类风险：

1. 误把开发完成等同于可预发布。
2. 在没有稳定 dev/staging 或外部系统时，用 localhost / stub / mock / fake / example / prod / production 证据替代真实证据。

用户已确认接受建议：下一步不新增业务功能波次，先把 Wave 5 closeout 做完，并把下一阶段定义为 Wave 6 预发布证据 / 外部依赖收口。

---

## 候选方案

### A. Wave 6 = 预发布 runtime evidence / 外部依赖收口（推荐）

把 Wave 6 定义为开发后、试运行前的证据收口波次。范围只包括真实环境、真实硬件、真实外部系统、灰度发布和证据治理。

优点：
- 对齐当前最大风险：不是代码功能缺口，而是真实运行证据缺口。
- 不打破 Wave 1-5 已完成的开发边界。
- 保留“禁止伪造 evidence”的硬约束。
- 能把分散 gate 收敛到一个当前 TODO 和一个出口检查。

缺点：
- Wave 6 依赖外部状态，可能等待 dev/staging、硬件、TMS、码上放心账号。
- 不产生明显新业务功能，进度表现不如功能开发直观。

### B. Wave 6 = v26 GSP 字段命名规范化

把 v26 backlog 中的字段 canonical / alias 规范化作为 Wave 6。

优点：
- 范围清晰，可本地完成。
- 可降低后续 schema 严格化成本。

缺点：
- 不是当前最大上线风险。
- 会把真实 runtime evidence 继续后移，容易形成“功能越写越多、预发布越不可证”的状态。

### C. Wave 6 = 新业务功能波次

继续追加新模块或高级功能。

优点：
- 产品功能继续扩展。

缺点：
- 违反“每波都必须可上线”的节奏铁律。
- 会在已有预发布 gate 未关闭时增加新不确定性。

---

## 决策

采用方案 A：**Wave 6 定义为预发布 runtime evidence 与外部依赖收口波次**。

Wave 6 不新增业务模块，不扩展用户故事范围，不把 v26 字段规范化默认纳入当前波次。Wave 6 的目标是把 Wave 1-5 已开发能力推进到可预发布 / 可试运行的证据状态。

### Wave 6 范围

| 编号 | Gate | 完成判据 |
|------|------|---------|
| W6.A | Wave 1 H2 runtime evidence | `docs/retros/wave-1-h2-runtime-evidence.json` 通过 `just wave-1-runtime-evidence-validate` |
| W6.B | Wave 1 W1.D 自动回滚 evidence | `docs/retros/wave-1-runtime-evidence.json` 通过 `just wave-1-runtime-evidence-validate` |
| W6.C | Wave 2 配置中心 Feature Flag evidence | `docs/retros/wave-2-runtime-evidence.json` 通过 `just wave-2-runtime-evidence-validate` |
| W6.D | Wave 3 真 PDA / L7 evidence | `docs/retros/wave-3-pda-runtime-evidence.json` 通过 `just wave-3-pda-runtime-evidence-validate`，证据按 [Wave 3 PDA Readiness Runbook](../runbooks/wave-3-pda-readiness.md) 采集 |
| W6.E | Wave 4 M-TC “码上放心”外部 evidence | `docs/retros/wave-4-external-dependencies.json` 通过 `just wave-4-external-dependencies-validate` |
| W6.F | Wave 5 M-PK 硬件 evidence | `docs/retros/wave-5-hardware-evidence.json` 通过 `just wave-5-hardware-evidence-validate`，证据按 [Wave 5 Hardware Evidence Runbook](../runbooks/wave-5-hardware-evidence.md) 采集 |
| W6.G | Wave 5 M10 TMS+ evidence | `docs/retros/wave-5-tms-evidence.json` 通过 `just wave-5-tms-evidence-validate`，证据按 [Wave 5 TMS+ Evidence Runbook](../runbooks/wave-5-tms-evidence.md) 采集 |
| W6.H | 首次试运行发布 evidence | `docs/retros/wave-6-deploy-evidence.json` 通过 `just wave-6-deploy-evidence-validate`，证据按 [Wave 6 Gray Release Evidence Runbook](../runbooks/wave-6-deploy-evidence.md) 采集 |

### 非范围

- 不新增业务模块。
- 不启动 v26 GSP 字段命名规范化，除非用户另行确认。
- 不补 i18n，仍按 ROADMAP 的 i18n 启动条件。
- 不用本地 PostgreSQL、localhost、stub、mock、fake、example、prod、production 替代真实 dev/staging 或外部系统证据。
- 每个 evidence 引用必须包含当前 `environment` 标记（`dev` 或 `staging`）；缺少环境标记的 CI、Vault、工单、日志或证据库引用不能关闭 gate。

---

## 后果

- 正面：
  - 上线前风险被集中管理。
  - 开发完成和预发布 ready 的边界清晰。
  - 外部依赖等待不再污染业务功能完成判定。
- 负面：
  - Wave 6 可能被外部系统、硬件采购、dev/staging 稳定性阻塞。
  - 没有真实环境时，Wave 6 只能推进 runbook / validator / 证据格式，不能关闭 gate。
- 风险：
  - 如果为了“完成 Wave 6”降低证据标准，会破坏前序所有 gate 的可信度。
  - 若未分组提交 Wave 4 / Wave 5 变更，Wave 6 PR review 会被历史改动噪音污染。

---

## 实施约束

1. Wave 6 启动前必须完成 Wave 5 closeout：TODO 归档、retro 落地、completion check 通过。
2. Wave 6 只收口 evidence，不新增业务故事。
3. 所有 secret / webhook / 外部凭证只允许放在环境变量、Vault 或运行环境 secret，禁止入仓。
4. 所有 evidence 引用必须指向真实 dev/staging、真实硬件或真实外部系统日志 / 工单 / 对账记录，且包含当前 `environment` 标记（`dev` 或 `staging`），禁止 local/mock/fake/stub/example/prod/production。
5. 已有 validator 是完成判据的单一事实源；缺 validator 的 gate，先补 runbook / validator，再关闭证据。
6. 首次试运行投产必须使用 ADR-0016 灰度发布策略链路，不允许全量直发。
7. 真实环境尚未到位时，只允许执行 `just wave-6-evidence-preflight` 这类静态预检；preflight 不写 evidence，也不能关闭 gate。
8. Wave 6 结束前必须写 `docs/retros/wave-6-retro.md`。

---

## 参考

- ROADMAP.md
- TODO.md
- docs/runbooks/wave-1-runtime-evidence.md
- docs/runbooks/wave-2-runtime-evidence.md
- docs/runbooks/wave-3-pda-readiness.md
- docs/runbooks/wave-4-external-dependencies.md
- docs/runbooks/wave-5-hardware-evidence.md
- docs/runbooks/wave-5-tms-evidence.md
- docs/runbooks/wave-6-deploy-evidence.md
- docs/runbooks/wave-6-evidence-preflight.md
- docs/runbooks/wave-6-closeout.md
- docs/domain/clarifications.md #50 / #66 / #67

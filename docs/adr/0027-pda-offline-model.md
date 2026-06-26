# ADR-0027：PDA 离线模型与技术栈定版框架

- 状态：Proposed
- 决策日期：2026-06-06
- 决策人：项目主人
- 起草人：AI 助手
- 关联：ADR-0001 / ADR-0015 / ADR-0024 / ADR-0026 / ADR-0029 / SPIKE-005 / SPIKE-005B / docs/runbooks/wave-3-pda-readiness.md

---

## 背景

ADR-0001 当前将 PDA 仓库作业端选为 React Native 独立 app，理由是仓库现场必须支持离线作业、实体扫码键、蓝牙打印等硬件能力，纯 Web/PWA 不足以作为生产候选。

用户提出 PDA 是否可以先做成网页版，再通过打包方式交付。经 review 后确认的边界是：不评估纯浏览器 / 纯 PWA，而是把 **Web UI + Android native shell + native plugins** 纳入 PDA 技术栈候选。这个候选可由 Capacitor 或最小 Android WebView wrapper 承载，Web 层复用 React / TypeScript / `@wms/api-client`，硬件能力通过原生插件接入。

因此需要在 SPIKE-005 RN 路线之外，新增 SPIKE-005B WebView/Capacitor 路线，并由本 ADR 作为统一决策框架。当前没有真 PDA、dev/staging runtime evidence 和两条 Spike 的同口径对比结果，所以本 ADR 只能进入 Proposed，不能 Accepted。

---

## 候选方案

### A. React Native bare app

沿用 ADR-0001 的当前 PDA 正式方向，以 RN bare app 承载 PDA 作业、离线队列、实体扫码键、蓝牙打印和本地加密存储。

优点：
- 与 ADR-0001 已有方向一致。
- RN 生态里接原生 SDK、扫码 Intent、蓝牙能力的路径成熟。
- 可以用 TypeScript 复用 `@wms/api-client` 与 OpenAPI 类型。

缺点：
- 需要维护独立 RN 构建、发布和调试链路。
- 与 Web 管理端 UI 不能直接复用 DOM 组件。
- 硬件适配仍依赖真 PDA 与厂商 SDK evidence。

### B. Web UI + Android native shell + native plugins

用 React Web UI 作为业务界面，外层由 Capacitor 或 Android WebView native shell 承载；扫码键、离线持久化、蓝牙打印、设备标识等能力通过 native plugin 暴露给 Web 层。

优点：
- 业务 UI、TypeScript 类型和 `@wms/api-client` 复用度更高。
- 管理端与 PDA 端的前端工程经验更接近。
- 对简单流程可能降低长期维护成本。

缺点：
- WebView 对实体扫码键、离线持久化和蓝牙打印的稳定性必须用真 PDA 证明。
- WebView 存储可能被系统策略影响，必要时必须切到 native SQLite / 加密 KV plugin。
- 不能把纯浏览器或手机摄像头测试结果当作生产 PDA evidence。

### C. 原生 Android / Kotlin 后备方案

如果 A/B 都不能满足实体硬件、离线恢复或性能要求，则启动原生 Android / Kotlin 评估，把 Web 或 RN 降为非主线。

优点：
- 对 Android PDA 硬件能力控制力最强。
- 厂商 SDK 接入路径最直接。

缺点：
- 与现有 TypeScript 前端资产复用最低。
- 团队需要额外维护 Kotlin/Android 原生能力。
- 当前没有证据表明必须升级到该方案。

---

## 决策

本 ADR 当前只确定 PDA 技术栈的决策框架，不在没有真机 evidence 的情况下改变 ADR-0001。

1. 生产 PDA 技术栈暂不定版。ADR-0001 的 React Native 方向继续保留为当前正式选型。
2. 首批只允许两个可进入对比的生产候选：`react-native` 与 `webview-capacitor`。
3. `webview-capacitor` 不是纯 Web/PWA，必须包含 Android native shell 与 native plugins，用于实体扫码键、离线持久化、蓝牙打印和设备能力。
4. SPIKE-005 与 SPIKE-005B 必须在同一真 PDA、同一条码样本、同一 M2/M3 dev/staging 测试数据和同一 evidence 标准下对比。
5. Wave 3 PDA runtime evidence 必须包含 `pda_stack_candidate`，取值只能是 `react-native` 或 `webview-capacitor`。
6. `pda_stack_candidate=react-native` 时，`spike005_result_ref` 必须指向 SPIKE-005 实测结果；`pda_stack_candidate=webview-capacitor` 时，必须指向 SPIKE-005B 实测结果。
7. 禁止用 local / prod / production / mock / fake / stub / example / browser / simulator / emulator / phone / camera 证据关闭 PDA gate。
8. 本 ADR 进入 Accepted 的前置条件是：至少一条候选完成真 PDA + dev/staging evidence；若两条都完成，则必须给出同口径对比结论。
9. `apps/pda-mobile` 生产 app 只有在本 ADR Accepted 后启动。Accepted 前只允许 readiness、runbook、validator 和 spike 级 PoC，不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖；这里的依赖段包含 `dependencies`、`devDependencies`、`optionalDependencies` 和 `peerDependencies`。`pnpm-workspace.yaml` 不得在 Accepted 前显式加入 `apps/pda-mobile`。
10. 本 ADR Accepted 后，生产 PDA 依赖、lockfile importer / packages / snapshots 条目和 Android native 打包脚本必须与 runtime evidence 的 `pda_stack_candidate` 一致：`react-native` 候选不能混入 Capacitor 生产链路，`webview-capacitor` 候选不能混入 React Native / Expo / EAS 生产链路。

---

## 后果

### 正面

- 保留 ADR-0001 的稳定性，同时把用户提出的 Web 打包方向纳入可验证候选。
- PDA 技术栈选择从观点判断转为 evidence 判断。
- WebView/Capacitor 若通过真机验证，可以提高 Web UI、OpenAPI 类型和 API client 的复用价值。
- RN 若更稳定，可以以 SPIKE-005 证据继续推进，不被纯 Web 方向干扰。

### 负面

- PDA 生产端启动继续依赖外部状态：真 PDA、dev/staging、条码样本和硬件 evidence。
- 需要维护两条 Spike 文档与 evidence 口径，短期治理成本增加。
- 在 ADR Accepted 前，无法用 `apps/pda-mobile` 生产 app 表示 Wave 3 PDA 已完成。

### 风险与应对

| 风险 | 应对 |
|------|------|
| WebView/Capacitor 在浏览器里表现正常，但实体扫码键不稳定 | 只有真 PDA + native plugin evidence 可以 accept，浏览器结果不得关闭 gate |
| RN 与 WebView/Capacitor evidence 样本不同，无法公平对比 | Wave 3 PDA readiness 要求同一设备、同一条码样本、同一 M2/M3 测试数据 |
| 为了推进进度提前创建生产 PDA app | `report_wave3_completion.py` 与 TODO 均把生产 app 保持为 pre-release gate |
| 候选引用与实际 Spike 不一致 | `validate_wave3_pda_runtime_evidence.py` 校验 `pda_stack_candidate` 与 `spike005_result_ref` 匹配 |
| 两条候选都 reject | 启动方案 C 原生 Android / Kotlin 评估，并在本 ADR 追加拒绝原因 |

---

## 实施约束

1. 不新增业务故事、角色、状态、canonical 字段或审批源。
2. 不创建生产 PDA app，不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖（含 `devDependencies`），不显式把 `apps/pda-mobile` 加入 `pnpm-workspace.yaml`，直到本 ADR Accepted。
3. Spike 级 PoC 代码必须放在 `spikes/` 下，不得混入 `apps/pda-mobile`。
4. PDA 写操作必须继续遵守 ADR-0006 的 L4 / L5 / L8 / L11 要求，以及 ADR-0015 的 A/B/C 规则分层。
5. PDA 端 API 调用必须走 ADR-0026 的 OpenAPI / `@wms/api-client` 链路，禁止绕过契约手写类型。
6. PDA 鉴权与离线 token 状态必须对齐 ADR-0024；离线 replay 必须记录 H2 `audit_event` 与 `Idempotency-Key`。
7. 原型或 Web UI 资产如迁入生产，必须按 ADR-0029 和 `docs/prototypes/prototype-to-production.md` checklist 执行。
8. 本 ADR 改为 Accepted 时，runtime evidence 的 `pda_stack_candidate` 指向哪条候选，对应 SPIKE-005 或 SPIKE-005B 必须已为 accepted，并有真实 `## 实测结果`。
9. 若 SPIKE-005 与 SPIKE-005B 均为 accepted，本 ADR 改为 Accepted 前必须追加 `## 同口径对比结论`，记录两者在同一真 PDA、同一条码样本、同一 M2/M3 dev/staging 测试数据下的对比结论。
10. ADR Accepted 后启动生产 app 时，`check_pda_production_gate.py` 继续校验生产依赖、lockfile importers / packages / snapshots 和 native 打包脚本是否匹配已选 `pda_stack_candidate`；Expo / EAS Android build 或 prebuild 命令同样视为 RN 生产链路。

---

## 参考

- [ADR-0001：技术栈选型](0001-tech-stack.md)
- [ADR-0015：多端业务规则放置](0015-multi-end-rules.md)
- [ADR-0024：鉴权模型](0024-auth-model.md)
- [ADR-0026：跨端契约管线](0026-cross-end-contract-pipeline.md)
- [ADR-0029：前端原型先行工作流](0029-frontend-as-prototype-workflow.md)
- [SPIKE-005：React Native 扫枪 + 离线队列](../spikes/spike-005-rn-scanner.md)
- [SPIKE-005B：WebView/Capacitor PDA 可行性验证](../spikes/spike-005b-webview-capacitor-pda.md)
- [Wave 3 PDA Readiness Runbook](../runbooks/wave-3-pda-readiness.md)
- [业务澄清 #71](../domain/clarifications.md#pda-2026-06-06)

---

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-06-06 | v0.1 | 建立 Proposed ADR，记录 RN 与 WebView/Capacitor 两候选的同口径 evidence 决策框架 |

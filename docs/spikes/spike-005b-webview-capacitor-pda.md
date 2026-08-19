# SPIKE-005B: WebView/Capacitor PDA 可行性验证

- 状态：deferred
- 时间盒：2 天（16 小时）-- 与 SPIKE-005 在同一真 PDA / dev 或 staging 条件下对比验证
- Owner：项目主人
- 起始：-- 完成：--（未启动）
- 关联 Wave 任务：W3.A PDA 生产端；W6.D 真 PDA + L7 evidence
- 关联 ADR：ADR-0001（当前 PDA 正式选型仍为 React Native）；ADR-0015 多端规则；ADR-0027 PDA 离线模型与技术栈定版框架（Proposed）

---

## 1. 背景与问题

ADR-0001 当前决定 PDA 用 React Native 独立 app，理由是仓库现场必须离线、需扫码硬件、需蓝牙打印，纯 Web/PWA 不够稳。SPIKE-005 也按 RN bare + 实体扫码键 + 离线队列方向设计。

用户提出：PDA 是否可以先做成网页版，再通过打包方式交付。修正后的边界是：不评估纯浏览器 / 纯 PWA，而评估 **Web UI + Android native shell + native plugins**。候选实现可以是 Capacitor 或最小 Android WebView wrapper；Web 层复用 React / TypeScript / `@wms/api-client`，硬件能力通过原生插件接入。

这个 Spike 只回答可行性，不直接替换 ADR-0001。即使本 Spike accept，也只表示 WebView/Capacitor 有资格进入 ADR-0027 与 RN 对比；最终 PDA 技术栈仍以真机证据和 ADR-0027 为准。

参考：

- Capacitor 官方文档：https://capacitorjs.com/docs
- Capacitor Android 自定义原生代码：https://capacitorjs.com/docs/android/custom-code
- Wave 3 PDA Readiness Runbook：`../runbooks/wave-3-pda-readiness.md`
- RN 对照方案：`spike-005-rn-scanner.md`

---

## 2. 验证假设

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | Web UI 可以在目标 PDA 的 Android WebView 中稳定运行，并复用 `@wms/api-client` 与 OpenAPI 生成类型 | 在 `spikes/spike-005b-webview-capacitor-pda/` 建最小 React build + native shell，调用 dev/staging M2/M3 API |
| H2 | 实体扫码键可以通过 Intent / KeyEvent / DataWedge 等厂商通道进入 Web 层，不依赖浏览器摄像头 | 至少 1 款真 PDA 上写 native plugin，把扫码结果分发给 JS 事件；记录设备型号和输入方式 |
| H3 | WebView 方案的扫码反馈延迟满足与 SPIKE-005 相同的样本口径 | 用同一批 50 个脱敏条码样本，记录 P95 扫码键触发到 UI 可见反馈的耗时 |
| H4 | 离线队列可以持久化并在 crash / 重启后恢复，每条 replay 携带 `Idempotency-Key` | 断网扫描 50 个 M2/M3 任务，强杀 app 后重启，联网 replay；服务端只入库 50 条业务结果 |
| H5 | token 离线状态机与 ADR-0024 / SPIKE-001 的 access 1h + refresh 24h 策略兼容 | 覆盖 access 过期、refresh 可用、refresh 过期、离线锁定、恢复登录后保留队列 |
| H6 | WebView + native plugin 可以记录 H2 `audit_event` 链路证据 | 每次 replay 后查询 `audit_event`，记录 `Idempotency-Key` / 操作人 / 时间 / 来源端 |
| H7 | 蓝牙打印 / 面单打印至少存在可落地的 native plugin 接入路径 | 在同一 native shell 中验证最小打印调用或记录目标设备 SDK 接入 PoC；如硬件未到位，明确 defer 到 M-PK 硬件 evidence |
| H8 | 工程化成本低于或不高于 RN 方案的长期维护成本 | 对比依赖数量、构建命令、测试命令、native plugin 代码量、CI 接入成本，与 SPIKE-005 一起进入 ADR-0027 |

---

## 3. 退出条件

| 状态 | 条件 |
|------|------|
| accept | H1 / H2 / H4 / H5 / H6 确认；H3 使用同一条码样本口径完成；H7 至少给出真硬件 PoC 或明确 defer 边界；输出与 SPIKE-005 的对比结论 |
| reject | 实体扫码键不能稳定进入 Web 层；离线队列在 crash / 重启后不能可靠恢复；或 WebView 性能/兼容性导致关键作业不可用 |
| defer | 真 PDA、dev/staging、蓝牙打印硬件或目标厂商 SDK 未到位，导致无法形成真实证据 |

accept 不等于直接改技术栈；accept 后仍需 ADR-0027 对比 RN 与 WebView/Capacitor，再定版 PDA 离线模型。

---

## 4. 实施路径

### 步骤 1：建 WebView/Capacitor spike 工程（2 小时）

```
spikes/spike-005b-webview-capacitor-pda/
├── package.json
├── android/
├── src/
│   ├── App.tsx
│   ├── screens/
│   │   ├── LoginScreen.tsx
│   │   ├── ScanScreen.tsx
│   │   └── QueueScreen.tsx
│   ├── lib/
│   │   ├── api.ts
│   │   ├── native-bridge.ts
│   │   └── offline-queue.ts
│   └── styles.css
└── docs/
    └── runtime-evidence.md
```

约束：

- 不创建 `apps/pda-mobile` 生产 app。
- 不把 spike 依赖加入生产 workspace。
- 不引入纯浏览器 / 纯 PWA 作为生产候选。
- Web 层只能复用已存在的 API 类型、表单校验思路和视觉 token；不能绕开 H1/H2/H3 契约。

### 步骤 2：native shell 与扫码键插件（4 小时）

- Android 侧监听 KeyEvent / Intent / DataWedge 输入。
- 将扫码结果通过 JS bridge 发送到 Web 层。
- 记录设备型号、Android 版本、扫码输入方式和失败码。

### 步骤 3：离线队列与 replay（4 小时）

- 使用 WebView 可持久化存储或 native 存储桥保存队列。
- 每条任务生成 `Idempotency-Key`。
- 覆盖断网、重启、恢复联网、幂等 replay、失败重试。

### 步骤 4：token 状态机（2 小时）

沿用 SPIKE-001 / ADR-0024 的 token 边界：

```
[Online + access valid] --access expire--> [Online + refresh]
[Online + refresh expire] --> [Force login, queue preserved]
[Offline + access valid] --access expire--> [Offline queue preserved]
[Offline + refresh expire] --> [Locked, await online]
```

### 步骤 5：真机证据与对比报告（4 小时）

- 用同一台 PDA、同一批 50 个条码、同一套 M2/M3 测试数据跑 RN 与 WebView/Capacitor。
- 输出扫码延迟、离线 replay、幂等 replay、audit_event、打印 PoC、构建/测试成本对比。
- 结论写入本文件 §7，并作为 ADR-0027 输入。

---

## 5. 风险与后备方案

| 风险 | 概率 | 影响 | 后备方案 |
|------|------|------|---------|
| WebView 对实体扫码键事件接入不稳定 | 中 | 高 | 回退 SPIKE-005 RN bare；或评估原生 Android + bridge |
| 离线队列存储在 WebView 中被系统清理或恢复不可靠 | 中 | 高 | 改用 native SQLite / 加密 KV 插件；仍不可靠则 reject WebView 方案 |
| 厂商 PDA 的 Intent / DataWedge 行为差异大 | 高 | 中 | 建立设备配置表；首期只支持 1-2 款已验证设备 |
| 蓝牙打印插件成本接近原生开发 | 中 | 中 | 打印能力从 Web 层抽象为 native service；与 M-PK 硬件 evidence 合并验证 |
| Web UI 复用导致 PDA 触控密度不符合一线操作 | 中 | 中 | PDA 页面仍按 ADR-0021 原型走查和触控基线重做，不直接复制 PC 页面 |
| OTA / 热更新绕过审计 | 低 | 高 | 不启用未审计的热更新；app 版本发布仍按灰度与审计流程 |

---

## 6. 产出物清单

- 代码：`spikes/spike-005b-webview-capacitor-pda/`
- 文档：本文件 §7；`spikes/spike-005b-webview-capacitor-pda/docs/runtime-evidence.md`
- 对比：RN vs WebView/Capacitor 证据表，作为 ADR-0027 输入
- ADR：如 accept，参与 `docs/adr/0027-pda-offline-model.md` 决策；如 reject，ADR-0027 记录拒绝理由

---

## 7. 决策记录

- 日期：2026-06-06
- 结论：**deferred**（等待真 PDA + dev/staging + 与 SPIKE-005 同口径对比）
- 时间盒消耗：0（未启动）

### 7.1 用户确认

用户确认按修正建议执行：

- 不把方案定义为纯网页 / 纯 PWA。
- 新增 WebView/Capacitor native shell 候选。
- 保留 RN SPIKE-005，不直接替换 ADR-0001。
- 最终 PDA 技术栈推迟到真机 Spike 后，通过 ADR-0027 定版。

### 7.2 启动条件

满足以下全部条件后启动：

1. 至少一台真 PDA 到位，并能记录设备资产引用。
2. dev/staging 可访问，且 M2/M3 handler、`Idempotency-Key`、H2 `audit_event` 链路可验证。
3. SPIKE-005 RN 对照验证同步启动或已有同口径证据。
4. 可保存扫码日志、离线 replay 日志、幂等 replay 日志、audit_event 查询证据和人工易用性走查记录。

### 7.3 与 SPIKE-005 的关系

SPIKE-005 继续作为当前正式 ADR-0001 路线的 RN 验证；本文件只补充 WebView/Capacitor 候选。两者共用 Wave 3 PDA Readiness Runbook 的真机、dev/staging、日志引用和拒绝边界。

如果两者都 accept，ADR-0027 根据证据选择生产 PDA 技术栈。如果只有一者 accept，ADR-0027 记录另一者 reject / defer 原因。如果两者都 reject，升级为原生 Android PDA 技术栈评估。

## 实测结果

> 当前状态为 deferred；本节是 accepted 前必须替换的 evidence 模板，不能保留待填内容。

- 待填：SPIKE-005B webview-capacitor 在真 PDA 上使用 dev/staging runtime evidence 验证。
- 待填：证据引用 `docs/retros/wave-3-pda-runtime-evidence.json`，并记录 native shell、native plugin 和扫码输入方式。
- 待填：覆盖 offline replay、Idempotency-Key replay、audit_event、L7 和 usability review。

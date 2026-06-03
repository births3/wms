# SPIKE-005: React Native 扫枪 + 离线队列

- 状态：deferred
- 时间盒：2 天（16 小时）— 推迟到启动条件满足时重新计时
- Owner：项目主人
- 起始：— 完成：— （未启动；推迟原因见 §7）
- 关联 Wave 任务：Wave 1 PDA 离线策略；W3.A M2 入库 PDA 端启动前置
- 关联 ADR：ADR-0015 多端规则；ADR-0001（RN 已选定）；拟产出 ADR-0027 PDA 离线模型（推迟随 spike）

---

## 1. 背景与问题

ADR-0001 决定 PDA 用 React Native（独立 app，复用 packages/api-client）。clarifications.md C1 决定"PDA 离线 24h 默认可配置"。
但**未验证**：

1. 实仓 PDA 上 RN 的扫码识别速度（GSP 规定收货环节扫码 ≤ 0.5s 反馈，否则换二维码影响业务）
2. 离线模式下"扫码任务排队 + 联网时回放上传"的状态机是否可靠
3. RN 项目骨架：Expo（managed/bare）vs RN CLI 哪个适合 PDA 长期维护
4. JWT 离线缓存策略（与 spike-001 H5 衔接）
5. 扫码硬件触发（PDA 实体扫码键，非摄像头）的 RN 集成
6. 蓝牙打印机（清场不在本 spike，留 Wave 5）

---

## 2. 验证假设

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | RN bare（CLI）+ react-native-vision-camera 在中端 Android（PDA 主流配置）上扫码识别延迟 < 500ms | 写最小 demo，手机上对 50 个不同条码扫描各 5 次取 P95 |
| H2 | RN PDA 实体扫码键（KeyEvent / Intent broadcast）能通过 react-native-event-listeners 或自写 native module 接入；不依赖摄像头 | 测 2 款 PDA：Honeywell EDA / 优博讯 i6310 |
| H3 | 离线队列用 `react-native-mmkv`（或 `expo-sqlite`）持久化任务 + Tanstack Query mutation 的 onError 入队，联网后批量重放，每条带 idempotency_key 防重 | 模拟飞行模式 + 上传 50 个收货任务 → 联网后服务端只入库 50 条 |
| H4 | JWT 离线缓存：access 1h + refresh 24h；access 过期但 refresh 仍有效 → 队列内任务标"待提交"；refresh 也过期 → 强制重新登录但保留本地任务待恢复 | 状态机测试 + UI 联动 |
| H5 | Bare RN 项目依然能复用 `packages/api-client`（生成的 TS 类型 + openapi-fetch）；Metro bundler 与 vite 共存于 monorepo | 在 spike-003 demo 基础上加 RN 子项目验证 import |
| H6 | 测试策略：jest + react-native-testing-library 能跑业务逻辑测试；E2E 用 Detox 或 Maestro | 写 3 个 unit + 1 个 E2E 跑 |

---

## 3. 退出条件

| 状态 | 条件 |
|------|------|
| accept | H1 / H3 / H4 / H5 全部确认；H2 至少 1 款 PDA 成功；H6 工具链选定；产出 ADR-0027 + spike 代码 |
| reject | H1 不成立（扫码 > 1s）→ 候选改用 native Android Java + RN bridge；新建 spike-005b 评估 |
| defer | H2 第二款 PDA 验证可延到正式采购后；蓝牙打印机延到 Wave 5 spike |

---

## 4. 实施路径

### 步骤 1：搭 RN bare 项目骨架（2 小时）

```
spikes/spike-005-rn-scanner/
├── package.json              # workspace member, 复用 packages/api-client
├── ios/                      # 暂不验证，但保留
├── android/
├── src/
│   ├── App.tsx
│   ├── screens/
│   │   ├── LoginScreen.tsx
│   │   ├── ScanScreen.tsx
│   │   └── QueueScreen.tsx
│   ├── stores/
│   │   ├── queue.ts          # mmkv-backed offline queue
│   │   └── auth.ts           # token state machine
│   └── lib/
│       └── api.ts            # 复用 @wms/api-client
└── jest.config.js
```

选择 RN CLI（bare） + react-native 0.74：理由 Expo managed 装不了实体扫码键 native module。

### 步骤 2：vision-camera 扫码（3 小时）

- 集成 `react-native-vision-camera` + `vision-camera-code-scanner`
- ScanScreen 上扫 50 个测试条码（GS1 + Code128 + 二维码各类）
- 测延迟：扫到 ↔ JS 拿到字符串的 ms 数

### 步骤 3：实体扫码键集成（4 小时）

- 写最小 native module（Java，监听 KeyEvent.KEYCODE_SCAN）
- bridge 到 JS 事件：`DeviceEventEmitter.addListener('hardwareScan', ...)`
- 测两款 PDA（用现有的 / 借测）
- 文档化键码差异（不同厂商可能不同 keycode）

### 步骤 4：离线队列（3 小时）

```ts
// src/stores/queue.ts
type Task = {
  id: string;             // uuid v4 = idempotency_key
  type: "M2_RECEIVE" | "M2_PUTAWAY" | ...;
  payload: any;
  created_at: string;
  status: "pending" | "syncing" | "synced" | "failed";
  retries: number;
};

// mmkv: read-write 同步、加密
// 联网状态变化 → 自动 flush
// 失败 3 次 → 进入 failed，UI 红色提示人工介入
```

模拟飞行模式：用 `react-native-network-info` + 手动断 wifi 验证。

### 步骤 5：Token 状态机（2 小时）

依据 spike-001 §H5 的状态图实现：
```
[Online + access valid] --access expire--> [Online + refresh] --refresh ok--> back
[Online + refresh expire] --> [Force logout, queue preserved]
[Offline + access valid] --access expire--> [Offline read-only + queue]
[Offline + refresh expire] --> [Locked, await online]
```

### 步骤 6：跑 unit + E2E（1.5 小时）

- 3 个 unit 测：queue dedup / token state machine / api retry
- 1 个 Maestro flow：login → scan → offline → online → queue flushed

### 步骤 7：写 ADR-0027（0.5 小时）

`docs/adr/0027-pda-offline-model.md` Proposed：
- Bare RN（理由）
- 队列结构 + 持久化技术
- Token 状态机（图）
- 实体扫码键集成模式
- 与 H1 / H2 的服务端衔接（idempotency_key 头部）

---

## 5. 风险与后备方案

| 风险 | 概率 | 影响 | 后备方案 |
|------|------|------|---------|
| vision-camera 扫码 > 500ms | 中 | 高 | 换 react-native-camera-kit 或纯 native 扫码控件 + bridge |
| 实体扫码键 keycode 各厂商不一致 | 高 | 中 | 配置文件 `pda-keycode.json` 按机型维护；Wave 1 仅支持 1-2 款 |
| mmkv 在某些 PDA 加密初始化慢 | 低 | 低 | 退到 expo-sqlite + 简单 key-value 表 |
| queue 重放与服务端去重失败 | 中 | 高 | 服务端 idempotency_key 唯一索引（spike-002 audit 含此字段）；冲突时返回幂等成功 |
| Maestro / Detox 设置成本高 | 中 | 低 | E2E 延后到 Wave 3，spike 仅做 unit |
| 借测 PDA 时间不可控 | 高 | 中 | H2 仅在 1 款机型上验证；剩余机型作为 Wave 3 backlog |

---

## 6. 产出物清单

- 代码：`spikes/spike-005-rn-scanner/`（RN 0.74 bare 项目）
- 文档：本文件 §7；`pda-keycode.json` 模板
- ADR：`docs/adr/0027-pda-offline-model.md`
- 治理：在 `docs/architecture-dependencies.md` 加 PDA 模块依赖（依赖 packages/api-client + spike-001 token 模型）
- 状态图：`spikes/spike-005-rn-scanner/docs/token-state-machine.md`

---

## 7. 决策记录

- 日期：2026-05-24
- 结论：**deferred**（推迟到启动条件满足时重启）
- 时间盒消耗：0（未启动）

### 7.1 推迟理由

本 Spike 涉及 **物理硬件 + 真机环境**，在当前 Wave 0.5 范围内强行跑会得到不可靠结论：

| 项 | 在当前环境跑的问题 |
|----|----------|
| H1 vision-camera 扫码延迟 < 500ms | 需要 Android 真机 + 实物条码 / 二维码；emulator 模拟摄像头无意义 |
| H2 实体扫码键 KeyEvent | 需要 Honeywell EDA / 优博讯 i6310 等真 PDA 设备；不同厂商 keycode 不同；当前无可借测设备 |
| H3 离线队列（mmkv + idempotency_key） | 可在不连真 PDA 时验证逻辑，但需结合 H1/H2 才有完整业务场景 |
| H4 JWT 离线缓存（与 SPIKE-001 衔接） | SPIKE-001 已定型双 token + 5 状态机文档（pda-offline-state.md），代码实施在 PDA 端，需要 H1/H2 验证后才能写 |
| H5 Bare RN + monorepo 集成 | 可纯前端验证，但 RN bare 项目骨架装包成本高（约 800 包），强行跑会污染主项目 pnpm workspace |
| H6 jest + Maestro / Detox E2E | 需要 H1-H5 的代码基础 |

更深的原因：**推迟才符合 ADR-0021 高保真原型策略 §约束 4** —— "Wave N 启动前，N 涉及的 UI 故事必须有高保真原型 + ≥1 次走查 approved"。
SPIKE-005 是 PDA 实施前的技术验证（不是原型），按 ROADMAP 应在 **Wave 3 W3.A M2 PDA 入库** 启动前的 0.5 周内做（与 Wave 3 启动同期）。

### 7.2 启动条件（任一满足即可重启）

1. **业务方采购 / 借测至少 1 款 PDA 设备**（Honeywell EDA52 / 优博讯 i6310 / Zebra TC52 之一）
2. **Wave 3 进入启动期**（W3.A M2 PDA 入库前 0.5 周）
3. **SPIKE-001 ADR-0024 鉴权模型已 Accepted 落地到 backend/**（即 Wave 1 W1.A 完成）+ packages/api-client 可用（Wave 1 W1.C 完成）—— 否则 PDA 写不出真接 token + API 的 demo

启动时**重新计时 2 天时间盒**，本 §7 决策记录追加 v0.2 修订段，不删原文。

### 7.3 不阻塞 Wave 0.5 退出的理由

按 docs/spikes/README.md §6 Wave 0.5 退出条件：
> 5 个 Spike 全部进入 accepted / rejected / deferred 三态之一（不能停在"起草"或"进行中"）

`deferred` 与 `accepted` 一样是合法终态。SPIKE-005 推迟不影响 Wave 1 启动，因为：
- Wave 1 W1.A/B/C 全是后端 + PC 前端工作（鉴权 / 审计 / OpenAPI 工具链 / packages/api-client）
- PDA 端 Wave 1 仅有"PDA 登录"故事（H1-001 PDA），且原型已交付（h1-login-pda）
- 真 PDA 业务实施在 Wave 3 W3.A M2 入库时；SPIKE-005 与 Wave 3 启动同期最经济

### 7.4 backlog 归属

写入 ROADMAP §"v25 后续波次 backlog"段（与"放射性药品 30 年保留"等同列）：

```
| SPIKE-005 RN 扫枪验证 | Wave 3 W3.A 启动前 0.5 周 | 业务方采购/借测 PDA 设备 + W1.A/W1.C 完成（鉴权 + api-client 可用）|
```

### 7.5 关联文档保留

本 spike 文档（5 假设 + 实施路径 + 风险表）**完整保留**，启动时按既有结构填 §7 v0.2 修订记录。pda-offline-state.md（已在 spike-001 落盘）继续作为 PDA 端 token 状态机参考，本 spike 启动时直接复用。

### 7.6 拒绝清单（spike 完成后追加；当前为预设）

| 候选 | 拒绝/推迟理由（spike 启动时再确认）|
|------|-----------|
| Expo managed | RN bare 已是预设方向；spike 启动时如发现 Expo 也能装 native module（实体扫码键），可重新评估 |
| flipper / Reactotron 调试 | spike 范围外，工程化决策延到 Wave 3 |
| OTA 热更新（CodePush 等）| GSP 监管考虑：app 升级须留痕 + 可审计；当前不引入 |

### 7.7 Wave 3 readiness 决策（2026-06-03）

用户确认：先落 readiness/runbook，不引入 RN 依赖，不创建 `apps/pda-mobile` 生产 app。

执行口径：

- 以 [Wave 3 PDA Readiness Runbook](../runbooks/wave-3-pda-readiness.md) 作为 SPIKE-005 启动前置。
- SPIKE-005 继续保持 deferred；真 PDA 和 dev/staging 到位后重新计时 2 天。
- RN bare / 实体扫码键 / 离线队列持久化 / ADR-0027 均等 SPIKE-005 启动后再落。

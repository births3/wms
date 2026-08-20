# Phase 3 设备中台工单清单（ADR-0048 Phase 3）

- 状态：Frozen（2026-08-20，随规格冻结）
- 规格：`docs/domain/device-platform-phase3-spec.md`（Frozen）
- 开发模式：每票外向内 TDD，先失败测试再实现
- 提交：一票一主题；票内含文档+接口时落地拆两个 Conventional Commit
- 阶段 3 第一票前钉死 P3 起点 SHA：`9b815807`

## 总览与依赖

```
T01 错误码/权限/字典/schema 测试（表已由 S1.2 建齐）──┐
T02 设备注册/启停/心跳/绑定/离线告警 ────────────────┼── T06 设备与异常大盘
T03 指令生成/派发/回执/事件处理/超时重试（含 DWS/RFID 校验）├── T07 治理收口
T04 PTL 拍灯（差异规则/落账/亮灯互斥）── T03
T05 AGV pod_move（不可达标记/前置校验）── T03
T02+T03+T04+T05 ── T06
```

共 7 票。真机协议 / sorter_divert 派发 / stacker / 波次三层兜底 / PDA 生产 app 一律不进票（规格 §2.2）。

---

## T01 — 错误码登记、权限/字典验证与 schema 集成测试

**What to build:** `docs/error-codes.md` §6 登记 `M1_DEVICE_*` / `M1_BIND_*` / `M1_WCS_*` / `M1_PTL_*` / `M1_POD_*` / `M1_LOCATION_*` / `M1_AGV_*` / `M1_EVENT_*` / `M1_NUMBERING_UNAVAILABLE` 全套错误码；postgres 测试验证 S1.2 迁移的表结构/CHECK/GRANT/权限种子/字典/M-CG 规则/H4 种子。

**Blocked by:** 无（S1.2 迁移已入库）

**主题：** 落地拆两个 commit：`文档(治理)：登记 M1 设备中台错误码` + `测试(接口)：设备中台 schema 与种子 postgres 测试`

**先失败测试：** postgres 测试断言 `iot_devices` 表存在、非法 device_type 插入被拒；表已建 → 断言查询通过即绿（表由 S1.2 建齐，本票补验证面）。

**一条可失败验收：** `check_error_codes.py` 绿（M1_DEVICE_* 等全部登记）；`iot_event_logs` 的 `wms_app` 仅有 SELECT/INSERT 权限；`document_type=wcs_task` 字典项与 M-CG 规则存在；H4 六类告警种子存在。

**模块/文件边界：**

- `docs/error-codes.md` §6
- `backend/crates/api/tests/m1_device_platform_schema_postgres.rs`（新建）
- 禁止业务 handler / 禁止改 S1.2 迁移语义

**不做：** 引擎、页面、PDA。

**验证：**

```bash
cargo test -p wms-api --test m1_device_platform_schema_postgres -- --test-threads=1
python3 scripts/governance/check_error_codes.py
just gov-t1
```

---

## T02 — 设备注册/启停/心跳/绑定/离线告警

**What to build:** 设备档案 CRUD 与启停、心跳上报与在线判定、库位-设备点位绑定/软解绑、心跳超时扫描与 H4 离线告警。

**Blocked by:** T01

**主题：** `功能(接口)`（绑定 API 与权限点按规格 §6.1 落地）

**先失败测试：** POST `/iot-devices` 注册 `device_code=PTL-01` 返回 201 且 `online_status=offline`；同仓重复编码 → 409 `M1_DEVICE_DUPLICATE_CODE`（GWT 1/2）。

**一条可失败验收：** GWT 1-8（注册/重复/非法类型/绑定冲突/角色设备不匹配/离线禁绑/心跳上线/超时离线告警）+ GWT 22 之前置；绑定冲突 409、解绑软解绑置 `valid_to`。

**模块/文件边界：**

- `device_handlers.rs` / `device_service.rs` / `device_repository.rs`（api/src 新建，按分层）
- 心跳扫描调度作业 `iot_heartbeat_scan`（沿既有调度骨架模式）
- OpenAPI `openapi_paths` 设备与绑定路径
- 管理端设备档案页在 T06 一并做页面，本票只做 API
- 禁止任务引擎 / 禁止改 P1 库位语义

**不做：** 指令生成、事件处理、PTL/AGV 流程、页面。

**验证：**

```bash
cargo test -p wms-api --test m1_device_lifecycle_postgres -- --test-threads=1
just gov-t1
```

---

## T03 — 指令生成/派发/回执/事件处理/超时重试（含 DWS/RFID 校验）

**What to build:** `wcs_tasks` 六态状态机引擎：业务事务内生成（幂等键）、受控模拟网关 `dispatch` / `receipt` API、事件通道（`ptl_press`/`rfid_batch`/`dws_result`/`heartbeat`）、超时扫描与退避重试、重试耗尽 failed、人工重发/作废/跳过确认。

**Blocked by:** T01

**主题：** `功能(接口)`

**先失败测试：** 生成 `ptl_light_on`（幂等键 K）→ 201 `pending`；同键重复 → 200 同任务不重复插入（GWT 9）。

**一条可失败验收：** GWT 9、11、15、19、20、21（幂等/回执幂等/孤儿事件/DWS 校验/RFID EPC 覆盖/重试耗尽与人工介入）；`dispatch` / `receipt` 要求 `m1.device.manage`、`Idempotency-Key` 与审计；DWS/RFID 落账校验入引擎规则（规格 §10.2/§10.5）。

**模块/文件边界：**

- `wcs_task_service.rs` / `wcs_task_repository.rs` / `device_event_service.rs`（api/src 新建）
- 状态机/校验规则入 domain（`wcs_task_state.rs` 纯函数）
- 超时扫描调度作业 `iot_wcs_timeout_scan`
- 事件处理同步执行 + 孤儿窗口（30 秒）
- 落账复用既有 inventory 上下文命令（`confirm_replenish_in_tx` 等），禁止新造库存写入
- 禁止常驻消费进程 / 禁止真机协议

**不做：** PTL 亮灯互斥与差异规则（T04）、AGV 不可达（T05）、页面。

**验证：**

```bash
cargo test -p wms-api --test m1_wcs_task_engine_postgres -- --test-threads=1
cargo test -p wms-domain -- device
just gov-t1
```

---

## T04 — PTL 拍灯流程（亮灯互斥/差异规则/落账）

**What to build:** `ptl_light_on` 亮灯互斥（I3）、`ptl_press` 拍灯确认落账（拍灯即确认）、数量差异规则（超阈值 422）、重复拍灯幂等、`ptl_light_off` 收尾。

**Blocked by:** T03

**主题：** `功能(接口)`

**先失败测试：** 同一 PTL 未终态亮灯任务存在时再生成 → 409 `M1_PTL_LIGHT_BUSY`（GWT 10）。

**一条可失败验收：** GWT 10、12、13、14（互斥/等量确认落账/差异落账+告警/超阈值阻断）；差异阈值判定入 domain 单测。

**模块/文件边界：**

- 亮灯互斥与差异判定入 domain（`ptl_validation.rs`）
- 事件确认路径复用 T03 事件处理
- 禁止改补货/波次落账语义（只调用既有命令）

**不做：** 真机亮灯、颜色扩展、PDA。

**验证：**

```bash
cargo test -p wms-api --test m1_ptl_light_postgres -- --test-threads=1
cargo test -p wms-domain -- ptl
just gov-t1
```

---

## T05 — AGV 货到人账务联动（pod_move 与不可达标记）

**What to build:** `pod_move` 生成（一托一搬 I4）、executing 置 `agv_unreachable_at`、终态清除、格口账务前置校验（I5）、标记一致性扫描（H4 `agv_marker_inconsistent`）。

**Blocked by:** T03

**主题：** `功能(接口)`

**先失败测试：** `pod_move` executing 回执后格口 `agv_unreachable_at` 非空；该格口上架 → 422 `M1_LOCATION_UNREACHABLE`（GWT 16）。

**一条可失败验收：** GWT 16、17、18、22（不可达阻断/一托一搬/全程不落库存账/标记一致性告警）。

**模块/文件边界：**

- `agv_service.rs`（api/src 新建）
- 不可达前置校验挂在既有上架/补货/拣选/移库写入路径（只加校验，不改语义）
- 一致性扫描调度作业并入 `iot_wcs_timeout_scan`
- 禁止 RCS 真机对接 / 禁止改格口库位字段语义

**不做：** 真机 RCS、货架物理位置维护、PDA。

**验证：**

```bash
cargo test -p wms-api --test m1_agv_pod_move_postgres -- --test-threads=1
just gov-t1
```

---

## T06 — 设备与异常任务大盘

**What to build:** 设备档案页（配置型）、绑定管理页、设备大盘与异常任务大盘（列表型），含模拟器派发/回执与重发/作废/跳过确认 Dialog。

**Blocked by:** T02、T03、T04、T05

**主题：** `功能(管理端)`

**先失败测试：** 新增 `apps/web-admin/self-checks/m1-device-self-check.mjs` 与 `m1-device-dashboard-self-check.mjs`：断言配置型分区/列表模板与 `@wms/api-client` 调用存在；文件未建 → 红。

**一条可失败验收：** 配置型页面分区符合规格 §8；保存/动作走 `@wms/api-client`；登记菜单 `m1-devices` / `m1-device-dashboard` 与 `page-query-core-fields.json`；页面 <800 行。

**模块/文件边界：**

- `apps/web-admin/src/pages/master/M1DevicePage.tsx`、`M1DeviceDashboardPage.tsx`（及页面私有组件）
- `apps/web-admin/src/features/device/`
- 开发 Mock
- 禁止裸 fetch / 禁止策略 CRUD 混入 / 禁止改 P1 页面

**不做：** 图表邮件、推送、真机状态页。

**验证：**

```bash
node apps/web-admin/self-checks/m1-device-self-check.mjs
node apps/web-admin/self-checks/m1-device-dashboard-self-check.mjs
just gov-t1
```

---

## T07 — 治理收口（OpenAPI/质量矩阵/菜单/H4 种子验证/page-query）

**What to build:** 规格 §6.1 路径表全量登记质量矩阵 `US-*` 的 `api_paths`；OpenAPI 同步无漂移；菜单与 page-query 核漏；H4 六种子验证；E2E 证据登记。

**Blocked by:** T02、T03、T04、T05、T06

**主题：** 落地拆 commit：`功能(接口)：openapi-sync` 与 `文档(治理)：质量矩阵与设备中台登记`（菜单已由 T06 登记，本票只核漏）

**先失败测试：** `check_quality_matrix` 在 `api_paths` 未扩全前失败；本票补齐后绿。

**一条可失败验收：** `api_paths` 覆盖规格 §6.1 全表；H4 六 event_type 定义种子核验；`just openapi-sync` 无未提交漂移。

**模块/文件边界：**

- `governance/quality-matrix.toml` + 生成页
- OpenAPI + `just openapi-sync`
- 菜单种子核验 / `page-query-core-fields.json`
- 禁止回改 T01-T06 行为

**不做：** 真机、Phase 4 事项。

**验证：**

```bash
just openapi-sync
just gov-t1
```

---

## 全清单「不做」（每票默认继承）

| 标签 | 内容 |
|---|---|
| 真机协议 | 厂商协议驱动、RCS 对接、HTTP/TCP/Modbus/MQTT 解析、设备 SDK |
| 类型登记不实现 | `sorter_divert` 派发、`stacker` 设备 |
| P1 已延期 | 波次三层兜底、拣选中断、质量区 7/3 天滞留 |
| 本切片不做 | 客户端推送网关、常驻事件消费进程、自检协议、拍灯颜色扩展 |
| 门禁 | `apps/pda-mobile` 生产启动（ADR-0027 未 Accepted） |

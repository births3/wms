# Phase 3 设备中台技术规范（AGV / PTL / DWS / RFID，ADR-0048 Phase 3）

- 状态：Frozen（三轮自查通过：语义矛盾 0、标准硬违规 0，2026-08-20）
- 底稿：`docs/domain/storage-location-model.md` §6.1-§6.6（表 schema 与业务流程的唯一事实源，本规范只引用不复制）；ADR-0048 决策 5/9
- 工单清单：`docs/domain/device-platform-phase3-tickets.md`（冻结后生成）
- 与 P2 补货的关系：P2 已声明「iot_devices / wcs_tasks / iot_event_logs 空表不进补货切片，与硬件一并放到 Phase 3」

## 1. 问题与方案

WMS 已具备空间/库位/补货/波次等仓内业务能力，但缺乏统一硬件接入面：AGV 货架（货到人）、PTL 电子标签、DWS 称重复核、RFID 通道均需指令下发、事件回传、账务联动与设备生命周期管理。若各模块自行对接厂商协议，将产生重复驱动、无统一证据流、无离线兜底。

**方案**：按 `storage-location-model.md` §6.1-§6.6 的通用中台四表（`iot_devices` / `location_device_bindings` / `wcs_tasks` / `iot_event_logs`）建齐基线并实现软硬件集成：指令-事件闭环、账务联动、设备生命周期、心跳离线告警、超时重试与人工介入。

**决策基线（指挥官已拍板，不可更改）**：

1. **硬件策略：模拟器/回放先行，真机证据后置**。本切片不接任何厂商真机协议；设备端（RCS/PTL/DWS/RFID）以可编程模拟网关（测试内联模拟器 + 开发 Mock）替代，协议驱动层（http/tcp/modbus_tcp/mqtt 解析）留待真机切片。`iot_devices.protocol` 仍登记协议类型，但派发器本切片只走模拟网关通道。
2. **设备类型首版范围：四类全做**（AGV / PTL / DWS / RFID）。`wcs_tasks.task_type` 六类（`pod_move` / `ptl_light_on` / `ptl_light_off` / `sorter_divert` / `dws_weigh` / `rfid_scan`）中，本切片实现 `pod_move` / `ptl_light_on` / `ptl_light_off` / `dws_weigh` / `rfid_scan` 五类与对应事件（`ptl_press` / `rfid_batch` / `dws_result` / `heartbeat`）；`sorter_divert`（分拣）仅登记类型与约束，不实现派发（见 §2.2）。

## 2. 范围与非范围

### 2.1 范围（本阶段必做）

| 能力 | 内容 |
|---|---|
| 中台四表基线 | `iot_devices` / `location_device_bindings`（既有基线，本阶段补权限与绑定 API）/ `wcs_tasks` / `iot_event_logs` 建齐 + `warehouse_locations.agv_unreachable_at` 列 |
| 设备生命周期 | 注册、档案维护、启停、心跳在线判定、离线告警（H4）、绑定/解绑/换绑（软解绑 `valid_to`） |
| 指令-事件闭环 | 指令生成（业务事务内）→ 派发 → 回执/事件 → 校验 → 账务联动同事务落账；五类指令 × 对应事件 |
| PTL 拍灯 | `ptl_light_on` 亮灯 → `ptl_press` 拍灯确认 → 数量落账（差异规则见 §10.3）；`ptl_light_off` 收尾 |
| AGV 货到人 | `pod_move` 下发（payload 货架+工作站）→ 同步回执推进状态 → `agv_unreachable_at` 不可达标记（executing 置位 / 终态清除）→ 格口作业前置校验 |
| DWS / RFID | `dws_weigh` → `dws_result`（重量/体积/通过）确认落账；`rfid_scan` → `rfid_batch`（EPC 集合匹配）确认落账 |
| 超时重试与人工介入 | 退避重试（1/5/15 分钟，max_retries=3）→ 耗尽 `failed` → 重发 / 作废 / 跳过确认 |
| 设备大盘 | 设备列表（在线/离线/启停）、异常任务列表（失败/超时）、受影响库位与待作业任务 |
| 模拟器先行 | 测试内联模拟网关（可编程回执/事件）+ 开发 Mock；真机协议驱动不在范围 |
| 治理收口 | OpenAPI、质量矩阵、菜单、page-query、H4 告警种子、错误码、M-CG 编号 |

### 2.2 非范围（禁止本阶段实现）

| 标签 | 内容 | 理由 |
|---|---|---|
| 真机协议 | 厂商协议驱动（HTTP 长轮询、TCP/Modbus-TCP/MQTT 解析、RCS 对接、厂商 SDK） | 决策基线 1：模拟器先行，真机证据后置 |
| sorter_divert 派发 | 分拣指令下发与分拣到位确认 | 设备类型值域（§6.1）无分拣机类型；`sorter_divert` 指令类型先登记（CHECK 值域含），派发随后续设备接入切片实现 |
| 波次三层兜底 / 拣选中断 | 波次缺口三层兜底、拣选中断恢复 | P1 已延期，见 P2 规格 §2.2 |
| 立库堆垛机 / stacker | `device_type='stacker'` 仅登记类型，不实现 | 四类全做范围不含 stacker |
| 设备自检协议 | 设备自检（自检握手、固件版本、IO 映射探测） | 真机切片随厂商协议一并实现；本切片以心跳+回执近似 |
| 拍灯颜色/闪烁高级指令 | `ptl_light_on` 仅支持单色提示，payload 扩展字段保留 | 最小闭环 |
| 事件网关守护进程 | 独立常驻事件消费进程（IPC/消息队列） | 本切片事件处理为调度作业轮询 + 同步回执双通道，见 §10.2 |
| 客户端推送 | 设备侧主动推送通道（WebSocket/长连接） | 模拟器先行；轮询与同步回执足够 |

### 2.3 与源文档的已裁定分歧（实施以本规范为准）

| 源文档 | 本规范裁定 |
|---|---|
| 模型 §6.6「Phase 2（v1 前）建齐三表结构」 | 实际基线未建（P2 明确空表不进补货切片）→ 本切片 S1.2 迁移建齐三表与 `agv_unreachable_at`，属 v1 前直接改基线，不算越界 |
| 模型 §6.5「绑定前设备须在线并通过自检」 | 自检无协议支撑 → 本切片仅要求设备档案存在且 `online_status != 'disabled'`；在线自检留真机切片 |
| 模型 §6.2「派发器下发至设备」 | 本切片派发 = 写入模拟网关（事务内同步回执）或进入待派发队列；真机驱动后置 |

## 3. 领域对象

全部表 schema 与字段语义以 `storage-location-model.md` §6.1 为唯一事实源，本规范仅列行为要点：

- **iot_devices**：设备主档。仓库级共享资产，不按货主隔离（无 `owner_id`）；`device_code` 仓库内唯一；`online_status ∈ {online, offline, disabled}`；`enabled` 启停开关。
- **location_device_bindings**：库位-点位绑定。仅 `ptl_light` / `rfid_antenna` 两类角色；同一库位同一角色同时仅一条生效绑定（部分唯一索引）；解绑软解绑（置 `valid_to`）。
- **wcs_tasks**：指令任务。`owner_id` 货主隔离（指令服务于货主业务）；`task_no` 走 M-CG；`idempotency_key` 唯一（业务动作 + 指令类型）；六态状态机；`version` 乐观锁。
- **iot_event_logs**：硬件事件。纯审计追加流，**只 INSERT 禁止 UPDATE/DELETE**；`task_id` 关联指令；`event_type ∈ {ptl_press, rfid_batch, dws_result, heartbeat}`。
- **warehouse_locations.agv_unreachable_at**：AGV 格口临时不可达标记（§6.4）。

## 4. 不变量

| 编号 | 不变量 | 违反时 |
|---|---|---|
| I1 | `wcs_tasks` 六态迁移只允许：`pending→sent→executing→succeeded`；`sent/executing/timeout→sent`（重试，`retry_count+1`）；任意非终态→`failed`；终态不可再迁移 | 422 `M1_WCS_TASK_STATE_INVALID` |
| I2 | 同一业务动作 + 同一指令类型只生成一条 `wcs_tasks`（`idempotency_key` 唯一） | 幂等返回已存在任务（200） |
| I3 | 同一 PTL 设备同一时刻最多一个未终态 `ptl_light_on` 任务（亮灯互斥） | 409 `M1_PTL_LIGHT_BUSY` |
| I4 | 同一货架（`payload.pod_code`）同一时刻最多一个未终态 `pod_move` 任务（一托一搬） | 409 `M1_POD_MOVE_ACTIVE` |
| I5 | `pod_move` 执行期间目标货架格口 `agv_unreachable_at` 非空；格口账务动作（拣选/上架/补货/移库/报损）前置校验不可达标记 | 422 `M1_LOCATION_UNREACHABLE` |
| I6 | 终态回执重复到达幂等忽略（事件仍记日志，任务不重复落账） | 忽略 |
| I7 | 账务确认与任务状态推进同事务；落账校验失败 → 任务回 `failed` 且业务账不回 | 任务 `failed` + 人工介入 |
| I8 | 事件→任务匹配按 `task_id`；无匹配任务的 `ptl_press` 挂起等待短窗口（默认 30 秒）后告警 | H4 `device_event_orphan` |
| I9 | 重试仅从 `sent / executing / timeout` 发起，`retry_count <= max_retries`；耗尽进入 `failed` | 人工介入 |
| I10 | 设备 `enabled=false` 或 `online_status='disabled'`：不再下发新指令、活跃任务停止重试并告警 | 422 `M1_DEVICE_DISABLED` / H4 `device_task_stalled` |
| I11 | `iot_event_logs` 只 INSERT | 审计门禁（`check_audit_trail_coverage`） |

## 5. 状态机

**wcs_tasks**：`pending → sent → executing → succeeded`；异常：`pending/sent/executing/timeout → failed`（重试耗尽或落账失败）；`sent/executing/timeout → sent`（重试，`retry_count+1`）；`timeout` 由超时扫描从 `sent / executing` 置位。终态 = `succeeded / failed`；`timeout` 非终态（可重试）。

**iot_devices.online_status**：`offline → online`（心跳到达/人工确认）；`online → offline`（心跳阈值超时，扫描作业置位并告警）；`disabled` 由人工启停，不参与自动迁移；`enabled` 开关独立于 `online_status`。

**location_device_bindings**：生效（`valid_to IS NULL`）→ 软解绑（置 `valid_to`）；无其他状态迁移。

## 6. API / 权限 / 错误码 / 审计 / 幂等

### 6.1 路径表

| 方法 | 路径 | 权限 | 说明 |
|---|---|---|---|
| POST | `/api/v1/iot-devices` | `m1.device.manage` | 设备注册 |
| GET | `/api/v1/iot-devices` | `m1.device.manage` / `m1.device.monitor` | 设备列表（筛选：仓库/类型/在线状态/启停） |
| GET | `/api/v1/iot-devices/{device_id}` | 同上 | 设备详情（含绑定与近期事件） |
| PATCH | `/api/v1/iot-devices/{device_id}` | `m1.device.manage` | 档案维护（编码/型号/IP 端口/extra_config）与启停 |
| POST | `/api/v1/iot-devices/{device_id}/heartbeat` | `m1.device.manage`（模拟器通道） | 心跳上报（模拟网关/真机后置同入口） |
| POST | `/api/v1/iot-devices/{device_id}/events` | `m1.device.manage`（模拟器通道） | 事件上报（`ptl_press`/`rfid_batch`/`dws_result`/`heartbeat`） |
| POST | `/api/v1/location-device-bindings` | `m1.device-bind.manage` | 绑定（角色/点位地址/有效期）；冲突 409 |
| POST | `/api/v1/location-device-bindings/{binding_id}/unbind` | `m1.device-bind.manage` | 软解绑（置 `valid_to`，带原因） |
| GET | `/api/v1/wcs-tasks` | `m1.device.manage` / `m1.device.monitor` | 指令任务列表（筛选：状态/类型/设备/业务引用） |
| GET | `/api/v1/wcs-tasks/{task_id}` | 同上 | 任务详情（含回执与事件链） |
| POST | `/api/v1/wcs-tasks/{task_id}/dispatch` | `m1.device.manage`（模拟器通道） | 受控模拟派发（`pending → sent`） |
| POST | `/api/v1/wcs-tasks/{task_id}/receipt` | `m1.device.manage`（模拟器通道） | 受控模拟回执（`start` / `success` / `fail`） |
| POST | `/api/v1/wcs-tasks/{task_id}/resend` | `m1.device.manage` | 人工重发（重置 `retry_count` 重新入队；仅 `failed`/`timeout`） |
| POST | `/api/v1/wcs-tasks/{task_id}/void` | `m1.device.manage` | 人工作废（仅未落账任务；作废时按 §10.5 补偿并记原因） |
| POST | `/api/v1/wcs-tasks/{task_id}/confirm-skip` | `m1.device.manage` | 跳过确认（现场已人工完成，凭证据补录账务，记录操作人） |
| GET | `/api/v1/iot-events` | `m1.device.manage` / `m1.device.monitor` | 事件流查询（设备/时间窗/类型；只读） |
| GET | `/api/v1/device-dashboard` | `m1.device.monitor` | 设备大盘（设备状态汇总、受影响库位、待作业任务） |

### 6.2 错误码（登记 `docs/error-codes.md` §6）

| 错误码 | 场景 |
|---|---|
| `M1_DEVICE_DUPLICATE_CODE` | 设备编码仓库内重复 |
| `M1_DEVICE_TYPE_INVALID` | 设备类型非法（CHECK 外业务判定） |
| `M1_DEVICE_VERSION_CONFLICT` | 设备档案乐观锁版本冲突 |
| `M1_DEVICE_NOT_FOUND` | 设备不存在 |
| `M1_DEVICE_DISABLED` | 设备已停用，禁止下发/重试 |
| `M1_DEVICE_OFFLINE` | 设备离线，禁止绑定新点位/下发（降级路径除外） |
| `M1_BIND_CONFLICT` | 同一库位同一角色已有生效绑定 |
| `M1_BIND_DEVICE_MISMATCH` | 绑定角色与设备类型不匹配（ptl_light↔`ptl_light` 设备，rfid_antenna↔`rfid_antenna`） |
| `M1_BIND_LOCATION_MISMATCH` | 库位不存在或不属于设备所在仓库 |
| `M1_BIND_NOT_FOUND` | 绑定不存在/已解绑 |
| `M1_WCS_TASK_NOT_FOUND` | 指令任务不存在 |
| `M1_WCS_TASK_STATE_INVALID` | 状态迁移非法（I1） |
| `M1_WCS_TASK_IDEMPOTENCY_CONFLICT` | 幂等键冲突但业务动作不一致（异常防御） |
| `M1_WCS_TASK_RETRY_EXHAUSTED` | 重试耗尽，任务终态 `failed` |
| `M1_WCS_TASK_VOID_BLOCKED` | 已落账任务不可作废 |
| `M1_PTL_LIGHT_BUSY` | 同一 PTL 已有未终态亮灯任务（I3） |
| `M1_PTL_QTY_DIFF_EXCEEDED` | 拍灯数量与提示数量差异超阈值（±20% 或绝对值 >10），强阻断 |
| `M1_POD_MOVE_ACTIVE` | 同一货架已有未终态搬运任务（I4） |
| `M1_LOCATION_UNREACHABLE` | 格口处于 AGV 不可达期，账务动作被阻断（I5） |
| `M1_AGV_MARKER_INCONSISTENT` | 不可达标记与活跃 `pod_move` 不一致（校验兜底） |
| `M1_EVENT_TASK_MISMATCH` | 事件与任务不匹配（类型/载荷/库位校验失败） |
| `M1_EVENT_ORPHAN` | 无匹配任务的业务事件，短窗口后未匹配 |
| `M1_NUMBERING_UNAVAILABLE` | M-CG `wcs_task` 无编号规则 |

### 6.3 审计、编号、事件

- **编号**：`wcs_tasks.task_no` 走 M-CG 编号规则，`document_type='wcs_task'`；无规则 → 422 `M1_NUMBERING_UNAVAILABLE`。
- **审计**：设备注册/启停/绑定/解绑/重发/作废/跳过确认为管理动作，全部写审计（`audit_events`）；`iot_event_logs` 为纯审计追加流（只 INSERT）；绑定解绑走既有审计模式。
- **幂等**：指令生成按 `idempotency_key` 幂等（业务动作 ID + 指令类型）；事件处理按事件 ID 幂等（重复投递忽略）；写路径统一 `Idempotency-Key` 头（与 P2 一致）。
- **H4 告警种子**（六个 event_type）：`device_offline`（心跳超时）、`device_event_orphan`（孤儿事件）、`wcs_task_failed`（重试耗尽）、`wcs_task_stalled`（设备停用致活跃任务停滞）、`ptl_qty_diff`（拍灯数量差异未超阈值但需复核）、`agv_marker_inconsistent`（不可达标记不一致）。

## 7. 表与迁移要点（v1 前直接改基线）

新迁移 `2026082X0001_m1_device_platform_baseline.sql`（单一迁移，建齐）：

1. `iot_devices`（§6.1 schema 原样）+ 索引 + GRANT `SELECT,INSERT,UPDATE`（无 DELETE；`wms_app`）；
2. `wcs_tasks`（§6.1 原样，含 `task_no`/`idempotency_key` UNIQUE、六态 CHECK、`version`）+ 索引 + GRANT `SELECT,INSERT,UPDATE`；
3. `iot_event_logs`（§6.1 原样）+ 索引 + GRANT `SELECT,INSERT`（**无 UPDATE/DELETE**）；
4. `location_device_bindings` 已存在基线：仅补 GRANT 与 `m1.device-bind.manage` 权限种子（若缺）；
5. `warehouse_locations` 增列 `agv_unreachable_at TIMESTAMPTZ`（默认 NULL，注释说明语义）；
6. 权限种子：`m1.device.manage` / `m1.device.monitor` / `m1.device-bind.manage`（既有 `auth_permissions` 模式 + 角色触发器）；
7. 系统字典：`document_type=wcs_task`（M-CG 可编）；设备类型/在线状态值域由 CHECK 承担，不入字典；
8. H4 告警定义种子（§6.3 六条，既有 `alert_definitions` 模式）。

## 8. 页面与分层

| 页面 | 页面族 | 归属票 | 说明 |
|---|---|---|---|
| 设备档案页 | 配置型（双栏/表格+详情弹窗） | T02 | 注册/启停/心跳展示，`apps/web-admin/src/pages/master/M1DevicePage.tsx`，自检 `m1-device-self-check.mjs` |
| 绑定管理页 | 配置型 | T02 | 库位-设备点位绑定/解绑（含受影响库位提示） |
| 设备大盘 | 列表型（ListPageTemplate + QueryPanel + DataGrid） | T06 | 设备状态汇总、受影响库位、待作业任务；模拟器派发 / 回执 |
| 异常任务大盘 | 列表型 | T06 | 失败/超时任务 + 重发/作废/跳过确认 Dialog |

分层：`page -> features/device -> @wms/api-client`；禁止裸 fetch；`page-query-core-fields.json` 登记设备/任务列表查询核心字段；菜单 `m1-devices` / `m1-device-dashboard`。

## 9. 与既有模块的接口约定

| 模块 | 约定 |
|---|---|
| M-TE | 本切片调度作业（心跳扫描、超时重试扫描、孤儿事件窗口）注册进既有调度骨架（沿 `replenishment_timeout_job` 同模式，见 P2 遗留 §3.6 事项）；不写 M-TE 单据表 |
| H4 | 离线/失败/差异/停滞/孤儿/标记不一致六类告警走 `alert_definitions` 种子 + 事件总线 `business.` 前缀（对齐 P2 先例） |
| 库存/波次/补货 | PTL/RFID/DWS 确认落账复用既有 inventory 上下文命令（`confirm_replenish_in_tx` 等）；本切片不新造库存写入；`pod_move` 不落库存账 |
| 基础档案 | 库位向导/库位档案已有 `is_agv_managed` / `agv_pod_code` / `pick_sequence_no` 字段（P1 已建），本切片只读消费，不改语义 |
| 审计 | 管理动作走既有审计表；事件走 `iot_event_logs`（只 INSERT） |
| 波次 | `sorter_divert` 仅登记类型不派发；波次进度不依赖本切片 |

## 10. 引擎算法（可测）

### 10.1 指令生成与派发

- 生成：业务事务内插入 `wcs_tasks`（`pending`），`idempotency_key = {business_ref_type}:{business_ref_no}:{task_type}`；同键已存在 → 返回已有任务（幂等）。
- 派发：模拟网关通道由受权限控制的 `dispatch` API 驱动，`pending → sent`（写 `sent_at`）；`receipt` API 接收模拟器「开始」回执推进 `executing`、「成功」回执校验后推进 `succeeded`（账务联动见 10.3-10.4）、「失败」回执按 10.5 重试。两个模拟器写接口均要求 `m1.device.manage`、`Idempotency-Key` 并写审计；真机协议驱动后置。
- 亮灯互斥（I3）：生成 `ptl_light_on` 前查该设备未终态亮灯任务，存在 → 409 `M1_PTL_LIGHT_BUSY`。

### 10.2 事件处理（异步事件通道）

- 事件经 `POST /iot-devices/{id}/events` 落 `iot_event_logs`（只 INSERT）→ 按 `event_type` 分发：
  - `ptl_press`：按 `task_id` 匹配 `ptl_light_on` 任务 → 校验库位/设备 → 数量差异判定（§10.3）→ 落账 + `succeeded`；
  - `rfid_batch`：按 `task_id` 匹配 `rfid_scan` 任务 → 校验 EPC 集合是否覆盖目标集合 → 覆盖则落账 + `succeeded`，否则 `failed`（`M1_EVENT_TASK_MISMATCH`）；
  - `dws_result`：按 `task_id` 匹配 `dws_weigh` 任务 → 校验 `pass=true` 且重量在预估 ±20% 内 → 落账 + `succeeded`，否则 `failed`；
  - `heartbeat`：更新 `last_heartbeat_at` / `online_status='online'`（无任务，不落账）。
- 无匹配任务：`ptl_press` 挂起等待 30 秒窗口（内存+落库 pending 标记 `task_id` 空），窗口内任务到达则正常处理，超窗写 H4 `device_event_orphan`；其他事件类型无任务 → 直接记录 + H4 `device_event_orphan`。
- 事件处理幂等：同事件 ID 重复投递忽略（`iot_event_logs` 主键幂等）。
- 本切片事件处理 = 同步处理（请求内完成）+ 调度扫描兜底（重试/孤儿窗口）；不建常驻消费进程（§2.2）。

### 10.3 PTL 拍灯数量规则

- 拍灯数量 = 提示数量 → 正常确认。
- 拍灯数量 ≠ 提示数量：以拍灯数量为实际确认数量落账（拍灯即确认），记录差异并 H4 `ptl_qty_diff`；差异超阈值（`|Δ| / 提示数量 > 20%` 或 `|Δ| > 10`，可配置）→ 422 `M1_PTL_QTY_DIFF_EXCEEDED`，任务回 `failed`，人工介入。
- 同任务重复拍灯（已 `succeeded`）→ 忽略，事件仍记录。

### 10.4 AGV 不可达与账务联动

- `pod_move` 生成：校验同一货架无未终态搬运任务（I4）→ `pending`；payload `{pod_code, target_station}`。
- `executing` 回执：同事务置该货架全部格口库位（`agv_pod_code = payload.pod_code`）`agv_unreachable_at = now()`。
- `succeeded` / `failed` / 作废：同事务清除不可达标记。
- 格口账务前置：上架/补货/拣选/移库/报损写入路径校验目标库位 `agv_unreachable_at IS NULL`，否则 422 `M1_LOCATION_UNREACHABLE`。
- 校验兜底：调度扫描核对「活跃 `pod_move` ↔ 不可达标记」一致性，不一致 → H4 `agv_marker_inconsistent`。
- `pod_move` 不产生库存账变（I7 之外纯状态推进）。

### 10.5 超时重试与人工介入

- 超时扫描（调度作业 `iot_wcs_timeout_scan`，每分钟）：`sent / executing` 超过指令超时（默认 120 秒）→ `timeout`；未耗尽重试 → 回 `sent` 重新派发（间隔退避 1/5/15 分钟，`retry_count+1`，`version+1`）；耗尽 → `failed` + H4 `wcs_task_failed`。
- 心跳扫描（调度作业 `iot_heartbeat_scan`，每 30 秒）：`last_heartbeat_at` 超过阈值（默认 90 秒，配置键 `device.heartbeat_timeout_secs`）→ `online_status='offline'` + H4 `device_offline`（恢复心跳回 `online`）。
- 人工介入（管理端）：`resend`（仅 `failed`/`timeout`，重置 `retry_count` 重新派发）；`void`（仅未落账任务：置 `failed`，`ack_payload` 记作废原因；若曾置不可达标记则同事务清除并补偿业务账——本切片 `void` 仅允许未生成账务动作的任务）；`confirm-skip`（现场已人工完成：按 §10.3-10.4 规则凭证据补录账务并 `succeeded`，审计记录操作人）。
- 设备停用（`enabled=false`）：活跃未终态任务停止重试（停留原态）+ H4 `wcs_task_stalled`；新指令生成 422 `M1_DEVICE_DISABLED`。

### 10.6 绑定与降级

- 绑定：`location_device_bindings` 插入（`valid_to IS NULL`）；同库位同角色已有生效绑定 → 409 `M1_BIND_CONFLICT`；绑定角色与设备类型不匹配（`ptl_light` 角色 ↔ 非 `ptl_light` 设备）→ 422 `M1_BIND_DEVICE_MISMATCH`；设备离线/停用 → 422 `M1_DEVICE_OFFLINE` / `M1_DEVICE_DISABLED`。
- 解绑：软解绑（置 `valid_to=now()` + 原因），保留历史链。
- 降级：设备离线/停用/绑定到期 → 该库位 PTL/RFID 作业降级人工扫码确认（记录「降级人工确认」与原因；本切片在任务侧暴露降级标记，PDA 扫码确认流程由既有作业链路承载，不新建 PDA app）。

## 11. Given / When / Then（GWT 1-22）

1. 注册 `device_code=PTL-01`、`device_type=ptl_light` → 201，`online_status=offline`、`enabled=true`。
2. 同仓库重复注册同编码 → 409 `M1_DEVICE_DUPLICATE_CODE`。
3. 注册 `device_type=robot`（非法）→ 422 `M1_DEVICE_TYPE_INVALID`。
4. 绑定 PTL-01 到库位 L1（角色 ptl_light）→ 201；再绑 L1 同角色 → 409 `M1_BIND_CONFLICT`。
5. 绑定 `rfid_antenna` 角色到 `ptl_light` 设备 → 422 `M1_BIND_DEVICE_MISMATCH`。
6. 绑定离线设备 → 422 `M1_DEVICE_OFFLINE`。
7. 心跳上报后 `online_status=online`；心跳停 90 秒+扫描 → `offline` + H4 `device_offline`。
8. `enabled=false` 后生成新指令 → 422 `M1_DEVICE_DISABLED`；活跃任务停滞 + H4 `wcs_task_stalled`。
9. 生成 `ptl_light_on`（幂等键 K）→ 201 `pending`；同键重复 → 200 返回同任务（不重复插入）。
10. 同一 PTL 未终态亮灯任务存在时再生成 → 409 `M1_PTL_LIGHT_BUSY`。
11. 派发成功回执 → `sent→executing→succeeded`；`succeeded` 后重复回执 → 忽略（状态不变）。
12. 拍灯数量 = 提示数量 → 目标在手 +Δ、`succeeded`（账务同事务）。
13. 拍灯数量 ≠ 提示数量（未超阈值）→ 按拍灯量落账 + H4 `ptl_qty_diff`。
14. 拍灯数量差异超阈值 → 422 `M1_PTL_QTY_DIFF_EXCEEDED`，任务 `failed`，账不回。
15. 无匹配任务 `ptl_press` → 30 秒窗口内任务到达正常处理；超窗 → H4 `device_event_orphan`。
16. `pod_move` 生成 → `executing` 回执后格口 `agv_unreachable_at` 非空；该格口上架 → 422 `M1_LOCATION_UNREACHABLE`；`succeeded` 后标记清除，上架恢复。
17. 同一货架已有未终态 `pod_move` 再生成 → 409 `M1_POD_MOVE_ACTIVE`。
18. `pod_move` 全程库存账不变；`succeeded` 仅推进状态与清除标记。
19. `dws_result pass=false` → 任务 `failed`，不落账；`pass=true` 且重量在 ±20% → 落账 `succeeded`。
20. `rfid_batch` EPC 集合覆盖目标集合 → 落账 `succeeded`；缺 EPC → `failed`（`M1_EVENT_TASK_MISMATCH`）。
21. 重试耗尽（3 次失败回执）→ `failed` + H4 `wcs_task_failed`；`resend` → 重置重试重新派发；已落账任务 `void` → 422 `M1_WCS_TASK_VOID_BLOCKED`。
22. 不可达标记与活跃 `pod_move` 不一致 → 扫描告警 H4 `agv_marker_inconsistent`。

## 12. 测试层级（本阶段必做）

| 层 | 内容 |
|---|---|
| domain 单测 | 状态机迁移（I1）、幂等判定、PTL 差异阈值、EPC 覆盖判定、不可达校验（`cargo test -p wms-domain device`） |
| postgres 集成 | `m1_device_platform_postgres.rs`：三表建齐/CHECK/GRANT、注册/绑定/心跳/派发/事件/落账/重试/人工介入（`--test-threads=1`） |
| 引擎可测 | 派发器/事件处理/超时扫描/心跳扫描为纯函数或依赖注入时钟，单测覆盖 GWT 1-22 |
| 前端 | `m1-device-self-check.mjs` / `m1-device-dashboard-self-check.mjs` |
| 治理 | `just openapi-sync` 无漂移、`check_error_codes.py`、质量矩阵 `api_paths`、`page-query-core-fields.json`、菜单种子 |

## 13. 实施边界

- 分层：`handler → service → repository`；引擎规则入 domain（纯函数）；`iot_event_logs` 只 INSERT。
- 禁止：真机协议驱动、常驻事件消费进程、改 P1 库位/补货/波次语义、`outbound_order_lines` 加列、新建推送网关、PDA 生产 app（ADR-0027 未 Accepted）。
- 迁移：v1 前直接改基线（ADR-0038）；单迁移建齐，命名 `2026082X0001_m1_device_platform_baseline.sql`。
- 文件规模：后端单文件 >=800 行拆分或说明例外；前端页面 <800 行。
- 提交：一票一主题（文档+接口落地拆两个 commit），中文 Conventional Commits。

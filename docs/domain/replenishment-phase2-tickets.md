# Phase 2 补货工单清单（US-M3-012 / ADR-0048 Phase 2）

- 状态：Frozen（阶段 C 三轮清单审查通过，2026-08-19）
- 规格：`docs/domain/replenishment-phase2-spec.md`（Frozen）
- 开发模式：每票外向内 TDD，先失败测试再实现
- 提交：一票一主题；若票内含文档+接口，落地时拆两个 Conventional Commit，不混进同一 commit
- 阶段 D 第一票前钉死 P2 起点 SHA：`95290f005dd54655b47f7fc1ccc35661f1c8f36d`

## 总览与依赖

```
T01 domain 不变量 ─────────────────────────────┐
T02 表/权限/错误码/字典 ── T03 库存三命令 ─────┼── T05 任务生成
                                               ├── T04 策略/组/预览/挂接 ── T11 策略页
T05 ── T06 领取下架确认 ── T07 取消改派退回挂起
     ├── T08 Min-Max 巡检
     ├── T09 波次缺口+事件
     └── T10 超时扫描（前置 T07 的取消回冲）
T06+T07 ── T12 任务大盘
T06      ── T13 PDA 列表契约（不建生产 PDA app）
T04+T06+T11+T12 ── T14 治理收口
```

共 14 票。P1 / P3 / ADR-0047 本切片不做 / US-RP-001/004/005 一律不进票。

---

## T01 — Domain 不变量（动线、可用量、任务量、状态守卫）

**What to build:** 保管员/主管保存非法动线会被拒绝；巡检算得出「该不该补、补多少」；作业动作在非法状态下被挡住。纯函数，无 IO。

**Blocked by:** 无

**主题：** `测试(库存)`（若同票补实现则 `功能(库存)` 可与测试同主题合并；禁止夹带 HTTP/SQL）

**先失败测试：** `validate_replenish_route(piece_pick, storage)` 返回非法；当前无此函数 → 编译/断言失败。

**一条可失败验收：** `validate_replenish_route(piece_pick, storage)` 为错误；`available_qty` / `task_qty`（包装向下取整，不足 1 包装为 0）与状态守卫（`picked_qty>0` 不可 cancel）可单测（GWT 1/19 的 L1 面）。

**模块/文件边界：**

- 新建 `backend/crates/domain/src/replenishment.rs`，`lib.rs` 导出
- 单测放同文件 `#[cfg(test)]` 或 `backend/crates/domain/tests/replenishment.rs`
- 禁止改 handler / 迁移 / 前端

**不做：** SQL、`FOR UPDATE`、时间源直读、库存增减、OpenAPI、页面、P1 容器质量锁语义。

**验证：**

```bash
cargo test -p wms-domain replenish
just gov-t1
```

---

## T02 — 任务表、策略约束、权限、错误码、单据类型

**What to build:** 库里能落下补货任务行；策略水位非法插不进；有 `m3.replenishment.manage` / `execute`；错误码字典能查到 `M3_REPLENISH_*`；M-CG 能编 `replenishment_task` 号。

**Blocked by:** 无（可与 T01 并行）

**主题：** 落地拆两个 commit：`文档(治理)：登记 M3_REPLENISH_*` + `功能(接口)：补货任务表与权限种子`

**先失败测试：** postgres 测试插入 `replenishment_tasks` 成功、插入 `min >= max` 的策略失败；表不存在 → 红。

**一条可失败验收：** 基线库存在 `replenishment_tasks`（含 `suspended` / `claimed_at` / `last_progress_at` / 波次外键列）且策略表有水位 CHECK（规范 §7）。

**模块/文件边界：**

- `backend/migrations/` 新脚本（v1 前直接改基线）
- `docs/error-codes.md` §6
- 权限种子（同既有 `auth_permissions` 模式）
- 系统字典 `document_type=replenishment_task`
- 禁止业务 handler、禁止改 P1 锁/6 维表语义、禁止 IoT 四表

**不做：** 引擎、PDA、出库等待列、`assigned_product_id`、`iot_devices`/`wcs_tasks`/`iot_event_logs`。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_schema_postgres -- --test-threads=1
python3 scripts/governance/check_error_codes.py
just gov-t1
```

无 `DATABASE_URL` 时 schema 测试写入证据说明未跑原因，不否决 T02 的错误码/治理部分。

---

## T03 — 库存补货三命令

**What to build:** 生成占用在途、确认转在手、取消回冲，全部走既有 inventory 上下文；补货 repository 仍不能直接改 `qty_*`。

**Blocked by:** T02（错误码存在即可；在途列已在 Phase 1）

**主题：** `功能(库存)`

**先失败测试：** `reserve_replenish_in_tx` 后目标 `in_transit+=Δ`、来源 `out_transit+=Δ`、`on_hand` 不变；函数不存在 → 红。

**一条可失败验收：** GWT 2 的账务面（仅命令、不经任务表）：reserve 不写 `inventory_movements`；confirm 写两条 `movement_type=replenish`；release 只回冲在途。

**模块/文件边界：**

- `backend/crates/api/src/inventory.rs`（或同上下文子模块）
- postgres 测试 `m3_replenishment_inventory_postgres.rs`
- 禁止新建 Inventory Service、禁止补货 repository 内联 `UPDATE inventory_batches.qty_*`

**不做：** 任务状态机、巡检、页面、改 P1 加解锁。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_inventory_postgres -- --test-threads=1
just gov-t1
```

---

## T04 — 策略、库位组、挂接、预览 API

**What to build:** 主管能配置策略与库位组、预览命中位、把拣选位挂到策略；非法动线 422；库位已挂其他策略 409。

**Blocked by:** T01、T02

**主题：** `功能(接口)`

**先失败测试：** POST `source_type=piece_pick, target_type=storage` → 422 `M3_REPLENISH_STRATEGY_INVALID`（GWT 1）。

**一条可失败验收：** GWT 1 + GWT 22 + GWT 23 + GWT 30（非法动线 / 缺幂等键 / 挂接冲突 / 非法 category scope_ref）。本票同步本票路径的 OpenAPI。

**模块/文件边界：**

- `replenishment_handlers` / `replenishment_service` / `replenishment_repository`
- OpenAPI `openapi_paths` 策略与组路径
- 禁止任务引擎、禁止改出库行、禁止前端页面（页面在 T11）

**不做：** Min-Max 真正生成任务、PDA、P3 硬件。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_strategy_postgres -- --test-threads=1
just gov-t1
```

---

## T05 — 任务生成（FEFO + 双字段 + 编号 + 手工发起）

**What to build:** 系统或主管发起一条补货任务时，同事务锁定来源批次、占用在途双字段、编任务号；一任务一批次。

**Blocked by:** T01、T02、T03

**主题：** `功能(接口)`

**先失败测试：** 调用生成后任务 `pending`、`qty` 为包装取整、目标 `in_transit` 增加；当前无生成入口 → 红。

**一条可失败验收：** GWT 2（可用 service 直接生成，不必等巡检）。两任务抢同一来源批次时后者可下架量不足则不生成（L6）。手工 POST `/tasks` 走同一生成。本票同步 POST `/tasks` OpenAPI。

**模块/文件边界：**

- service `create_task` / 生成编排
- repository 来源 `FOR UPDATE` + 插任务
- 调用 T03 三命令的 reserve
- M-CG `replenishment_task`；无规则 → 422 `M3_REPLENISH_NUMBERING_UNAVAILABLE`
- 禁止巡检调度、禁止波次模块改出库列

**不做：** claim/pick/confirm、超时作业、IoT、US-RP-001 动态阈值。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_generate_postgres -- --test-threads=1
just gov-t1
```

---

## T06 — 领取、下架、送达确认

**What to build:** 作业员领取任务、扫来源下架、扫目标送达并账面转换；一人一任务；幂等重放不加倍。

**Blocked by:** T05

**主题：** `功能(接口)`

**先失败测试：** pick 扫描错误库位 → 422 `M3_REPLENISH_SOURCE_MISMATCH`（GWT 18）。

**一条可失败验收：** GWT 5、6、7、8、12、13、15、16、18、29（冲突领取 / 超量 / 确认转账 / 部分确认 / 6 维③ 阻断 / 幂等 / 无权限 403 / 整托释箱 / 扫码不符 / 未下架确认）。本票同步作业路径 OpenAPI。

**模块/文件边界：**

- handler：`/claim` `/pick` `/confirm`
- service 状态机 + T03 confirm
- 整托完成释箱（GWT 16）
- 禁止取消/超时、禁止改 P1 6 维函数语义（只调用）

**不做：** 取消回冲、巡检、页面、拍灯、`wcs_tasks`。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_job_postgres -- --test-threads=1
just gov-t1
```

---

## T07 — 取消、改派、退回、来源冻结挂起

**What to build:** 主管取消未下架任务并回冲；改派回池；作业员退回；来源冻结后任务挂起，已下架量仍可送达。

**Blocked by:** T06

**主题：** `功能(接口)`

**先失败测试：** `picked_qty=4` 时 cancel → 422 `M3_REPLENISH_CANCEL_BLOCKED`（GWT 9）。

**一条可失败验收：** GWT 9、10、17、24、25、31。

**模块/文件边界：**

- handler：`/cancel` `/reassign` `/return`
- service：`suspended` + T03 release（仅取消）
- H4 `replenishment_source_frozen` / `replenishment_source_mismatch` 写出
- 禁止超时扫描（T10）、禁止波次拆单、禁止改出库行

**不做：** 20 分钟自动取消作业、P3、内容物表。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_exception_postgres -- --test-threads=1
just gov-t1
```

---

## T08 — Min-Max 巡检引擎与调度

**What to build:** 白天/夜间按配置巡检已挂策略的拣选位，水位低于 min 则生成任务；在途覆盖后不再生成；容器质量锁来源跳过。

**Blocked by:** T05

**主题：** `功能(接口)`

**先失败测试：** 可用量 2、min=5、max=20 → 生成 qty=18；再跑不生成第二单（GWT 2+3）。无 `run_min_max_patrol` → 红。

**一条可失败验收：** GWT 2、3、11、19、26。

**模块/文件边界：**

- `ReplenishmentService::run_min_max_patrol`
- M-TE 调度骨架注册 `replenishment_min_max`（任务不写入 M-TE 单据表）
- 命中集合按规范 §10.1
- 禁止波次缺口、禁止改策略 CRUD、禁止页面

**不做：** US-RP-001 日均公式、递归补下一条、IoT。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_patrol_postgres -- --test-threads=1
just gov-t1
```

---

## T09 — 波次缺口引擎与事件总线

**What to build:** 波次算单调用缺口引擎生成 urgent 任务，并写入 `event_bus_event` `replenishment.waiting`；不改正文出库行。

**Blocked by:** T05

**主题：** `功能(接口)`

**先失败测试：** demand=10、可用量=3 → 任务 qty=7 且存在 `replenishment.waiting`（GWT 4）。

**一条可失败验收：** GWT 4、27；返回列表可空；`outbound_order_lines` 列数不变。

**模块/文件边界：**

- `ReplenishmentService::create_wave_gap_tasks`
- 写 `event_bus_event`（不新建 outbox 表）
- 禁止实现波次消费/重算单、禁止加出库等待列、禁止 `fail_order`/`hold_wave` 模板

**不做：** 波次三层兜底、拣选中断、US-RP-004 短拣。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_wave_postgres -- --test-threads=1
just gov-t1
```

---

## T10 — 超时扫描（10/20/60 分钟）

**What to build:** 每分钟扫描：urgent 10 分钟告警、20 分钟自动取消回冲并写 `replenishment.cancelled`；领取后 1 小时无进度只告警不取消。

**Blocked by:** T07（复用取消回冲）

**主题：** `功能(接口)`

**先失败测试：** 创建 20 分钟仍 pending 的 urgent，跑 `run_timeout_scan` → `cancelled` + 在途回冲（GWT 14）。

**一条可失败验收：** GWT 14、28。

**模块/文件边界：**

- `ReplenishmentService::run_timeout_scan`
- 调度作业 `replenishment_timeout`
- 禁止改策略、禁止 PDA 生产 app、禁止自动取消 `in_progress`

**不做：** 新建推送网关、拍灯。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_timeout_postgres -- --test-threads=1
just gov-t1
```

---

## T11 — PC 补货策略配置页

**What to build:** 主管在独立菜单维护策略、库位组、挂接与命中预览。

**Blocked by:** T04

**主题：** `功能(管理端)`

**先失败测试：** 新增 `apps/web-admin/self-checks/m3-replenishment-strategy-self-check.mjs`：断言配置型分区与 `@wms/api-client` 调用存在；文件未建 → 红。

**一条可失败验收：** 配置型页面分区符合规范 §8；保存走 `@wms/api-client`；本票登记菜单 `m3-replenishment-strategies` 与 `page-query-core-fields.json`。

**模块/文件边界：**

- `apps/web-admin/src/pages/inventory/M3ReplenishmentStrategyPage.tsx`（及页面私有组件，单文件 <800 行）
- `apps/web-admin/src/features/replenishment/`
- 开发 Mock
- 禁止裸 fetch、禁止大盘页（T12）、禁止改 P1 向导

**不做：** 图表/邮件（US-RP-005）、任务大盘操作。

**验证：**

```bash
node apps/web-admin/self-checks/m3-replenishment-strategy-self-check.mjs
just gov-t1
```

---

## T12 — PC 补货任务大盘

**What to build:** 主管看任务列表、筛选、手动发起、重派、取消；超时行高亮。

**Blocked by:** T06、T07

**主题：** `功能(管理端)`

**先失败测试：** 新增 `apps/web-admin/self-checks/m3-replenishment-task-self-check.mjs`：断言固定列与 Dialog 私有动作；文件未建 → 红。

**一条可失败验收：** 列表型 ListPageTemplate + QueryPanel + DataGrid；私有动作走 Dialog；本票登记菜单 `m3-replenishment-tasks` 与 `page-query-core-fields.json`。

**模块/文件边界：**

- `apps/web-admin/src/pages/inventory/M3ReplenishmentTaskPage.tsx`
- 复用 `features/replenishment` hooks
- 禁止策略 CRUD 表单进本页、禁止裸 fetch

**不做：** 导出图表、邮件推送、P3 设备大盘。

**验证：**

```bash
node apps/web-admin/self-checks/m3-replenishment-task-self-check.mjs
just gov-t1
```

---

## T13 — PDA 作业列表契约（不启动生产 PDA app）

**What to build:** execute 权限下任务列表：urgent 置顶 → `pick_sequence_no` → `task_no`；普通任务受班组库区限制；urgent 可跨区。

**Blocked by:** T06

**主题：** `功能(接口)`

**先失败测试：** GWT 20 / 21（跨区拒绝 / urgent 可跨区）。

**一条可失败验收：** GET `/tasks` 以 execute 身份返回的排序与库区过滤符合规范 §8 / §10.6。

**模块/文件边界：**

- 任务列表查询（handler/service/repository）
- 只读 M-TE `task_groups`
- **禁止** 在 `apps/pda-mobile` 新增生产文件（ADR-0027 未 Accepted，T1 门禁会红）
- 禁止离线队列

**不做：** 生产 RN/Expo/Capacitor app、PDA 扫码加锁、离线暂存（跨故事 7 仅移库/盘点）。

**验证：**

```bash
cargo test -p wms-api --test m3_replenishment_list_postgres -- --test-threads=1
just gov-t1
```

---

## T14 — 治理收口（矩阵、OpenAPI、菜单、H4 种子、page-query）

**What to build:** 规范 §15 七件治理物全部落地，脚本可检查。

**Blocked by:** T04、T06、T08、T09、T10、T11、T12

**主题：** 落地拆 commit：`功能(接口)：openapi-sync` 与 `文档(治理)：质量矩阵与 H4 种子`（菜单已由 T11/T12 登记，本票只核漏）

**先失败测试：** `check_quality_matrix` 在 `api_paths` 未扩全前失败；本票补齐后绿。

**一条可失败验收：** `US-M3-012.api_paths` 覆盖规范 §6 全表；H4 六个 event_type 有定义种子；`just openapi-sync` 无未提交漂移。

**模块/文件边界：**

- `governance/quality-matrix.toml` + 生成页
- OpenAPI + `just openapi-sync`
- 管理端菜单种子 / `admin-menu-dev-mock.ts`
- `page-query-core-fields.json`
- H4 告警定义种子
- 禁止回改 P1 行为、禁止建 IoT 四表

**不做：** 波次消费端、Phase 3 硬件。

**验证：**

```bash
just openapi-sync
just gov-t1
```

---

## 全清单「不做」（每票默认继承）

| 标签 | 内容 |
|---|---|
| P1 已延期 | PDA 扫码加锁、波次三层/拣选中断、质量区 7/3 天滞留 |
| 本切片不做 | 内容物表、嵌套、打印、回收状态机 |
| P1 已关闭 | 回改容器质量锁 / 6 维 / 库位向导语义 |
| P2 外 | US-RP-001 动态公式、US-RP-004 短拣、US-RP-005 图表邮件 |
| P3 | AGV/PTL 真硬件、IoT 四表、拍灯、`wcs_tasks` |
| 新字段不建 | `assigned_product_id`、出库行等待补货列 |
| 门禁 | `apps/pda-mobile` 生产启动（ADR-0027） |

## US-M3-012 对照

| 验收 | 工单 |
|---|---|
| #1 策略/组/预览/权限 | T01、T02、T04、T11 |
| #2 双触发 + 等待事件 | T08、T09 |
| #3 生成+双字段+FEFO | T03、T05 |
| #4 PDA 四步 | T06（作业 API）；T13（列表契约）；生产 PDA UI 明确不做 |
| #5 部分执行/整托 | T06 |
| #6 取消 | T07 |
| #7 大盘 | T12 |
| #8 幂等+挂起 | T06、T07 |
| #9 推送/超时/改派 | T07、T10、T13 |
| #10 动线/路径序 | T01、T13 |

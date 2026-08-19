# Phase 2 补货技术规范（US-M3-012 / ADR-0048 Phase 2）

- 状态：Frozen（HEAD `8fee3ac3` 三轮独立 Standards+Spec：规格语义矛盾 0、标准硬违规 0，2026-08-19）
- 日期：2026-08-19
- 规格源：ADR-0048 决策 6–7、里程碑 Phase 2；`docs/domain/storage-location-model.md` §5.1–§5.7；US-M3-012 验收 1–10
- 固定点：阶段 A 收口后工作区；实施起点 SHA 在阶段 D 第一票前钉死
- 模块：M3 库内作业 + M-RP 补货限界；调度骨架复用 M-TE，任务实体不混入 M-TE 单据

## 1. 问题与方案

保管员需要拣选位不断档，仓库主管需要按 Min-Max 与波次缺口自动生成补货任务并在 PC 大盘监控。本阶段落地独立补货策略、双重引擎、任务状态机、在途双字段账务、PC 大盘与 PDA 作业（领取→扫来源下架→扫目标送达→账面转换）。库存增减必须扩展既有库存领域服务（`inventory` 上下文的 in-tx 增减与 `inventory_movements`），禁止补货 repository 直接改 `qty_on_hand`，禁止另起一套库存 Service。

## 2. 范围与非范围

### 2.1 范围（本阶段必做）

| 能力 | 规格锚点 |
|---|---|
| PC 补货策略 CRUD、库位组、命中预览、挂接拣选位 | US-M3-012#1；模型 §5.3、§5.6 |
| Min-Max 巡检引擎 | US-M3-012#2 日常；ADR-0048 决策 7；模型 §5.1.1 |
| 波次缺口即时引擎（`urgent`） | US-M3-012#2 波次；模型 §5.1.2 |
| 任务生成 + 在途双字段同事务 | US-M3-012#3；ADR-0048 决策 6；模型 §5.2 |
| PDA 领取→下架→送达→账面转换 | US-M3-012#4–5；模型 §5.5 |
| 取消 / 超时 / 改派 / 退回 | US-M3-012#6、#9；模型 §5.4–§5.7 |
| PC 任务大盘（列表、筛选、手动发起、重派、取消、超时告警） | US-M3-012#7、#9；模型 §5.6 |
| 权限 `m3.replenishment.manage` / `m3.replenishment.execute` | US-M3-012#1；ADR-0048 决策 7 |
| 与 P1 容器质量锁 / 6 维②③⑥ / 库位形态 / 路径序的接口 | 见第 9 节 |

### 2.2 非范围（禁止本阶段实现）

| 标签 | 内容 | 理由 |
|---|---|---|
| P1 已延期 | PDA 扫码加锁、波次三层兜底、拣选中断、质量区 7/3 天滞留 | 阶段 A 已延期；本规范不回改 |
| 本切片不做 | 内容物表、嵌套、打印、回收状态机 | ADR-0047 |
| P1 已关闭 | 容器质量锁、6 维上架骨架、库位向导 | 只消费接口，不改语义 |
| P2 外的 M-RP 旧故事 | US-RP-001 日均出库动态阈值公式；US-RP-004 拣选短拣补拣；US-RP-005 图表/邮件报表 | 不在 US-M3-012；短拣属波次三层延期 |
| P3 | AGV/PTL 真硬件、`iot_devices`/`wcs_tasks`/`iot_event_logs` 启用与拍灯 | ADR-0048 决策 9 Phase 3。基线⑥「Phase 2 建齐中台四表」本切片明确不做：空表不进补货切片，与硬件一并放到 Phase 3。PDA 确认不写 `wcs_tasks`、不拍灯 |
| M8 | 门店补货建议 `retail_replenishment_suggestions` | 连锁模块，不是仓内补货 |
| 新字段不建 | 库位 `assigned_product_id` 槽位商品；出库行等待补货列 | 无既有列可复用；等待标记由波次消费事件总线后写入波次自己的等待态（wave-model §6），本切片不改 `outbound_order_lines` |

### 2.3 与 M-RP 旧故事的关系

US-M3-012 取代仓内补货的实施口径：水位只读策略表 Min-Max，不跑 US-RP-001 动态公式。US-RP-002/003 的「日常/紧急」映射为本规范的 `min_max` / `wave_gap`。US-RP-005 的字段查询由任务大盘列表覆盖；图表导出与定时推送不做。

### 2.4 与源文档的已裁定分歧（实施以本规范为准）

| 源 | 原文 | 本规范裁定 |
|---|---|---|
| 模型 §5.1.2 | `urgent_qty = 缺口 − 在途量`，同时缺口已用「含在途」可用量 | **双重扣减，废弃**。可用量已含 `qty_replenish_in_transit`，`urgent_qty = 订单需求 − 拣选位该商品可用量` |
| 模型 §5.4 状态 | 无 `suspended` | US-M3-012#8 要求挂起：本阶段 **增加** `suspended` |
| 模型 §5.4 列 | 无波次外键、无 `claimed_at` | 本阶段增加 `wave_id` / `outbound_order_id` / `outbound_line_no` / `claimed_at` / `last_progress_at` / `return_reason` |
| 模型数量精度 | `NUMERIC(12,4)` | 与已落地库存列对齐，用 `NUMERIC(19,4)` |
| ADR-0048 基线⑥ | Phase 2 建齐 IoT 四表结构 | 按决策 9 里程碑推迟到 Phase 3，本切片明确不做 |
| ADR-0048 决策 7 状态机 | 未写 `suspended` | 同 US-M3-012#8，增加 `suspended` |
| US-M3-012#2「订单行标记等待补货」 | 像是补货事务内改出库行 | 补货事务只写 `event_bus_event`（`replenishment.waiting`）；波次模块消费后打等待态。本切片 GWT 验收停在事件已落库 |
| §4.2 字面扣减全部 `qty_allocated` | 波次算单后拣选面占用再减一次，`urgent_qty` 放大 | **明确不做字面二次扣减**。波次缺口可用量 = §4.2 + 加回本波次 `inventory_allocations.status=locked`（`pick_available_qty_excluding_wave`）。本波次冻结不算缺口洞；GWT 4 / `wave4_replenish_after_allocate_uses_pick_face_gap` 期望 `qty=7`。不回退。 |
| §6.2 `M3_REPLENISH_SOURCE_UNAVAILABLE` 422 | 冻结上升也返回 422 | **作业冻结挂起明确不走 422**。生成/库存命令仍 422。`pick`，以及 `confirm` 且 `picked_qty=0`：来源冻结 → 200 + `suspended` + H4（GWT 17）。`confirm` 且 `picked_qty>0`：只送达已下架量，不因冻结再挂起（GWT 31）。不把挂起路径改回 422。挂起谓词见 §5；**禁止**用字面 §4.2 可下架量 < 剩余任务量（生成后该式恒成立，会吃掉 GWT 29）。 |

## 3. 领域对象

| 对象 | 说明 | 已有/待建 |
|---|---|---|
| `ReplenishmentStrategy` | 货主级策略：scope、动线、Min-Max、触发模式 | 表已在 Phase 1 基线 |
| `ReplenishmentLocationGroup` / `Member` | 库位组及成员 | 表已在 Phase 1 基线 |
| `ReplenishmentTask` | 作业过程；不算量权威 | **本阶段新建表** |
| `InventoryBatch` 在途双字段 | `qty_replenish_in_transit` / `qty_replenish_out_transit` | 列已在 Phase 1 |
| `WarehouseLocation.replenish_strategy_id` | 拣选位挂策略，可空 | 列已在 Phase 1 |
| M-TE `TaskGroup` | 作业员库区绑定：`task_type_codes` 含 `replenish` 的班组 | 已有；本阶段只读消费 |
| `event_bus_event` | 等待/完成/取消通知波次 | 已有；本阶段写入，不新建 outbox 表 |

未挂策略的拣选位不参与 Min-Max 巡检；波次缺口仍可按命中策略或默认动线生成 `urgent` 任务。

策略表已有字段 `location_type` 与 `target_type`：保存时 **二者必须相等**（`location_type` 表示策略作用的拣选形态，`target_type` 表示动线终点）。`source_type`/`target_type` 合法组合见 §4.11。

## 4. 不变量

1. **货主隔离**：所有读写带 `owner_id`；策略、组、任务、批次均按货主过滤。
2. **可用量口径（唯一）**
   - 拣选位某商品可用量 = 该位该商品全部 `status=qualified` 行的 `Σ(qty_on_hand − qty_allocated − qty_frozen + qty_replenish_in_transit)`
   - 来源批次可下架量 = `qty_on_hand − qty_allocated − qty_frozen − qty_replenish_out_transit`
   `qty_frozen` 含容器质量锁联动冻结，与补货在途互不混用。
   波次缺口引擎在上式之上 **加回本波次** `inventory_allocations.status=locked`（见 §2.4、§10.2）。Min-Max 巡检不加回。
3. **Min-Max 触发**：目标拣选位该商品可用量 `<= min_safety_threshold` 才生成；任务量 = `min(max_replenish_target − 可用量, 来源可下架量)`，再按商品默认包装向下取整（§10.7）；余量不足 1 包装本轮不生成。
4. **波次缺口**：`urgent_qty = 订单需求 − 拣选位该商品可用量`；`urgent_qty <= 0` 则不生成。可用量已含在途，不再减第二次。
5. **在途权威**：任务生成同事务 +Δ 双字段；确认同事务 −Δ 并转在手；取消/20 分钟未领超时同事务回冲。禁止靠聚合任务表算可用量。
6. **状态机**：`pending → in_progress → done`；`cancelled` 仅从 `pending`/`in_progress`/`suspended` 且 `done_qty = 0` **且** `picked_qty = 0`。来源在生成后被质检冻结导致可下架量不足时进入 `suspended` 并 H4 告警，人工复核后改派或取消。`done_qty = qty` 才置 `done`。
7. **一人一任务**：领取写 `operator_id`，`version` 乐观锁；同任务同时仅一名作业员。同一作业员同时只能有一条 `in_progress` 补货任务。
8. **一任务一批次**：来源按 FEFO 选 `source_batch_id`；不足则按效期再开下一任务。此 FEFO **只用于补货来源选择**，不改变出库「ERP 指定批号」规则（US-M3 跨故事约束 6）。
9. **库存写路径**：在手/在途增减必须调用既有 `inventory` 上下文的三个补货命令（§10.4），写入 `inventory_movements.movement_type=replenish`。补货 repository 不得直接 `UPDATE inventory_batches.qty_*`。
10. **scope 优先级**：同目标位多策略命中时 `product > category > location_group`，只取一条。
11. **动线合法组合**：`storage→case_pick`、`storage→piece_pick`、`case_pick→piece_pick`。其他组合拒绝保存策略。domain 函数名 `validate_replenish_route`（术语表 #80 `replenish-route`）。
12. **质量与形态**：来源/目标必须匹配策略 `source_type`/`target_type`；目标必须是合格区（`quality_color` 空 / `qualified_green` / `qualified`，与 P1 `zone_treats_as_qualified` 同一函数）；来源批次不得处于容器质量锁联动的隔离/不合格状态（`status` 非 `qualified` 或容器 `current_lock_category` 为 `quarantine`/`rejected`）。
13. **策略挂接**：Min-Max 只处理 `replenish_strategy_id = 本策略.id` 的拣选位。库位组保存或「绑定库位」接口负责写入/清空该列；冲突（库位已挂其他策略）拒绝。
14. **PDA 必须在线**：US-M3 跨故事约束 7 的离线暂存只覆盖移库/盘点。补货 `picked_qty` 只用于在线断线重连后恢复进度，不提供离线队列，不在冲突时静默覆盖。

## 5. 状态机

```
pending ──claim──► in_progress ──confirm(done_qty=qty)──► done
   │                    │
   │                    ├── confirm(done_qty<qty) 仍 in_progress
   │                    ├── source_frozen ──► suspended ──reassign──► pending
   │                    │                         └── confirm(picked_qty>0) ──► done 或仍 suspended
   │                    ├── reassign ──► pending（清空 operator_id）
   │                    └── return ──► pending + return_reason
   └── cancel / urgent_timeout ──► cancelled
in_progress / suspended / pending ──cancel(picked_qty=0,done_qty=0)──► cancelled
```

- `claim`：仅 `pending` 且无 `operator_id`；写作业员、`claimed_at`、`last_progress_at`。普通任务按作业员 M-TE 班组库区（§10.6）；`urgent` 可跨区。同一作业员同时只能有一条 `in_progress` 补货任务。
- `pick`：仅 `in_progress`（`suspended` 禁止再下架）。下架前跑挂起谓词（见下）。扫描来源库位码或容器 LPN 必须与任务 `source_location_id`/`source_lpn_id` 一致；`picked_qty += Δ`；更新 `last_progress_at`；`picked_qty > qty` 拒绝。
- `confirm`：状态为 `in_progress`，或（`suspended` 且 `picked_qty>0`，只送达已下架量、不再下架）。挂起谓词：`available_for_task = §4.2 可下架量 + (qty − done_qty)`（加回本任务仍占的 `qty_replenish_out_transit`），`remaining_unpicked = qty − done_qty − picked_qty`；仅当 `available_for_task < remaining_unpicked` 才 200 挂起。`picked_qty=0` 时该式等价于 §4.2 可下架量 **< 0**（GWT 17）。**禁止**用「字面 §4.2 < 剩余任务量」。`confirm` **仅** `picked_qty=0` 时跑挂起谓词（优先于数量守卫）；`picked_qty>0` 不再因来源冻结拦截，只送达已下架量（GWT 31，冻结未解除也送达）。无冻结（`available_for_task >= remaining_unpicked`）且 `picked_qty=0` → `M3_REPLENISH_STATE_INVALID`（GWT 29）。无冻结时本次确认量必须 `> 0` 且 `<= picked_qty`。扫描目标位必须等于 `target_location_id`；账面转换后 `picked_qty −= Δ`、`done_qty += Δ`。`suspended` 确认后若 `done_qty=qty` 则 `done`，否则仍 `suspended`。
- `reassign`：**一律回 `pending` 并清空 `operator_id`、`claimed_at`**，记审计。`picked_qty > 0` 时仍回 `pending`，新作业员继续同一任务，不回冲在途、不丢 `picked_qty`。
- `return`：作业员退回 `pending` + `return_reason`。`source_mismatch` 时另写事件总线 `replenishment.source_mismatch` 与 H4，不改库存。`picked_qty > 0` 时禁止退回（与取消同一门槛，已下架实物不能随任务退回）。
- `cancel`：`done_qty = 0` 且 `picked_qty = 0`；已下架未送达禁止取消。
- `suspended`：确认或下架时来源 `qty_frozen` 上升导致上条挂起谓词成立 → 保持已下架实物不回冲，任务挂起并告警。`picked_qty=0` 时可取消或改派回 `pending`。`picked_qty>0` 禁止取消、禁止再 pick；允许 `reassign` 回池，或按上条 `confirm` 把已下架量送达。
- `urgent` 下发：本阶段不新建推送网关。生成当时只保证 PDA 列表 `priority=urgent` 置顶（不写 10 分钟告警）。PDA 若已有通用任务提醒通道，生成时复用该通道一次；没有则仅列表置顶。列表置顶**不打断**当前 `in_progress`。10 分钟未领才写 H4 `replenishment_urgent_unclaimed`；20 分钟仍 `pending` 自动取消并回冲，通知波次。
- 领取后 1 小时 `last_progress_at` 无变化：告警，允许强制改派，**不**自动取消。
- 整托：任务量 ≥ 来源容器在手量 × 配置键 `replenishment.full_lpn_ratio`（默认 0.8）则写 `source_lpn_id`；`done` 且该 LPN 在来源位 `qty_on_hand=0` 时容器改 `idle`。拆托任务 `source_lpn_id` 为空，完成不释放容器。

作业四步 = 领取 / 下架 / 送达 / 账面转换。幂等与完成联动不算作业步。

## 6. API / 权限 / 错误码 / 审计 / 幂等

权限：PC 策略与大盘 `m3.replenishment.manage`；PDA 领取/下架/确认/退回 `m3.replenishment.execute`。无权限 403。读大盘与策略列表：manage 或 execute。execute 读任务时只见可领任务与本人任务。

所有写接口必须带 `Idempotency-Key`，复用既有 `idempotency_request` 中间件。路径在质量矩阵 `US-M3-012.api_paths` 之上补齐作业与策略写接口（实施时同步矩阵，见 §15）。

| 方法 | 路径 | 权限 | 幂等 | 说明 |
|---|---|---|---|---|
| GET | `/api/v1/replenishment/strategies` | manage | 否 | 查询 `enabled`/`scope_type`/`keyword` |
| GET | `/api/v1/replenishment/strategies/{id}` | manage | 否 | 详情 |
| POST | `/api/v1/replenishment/strategies` | manage | 是 | 新建 |
| PUT | `/api/v1/replenishment/strategies/{id}` | manage | 是 | 更新；禁止改 `owner_id` |
| POST | `/api/v1/replenishment/strategies/{id}/disable` | manage | 是 | 停用；无物理删除 |
| PUT | `/api/v1/replenishment/strategies/{id}/locations` | manage | 是 | 替换挂接拣选位 |
| GET | `/api/v1/replenishment/strategies/{id}/preview` | manage | 否 | 命中拣选位与当前可用量 |
| GET | `/api/v1/replenishment/location-groups` | manage | 否 | 库位组列表 |
| GET | `/api/v1/replenishment/location-groups/{id}` | manage | 否 | 组 + 成员 |
| POST | `/api/v1/replenishment/location-groups` | manage | 是 | 组 + 成员全量替换 |
| PUT | `/api/v1/replenishment/location-groups/{id}` | manage | 是 | 组 + 成员全量替换 |
| POST | `/api/v1/replenishment/location-groups/{id}/disable` | manage | 是 | 停用组；成员库位若只挂本策略则 `replenish_strategy_id` 置空 |
| GET | `/api/v1/replenishment/tasks` | manage 或 execute | 否 | 大盘/PDA 列表 |
| GET | `/api/v1/replenishment/tasks/{id}` | manage 或 execute | 否 | 详情；execute 仅本人或可领 |
| POST | `/api/v1/replenishment/tasks` | manage | 是 | 手动发起 `trigger_mode=manual` |
| POST | `/api/v1/replenishment/tasks/{id}/claim` | execute | 是 | 领取 |
| POST | `/api/v1/replenishment/tasks/{id}/pick` | execute | 是 | 来源下架登记 |
| POST | `/api/v1/replenishment/tasks/{id}/confirm` | execute | 是 | 送达确认 + 账面转换 |
| POST | `/api/v1/replenishment/tasks/{id}/cancel` | manage | 是 | 取消回冲 |
| POST | `/api/v1/replenishment/tasks/{id}/reassign` | manage | 是 | 改派 |
| POST | `/api/v1/replenishment/tasks/{id}/return` | execute | 是 | 退回 pending |

引擎入口不是外部 CRUD：巡检由调度调用 `ReplenishmentService::run_min_max_patrol`；超时由调度调用 `ReplenishmentService::run_timeout_scan`；波次算单在缺口处调用 `ReplenishmentService::create_wave_gap_tasks`（同步生成任务，波次重算单仍异步）。订单行「等待补货」**只**通过 `event_bus_event.event_type=replenishment.waiting` 通知波次模块。

### 6.1 请求 / 响应字段

**Strategy upsert**（POST/PUT）

| 字段 | 必填 | 约束 |
|---|---|---|
| `strategy_code` | 是 | 货主内唯一 |
| `strategy_name` | 是 | 非空 |
| `scope_type` | 是 | `location_group` / `category` / `product` |
| `scope_ref` | 是 | `product`→本货主 `products.id`；`location_group`→本货主组 id；`category`→`system_dictionary_items.id` 且 `dict_code='special_drug_category'` |
| `source_type` | 是 | `storage` / `case_pick` |
| `target_type` | 是 | `case_pick` / `piece_pick`；写入时 `location_type = target_type` |
| `min_safety_threshold` | 是 | `>= 0` |
| `max_replenish_target` | 是 | `> min_safety_threshold` |
| `trigger_modes` | 是 | 非空子集，元素 ∈ `{min_max, wave_gap}` |
| `enabled` | 否 | 默认 true |

有未完成任务（`status ∈ {pending,in_progress,suspended}`）时禁止改 `scope_type`/`scope_ref`/`source_type`/`target_type`；只允许改名称、水位、`trigger_modes`、`enabled`。

**Bind locations** `PUT .../locations`：`{ "location_ids": [uuid] }` 全量替换。每个库位必须 `location_type = target_type` 且本货主；已挂其他策略 → `M3_REPLENISH_LOCATION_BOUND`。被移出的库位 `replenish_strategy_id` 置空。

**Location group upsert**：`group_code`/`group_name`/`enabled`/`location_ids`。成员必须本货主拣选位。组绑定到 `scope_type=location_group` 的策略时，保存成员同步重写这些成员的 `replenish_strategy_id`。

**Preview 响应**：`{ data: [{ location_id, location_code, product_id, available_qty, min_safety_threshold, max_replenish_target, would_trigger }] }`

**Manual task** POST `/tasks`：`source_location_id`、`source_batch_id`、`target_location_id`、`qty`、可选 `source_lpn_id`。服务端跑与引擎相同的形态/6 维/可下架量/包装取整校验；`qty` 必须已是包装整数否则 422 `M3_REPLENISH_STRATEGY_INVALID`。同事务 `reserve_replenish_in_tx`。`trigger_mode=manual`、`priority=normal`、`strategy_id` 可空、`created_by` = 当前用户名。

**Task 列表查询**：`status`、`trigger_mode`、`priority`、`source_location_id`、`target_location_id`、`operator_id`、`wave_id`、`keyword`（任务号）、`created_from`/`created_to`、分页。PDA 排序：`priority=urgent` 置顶 → 目标位 `pick_sequence_no` 升序（空值最后）→ `task_no`。

**claim / reassign**：`{ "version": n }`。reassign 不指定新作业员（回池待领）。

**pick**：`{ "version": n, "scanned_location_code": string, "scanned_lpn_code"?: string, "qty": decimal }`。有 `source_lpn_id` 时 `scanned_lpn_code` 必填且必须匹配。

**confirm**：`{ "version": n, "scanned_location_code": string, "qty": decimal }`

**cancel**：`{ "version": n, "reason": string }`，`reason` 非空。

**return**：`{ "version": n, "return_reason": "source_mismatch" \| "target_blocked" \| "other", "note"?: string }`

**Task 响应**含表全部列 + `version`。写成功后 `version += 1`。version 不匹配 → `M3_REPLENISH_CLAIM_CONFLICT`（领取）或 `M3_REPLENISH_STATE_INVALID`（其余）。

### 6.2 错误码

实施时写入 `docs/error-codes.md` 并过 `check_error_codes`，本规范冻结码表：

| 码 | HTTP | 含义 |
|---|---|---|
| `M3_REPLENISH_PERMISSION_DENIED` | 403 | 权限不足 |
| `M3_REPLENISH_STRATEGY_INVALID` | 422 | 动线/scope/Min-Max 非法（含 min≥max、动线组合非法、`location_type≠target_type`） |
| `M3_REPLENISH_SCOPE_NOT_FOUND` | 404 | scope_ref 不存在或不属于本货主 |
| `M3_REPLENISH_LOCATION_BOUND` | 409 | 库位已挂其他策略 |
| `M3_REPLENISH_TASK_NOT_FOUND` | 404 | 任务不存在或跨货主 |
| `M3_REPLENISH_STATE_INVALID` | 422 | 状态不允许该动作或 version 不符（非领取） |
| `M3_REPLENISH_CLAIM_CONFLICT` | 409 | 已被他人领取、本人已有 in_progress、或 version 冲突 |
| `M3_REPLENISH_ZONE_DENIED` | 422 | 普通任务目标库区不在作业员班组 |
| `M3_REPLENISH_QTY_EXCEEDED` | 422 | 下架/确认超量 |
| `M3_REPLENISH_SOURCE_MISMATCH` | 422 | 扫描来源库位/LPN 与任务不符 |
| `M3_REPLENISH_TARGET_MISMATCH` | 422 | 扫描目标位与任务不符 |
| `M3_REPLENISH_SOURCE_UNAVAILABLE` | 422 | 生成/库存命令：来源可下架量不足。作业中冻结上升 **不** 用本码（200 + `suspended`，GWT 17，§2.4） |
| `M3_REPLENISH_CANCEL_BLOCKED` | 422 | `done_qty>0` 或 `picked_qty>0` |
| `M3_REPLENISH_RETURN_BLOCKED` | 422 | `picked_qty>0` 不可退回 |
| `M3_REPLENISH_IDEMPOTENCY_CONFLICT` | 409 | 同键不同体 |
| `M3_REPLENISH_IDEMPOTENCY_REQUIRED` | 400 | 缺幂等键 |
| `M3_REPLENISH_PUTAWAY_BLOCKED` | 422 | 送达 6 维②③⑥ 或目标作业锁阻断 |
| `M3_REPLENISH_NUMBERING_UNAVAILABLE` | 422 | M-CG 无 `replenishment_task` 规则 |

### 6.3 审计、编号、事件

审计（H2，只 INSERT）：策略 CRUD/停用/挂接、组维护、任务生成/领取/下架/确认/取消/改派/退回/超时取消。`module=M3`，`resource_type` 为 `replenishment_strategy` / `replenishment_location_group` / `replenishment_task`。库存流水 `movement_type=replenish`，`approval_source=SYSTEM`（引擎/超时）或 `MANUAL`（大盘手工），`source_document_type=replenishment_task`，`source_document_id=task.id`。

编号：任务号走 M-CG，`document_type=replenishment_task`，实施时在系统字典 `document_type` 增加该项。

**事件总线**（复用 `event_bus_event`，`source_module=M3`，`resource_type=replenishment_task`，`resource_id=task.id`，`idempotency_key="{event_type}:{task_id}"`）：

| event_type | 何时 | payload |
|---|---|---|
| `replenishment.waiting` | 波次缺口任务插入成功 | `wave_id, outbound_order_id, outbound_line_no, task_id, qty` |
| `replenishment.done` | 任务置 `done` | 同上 + `done_qty` |
| `replenishment.cancelled` | 取消或 20 分钟超时 | 同上 + `reason` |
| `replenishment.source_mismatch` | 退回原因为 `source_mismatch` | `task_id, source_location_id, source_batch_id, operator_id` |
| `replenishment.patrol_fail` | 巡检/缺口生成跳过（6 维/无源/锁） | `target_location_id, product_id, reason_code, strategy_id` |

波次模块订阅前三个类型。本切片不实现波次消费与出库行等待列。

**H4 告警 event_type**（复用既有 H4 定义登记，实施时写入告警定义种子）：

| event_type | 触发 |
|---|---|
| `replenishment_patrol_fail_repeat` | 同目标位同因连续 3 次生成失败 |
| `replenishment_urgent_unclaimed` | urgent 满 10 分钟仍 pending |
| `replenishment_urgent_timeout` | urgent 满 20 分钟自动取消 |
| `replenishment_no_progress` | 领取后 1 小时 `last_progress_at` 未变 |
| `replenishment_source_frozen` | 进入 `suspended` |
| `replenishment_source_mismatch` | 退回 `source_mismatch` |

## 7. 表与迁移要点（v1 前直接改基线）

已存在、本阶段只使用不改语义：`replenishment_strategies`、`replenishment_location_groups`、`replenishment_location_group_members`、`warehouse_locations.replenish_strategy_id`、`inventory_batches.qty_replenish_*`、`event_bus_event`、`product_packaging_levels`、M-TE `task_groups`（`task_type_codes` 已含 `replenish`）。

本阶段新增（对齐模型 §5.4 + §2.4 增补，精度与库存列一致用 `NUMERIC(19,4)`）：

```sql
CREATE TABLE replenishment_tasks (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL,
    task_no VARCHAR(64) NOT NULL,
    trigger_mode VARCHAR(16) NOT NULL,
    priority VARCHAR(16) NOT NULL DEFAULT 'normal',
    strategy_id UUID REFERENCES replenishment_strategies(id),
    source_location_id UUID NOT NULL,
    source_batch_id UUID NOT NULL,
    source_lpn_id UUID,
    target_location_id UUID NOT NULL,
    product_id UUID NOT NULL,
    batch_no VARCHAR(64) NOT NULL,
    qty NUMERIC(19, 4) NOT NULL CHECK (qty > 0),
    picked_qty NUMERIC(19, 4) NOT NULL DEFAULT 0,
    done_qty NUMERIC(19, 4) NOT NULL DEFAULT 0,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    operator_id UUID,
    wave_id UUID,
    outbound_order_id UUID,
    outbound_line_no INT,
    claimed_at TIMESTAMPTZ,
    last_progress_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    cancel_reason TEXT,
    return_reason VARCHAR(32),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, task_no),
    CHECK (trigger_mode IN ('min_max', 'wave_gap', 'manual')),
    CHECK (priority IN ('normal', 'urgent')),
    CHECK (status IN ('pending', 'in_progress', 'suspended', 'done', 'cancelled')),
    CHECK (picked_qty >= 0 AND done_qty >= 0 AND picked_qty + done_qty <= qty),
    CHECK (return_reason IS NULL OR return_reason IN ('source_mismatch', 'target_blocked', 'other')),
    CHECK (outbound_line_no IS NULL OR outbound_line_no > 0)
);

CREATE INDEX replenishment_tasks_owner_status_target_idx
    ON replenishment_tasks (owner_id, status, target_location_id);
CREATE INDEX replenishment_tasks_owner_source_batch_idx
    ON replenishment_tasks (owner_id, source_batch_id);
CREATE INDEX replenishment_tasks_owner_priority_created_idx
    ON replenishment_tasks (owner_id, priority, created_at);
```

策略表补约束（基线无水位 CHECK，本阶段加）：

- `CHECK (min_safety_threshold >= 0 AND max_replenish_target > min_safety_threshold)`
- 动线三组合由 domain `validate_replenish_route(source, target)` 强制；保存时写 `location_type = target_type`

GRANT 给 `wms_app`。不做兼容双写、不做旧表回填。不建 IoT 四表。

权限种子：插入 `m3.replenishment.manage`、`m3.replenishment.execute`；`warehouse_manager` 得 manage，`custodian` 得 execute（与既有 M3 权限触发器同一模式）。

## 8. 页面与分层

| 页面 | 族 | 入口 | 查询 |
|---|---|---|---|
| 补货策略配置 | 配置型 | 独立菜单 `m3-replenishment-strategies`，权限 manage | 核心：keyword、enabled；更多：scope_type、target_type |
| 补货任务大盘 | 列表型 ListPageTemplate + QueryPanel + DataGrid | 独立菜单 `m3-replenishment-tasks`，权限 manage | 核心：status、trigger_mode；更多：priority、location、operator、owner（超管货主切换） |
| PDA 补货作业 | 作业流（非 PC 页面族） | PDA 任务列表，权限 execute | urgent 置顶 → `target.pick_sequence_no` → `task_no`；推送失败仍可拉取 |

菜单挂「库内业务 / 库存管理」，与既有 M3 页同组。登记 `page-query-core-fields.json`。

**页面设计契约**

- 策略页（配置型）：分区 = 基本信息 / 动线与 scope / Min-Max 与触发模式 / 挂接拣选位 / 启停 / 命中预览。标准动作：保存、启停。私有动作：预览命中位、绑定库位、维护库位组。详情不常驻；审计不进本页（走 H2 审计列表）。
- 大盘（列表型）：主载体 DataGrid。列固定为任务号、触发模式、优先级、来源位、目标位、数量/`done_qty`、状态、作业员、创建/更新时间。标准动作：刷新。私有动作：手动发起、重派、取消，一律走 Dialog，禁止在列表区常驻动作表单或当前处理对象。超时告警行高亮（urgent 超时或 1 小时无进度）。
- PDA：四步向导，触控目标 ≥ 48pt、字号 ≥ 16pt；扫码框与数量输入；当前 `in_progress` 时列表不自动改派到新 urgent。

后端：`replenishment_handlers` → `ReplenishmentService` → domain 不变量 + `ReplenishmentRepository` + **库存领域服务三个命令**。
前端：page → `features/replenishment` hooks → `@wms/api-client`。禁止页面裸 fetch。

调度：复用 M-TE 巡检骨架注册两个作业，补货任务表不写入 M-TE 单据表：

| 作业名 | 间隔 | 调用 |
|---|---|---|
| `replenishment_min_max` | 按 §10.1 窗口计算下一次（白天 60 / 夜间 15） | `run_min_max_patrol` |
| `replenishment_timeout` | 每 1 分钟 | `run_timeout_scan`（10 分钟告警 / 20 分钟取消 / 1 小时无进度告警） |

## 9. 与 P1 的接口约定

| P1 能力 | 本阶段如何消费 | 禁止 |
|---|---|---|
| 容器质量锁 | 来源批次 `status` 非 `qualified` 或容器 `current_lock_category∈{quarantine,rejected}` 不得选为来源；`qty_frozen` 进入可下架量公式 | 不改加锁/解锁语义，不在补货里写锁事件 |
| 6 维 | **生成**时对目标位跑 ②温区 ③质量 ⑥外用串味+容量；失败跳过该位并写 `replenishment.patrol_fail`。**送达确认**时再跑 ②③⑥，失败 `M3_REPLENISH_PUTAWAY_BLOCKED` | 不重做 ①品类大区以外的上架全流水；④特药双人在补货送达不强制（补货不是上架事务） |
| 库位形态 | 来源 `location_type` 必须等于策略 `source_type`；目标必须等于 `target_type`；`storage` 来源可带 `source_lpn_id`；`case_pick` 来源 `source_lpn_id` 必空 | 不把补货目标写成 `storage`/`staging` |
| 路径序 | PDA/大盘作业排序用目标位 `pick_sequence_no` | 不改向导生成规则 |
| 库位锁与 M3 作业锁 | 目标（上架）跳过/确认阻断 `lock_in`/`lock_all`；来源不可下架 `lock_out`/`lock_all`。对齐 P1：`lock_in` 禁入、`lock_out` 禁出。目标处于盘点锁或养护作业锁时生成跳过、确认 `M3_REPLENISH_PUTAWAY_BLOCKED`（复用既有 M3 锁查询，不新造锁） | 不新造补货专用库位锁、不改 P1 锁方向 |
| 在途字段 | 只按第 4 节公式读写已有列 | 不把 `qty_frozen` 当补货占用 |

完成联动（任务 `done` 后异步，不在确认事务内调波次模块）：

1. 同事务写入 `event_bus_event` `replenishment.done`（payload 含 `wave_id`/`outbound_order_id`/`line_no`）。波次消费后清除「等待补货」并重算单。
2. 不在任务内递归生成下一条 Min-Max 任务；水位重判交给下一巡检周期。

`urgent` 取消：波次消费 `replenishment.cancelled` 后将该行降级到下一波次重算单，本阶段不在补货模块内拆单。

## 10. 引擎算法（可测）

### 10.1 Min-Max 命中集合与巡检

对每个货主、每个 `enabled` 且 `trigger_modes` 含 `min_max` 的策略：

**命中库位** = 同时满足：

1. `owner_id` 匹配，`status ≠ disabled`，`location_type = strategy.target_type`
2. `replenish_strategy_id = strategy.id`

**命中商品**（每位分别）：

- `scope_type=product`：`scope_ref` = `products.id`（本货主）；该位即使无库存行也按可用量 0 参与（空位补货）。
- `scope_type=category`：`scope_ref` = `system_dictionary_items.id`，且该项 `dict_code='special_drug_category'`（商品侧现有唯一分类锚点，不新增品类表/列）。命中 = 该位已有库存行、且 `products.special_drug_category` = 该项 `item_code` 的每个 `product_id`；空位跳过。`scope_ref` 不是该字典项 → `M3_REPLENISH_SCOPE_NOT_FOUND`。
- `scope_type=location_group`：`scope_ref` = `replenishment_location_groups.id`（本货主）。该位已有库存行的每个 `product_id` 分别计算；空位跳过。

同库位若被多条策略命中，按 §4.10 只保留优先级最高的一条，其余跳过。

对每个 (库位, 商品)：

1. 算可用量；若 `> min_safety_threshold` 则跳过。
2. 目标量 = `max_replenish_target − 可用量`。
3. FEFO 扫描动线内来源批次（`location_type = source_type`，同货主同商品，`status=qualified`，容器未隔离/不合格），由 repository 对来源行 `SELECT ... FOR UPDATE`，跳过库位锁/可下架量≤0。
4. 任务量 = 包装取整 `min(目标量剩余, 该来源可下架量)`。
5. 对目标跑 6 维②③⑥；失败写 `event_bus_event` `replenishment.patrol_fail`（payload：`target_location_id, product_id, reason_code, strategy_id`；`idempotency_key=patrol_fail:{strategy_id}:{location_id}:{product_id}:{reason_code}:{patrol_run_id}`）。不新建巡检日志表。同目标位+商品+原因码连续 3 条 fail 且中间无成功生成 → H4 `replenishment_patrol_fail_repeat`。
6. 同事务：插 `replenishment_tasks`（`trigger_mode=min_max`,`priority=normal`,`created_by=system:min_max`）+ `reserve_replenish_in_tx` + 审计。目标尚无库存行则命令内按来源批次属性 **插入** `qty_on_hand=0` 的目标行再 +`in_transit`。

白天窗口默认 08:00–20:00 每 60 分钟，夜间每 15 分钟；配置键 `replenishment.day_interval_minutes=60`、`replenishment.night_interval_minutes=15`、`replenishment.day_start=08:00`、`replenishment.day_end=20:00`，不引入新配置中心域。整托比例 `replenishment.full_lpn_ratio=0.8`。

### 10.2 波次缺口

波次算单调用 `create_wave_gap_tasks`，入参：`owner_id, wave_id, outbound_order_id, outbound_line_no, product_id, demand_qty, target_location_id`。

1. 校验目标位本货主且 `location_type ∈ {case_pick, piece_pick}`。
2. 按 scope 优先级找 `enabled` 且 `trigger_modes` 含 `wave_gap` 的策略；找不到则默认 `source_type=storage`、`target_type=该位 location_type`，`strategy_id` 空。
3. `urgent_qty = demand_qty − 该位该商品可用量`；可用量 = §4.2 + 加回本波次 locked 占用（本波次冻结不算缺口洞，§2.4）。`<= 0` 则不生成、不写事件。
4. FEFO 生成一条或多条 `trigger_mode=wave_gap`,`priority=urgent` 任务（规则同 §10.1 步 3–6）。每条按包装取整；最后不足 1 包装的余量丢弃，本轮不再生成（与 Min-Max 同一取整）。`created_by=system:wave:{wave_id}`。
5. 同事务写 `event_bus_event` `replenishment.waiting`（每条任务一条，幂等键含 `task_id`）。方法返回本次插入的任务列表，可空；调用方不得假定一定生成。

### 10.3 超时扫描

`run_timeout_scan` 每分钟：

1. `priority=urgent AND status=pending AND now-created_at >= 10min` 且尚未告警过 → H4 `replenishment_urgent_unclaimed`（同一任务只告一次，用事件总线幂等键 `replenishment.urgent_unclaimed:{task_id}`）。
2. `priority=urgent AND status=pending AND now-created_at >= 20min` → 走取消回冲 + `replenishment.cancelled` + H4 `replenishment_urgent_timeout`。
3. `status=in_progress AND last_progress_at` 距今 ≥ 1 小时 → H4 `replenishment_no_progress`（同一 `last_progress_at` 只告一次）。

### 10.4 库存命令（扩既有 inventory 上下文，不是新 Service）

| 命令 | 同事务效果 | 调用点 |
|---|---|---|
| `reserve_replenish_in_tx` | 来源行 `out_transit += Δ`（`FOR UPDATE` 且可下架量 ≥ Δ）；目标行 `in_transit += Δ`（无则插入）；不写 `inventory_movements` | 生成 |
| `confirm_replenish_in_tx` | 来源 `on_hand −= Δ` 且 `out_transit −= Δ`；目标 `on_hand += Δ` 且 `in_transit −= Δ`；插两条 `inventory_movements`（来源负、目标正，`movement_type=replenish`） | 确认 |
| `release_replenish_in_tx` | 来源 `out_transit −= 剩余`；目标 `in_transit −= 剩余`；剩余 = `qty − done_qty`；不改 `on_hand` | 取消 / 20 分钟超时 |

三条命令都带 `owner_id` + 行 version/数量前置条件，失败返回不足，由 service 映射为 `M3_REPLENISH_SOURCE_UNAVAILABLE`。禁止补货 repository 直接改 `qty_*`。

### 10.5 来源选择与整托

FEFO：`ORDER BY expiry_date ASC NULLS LAST, id`。一任务一批次。整托判定用该来源行 `container_lpn` 对应容器在该位的 `qty_on_hand`。`case_pick` 来源强制 `source_lpn_id` 空。

### 10.6 作业员库区

复用 M-TE `task_groups`：`enabled` 且 `task_type_codes` 含 `replenish` 且 `member_user_ids` 含当前用户。普通任务可领当且仅当目标位 `zone_id` ∈ 这些班组的 `zone_ids` 并集；班组 `zone_ids` 为空视为该仓库全部库区。urgent 跳过库区限制，仍受货主与「一人一 in_progress」限制。不新建作业员-库区表。

### 10.7 包装取整

商品 `product_packaging_levels` 中 `is_default=true` 的 `ratio_to_base` 为包装粒度；无默认层则按 1（基础单位）。`task_qty = floor(raw / ratio) * ratio`，结果用基础单位写入任务与库存。不足 1 包装 → 本轮不生成。

### 10.8 目标库存行

匹配键：`(owner_id, product_id, batch_no, location_id)` 且散货 `container_lpn` 空。生成时无行则插入：从来源行复制 `batch_no` / 效期 / 生产日期 / 商品，`qty_on_hand=0`，`status=qualified`。确认只增减数量，不改效期。

## 11. Given / When / Then

1. **策略动线非法**
   Given 货主已登录且有 `manage`
   When POST 策略 `source_type=piece_pick, target_type=storage`
   Then 422 `M3_REPLENISH_STRATEGY_INVALID`，无行插入。

2. **Min-Max 触发并写在途**
   Given 拣选位该商品可用量 2、`min=5`、`max=20`，来源可下架 30，默认包装 1，库位已挂该策略
   When 巡检跑过该位
   Then 生成 1 条 `pending` 任务 `qty=18`；目标 `qty_replenish_in_transit +=18`；来源 `qty_replenish_out_transit +=18`；`qty_on_hand` 不变。

3. **在途覆盖后不再生成**
   Given 上例任务仍 `pending`（在途已 +18，可用量 20）
   When 再次巡检
   Then 不生成第二任务。

4. **波次缺口 urgent**
   Given 订单需求 10、拣选位可用量 3（含在途）
   When 波次算单调用缺口引擎
   Then 生成 `wave_gap/urgent` 任务 `qty=7`，并插入 `event_bus_event` `replenishment.waiting`（本切片不改出库行列）。

5. **领取冲突**
   Given 任务 `pending`
   When 两作业员同时 claim 同一 `version`
   Then 一人 200 且 `in_progress`，另一人 409 `M3_REPLENISH_CLAIM_CONFLICT`。

6. **超量下架**
   Given 任务 `qty=10`、`picked_qty=8`
   When pick `qty=3`
   Then 422 `M3_REPLENISH_QTY_EXCEEDED`。

7. **确认账面转换**
   Given `picked_qty=10`、`qty=10`
   When confirm 10
   Then 来源 `qty_on_hand−10` 且 `out_transit−10`；目标 `qty_on_hand+10` 且 `in_transit−10`；任务 `done`；流水 `replenish`；发 `replenishment.done`。

8. **部分确认**
   Given 任务 `qty=10`、`picked_qty=4`
   When confirm 4
   Then `done_qty=4`、`status=in_progress`，剩余 6 仍由原任务承担。

9. **已下架不可取消**
   Given `picked_qty=4`、`done_qty=0`
   When cancel
   Then 422 `M3_REPLENISH_CANCEL_BLOCKED`，双字段不回冲。

10. **无下架可取消**
    Given `picked_qty=0`、`done_qty=0`
    When cancel
    Then `cancelled`，双字段 −Δ 回冲。

11. **容器质量锁来源跳过**
    Given FEFO 最近效期批次所在容器为 `quarantine`
    When 巡检选源
    Then 不选该批，改选下一合格批次；无合格源则不生成并写 `replenishment.patrol_fail`。

12. **送达 6 维③ 阻断**
    Given 目标区位 `quality_color=quarantine_yellow`
    When confirm
    Then 422 `M3_REPLENISH_PUTAWAY_BLOCKED`，在手不变。

13. **幂等重放确认**
    Given 已成功 confirm 且同一 Idempotency-Key
    When 再 POST confirm
    Then 200 回放原结果，`done_qty` 不加倍。

14. **urgent 20 分钟超时**
    Given `wave_gap` 任务创建 20 分钟仍 `pending`
    When 超时作业运行
    Then 任务 `cancelled`，双字段回冲，事件总线有 `replenishment.cancelled`。

15. **无权限**
    Given 用户无 `execute`
    When claim
    Then 403。

16. **整托完成释箱**
    Given 来源容器在手 10、任务 `qty=10`、`source_lpn_id` 有值
    When confirm 10 且该 LPN 来源位在手变为 0
    Then 容器 `status=idle`。

17. **来源冻结挂起**
    Given 任务 `in_progress`、`picked_qty=0`、`done_qty=0`，确认前该批次 `qty_frozen` 升至使 §4.2 可下架量 **< 0**（本任务 `out_transit` 已计入；不是「可下架量 < 剩余任务量」）
    When confirm
    Then **HTTP 200**，任务 `suspended`，H4 告警，在手不改。不返回 422 `M3_REPLENISH_SOURCE_UNAVAILABLE`（§2.4）。

18. **扫码不匹配**
    Given 任务来源位 A、作业员已 claim
    When pick 扫描库位 B
    Then 422 `M3_REPLENISH_SOURCE_MISMATCH`，`picked_qty` 不变。

19. **包装取整不足不生成**
    Given 目标量剩余 5，默认包装 `ratio_to_base=12`
    When 巡检
    Then 不生成任务。

20. **普通任务跨区拒绝**
    Given 作业员班组 `zone_ids` 不含目标库区，任务 `priority=normal`
    When claim
    Then 422 `M3_REPLENISH_ZONE_DENIED`。

21. **urgent 可跨区**
    Given 同上班组，任务 `priority=urgent`
    When claim
    Then 200，`in_progress`。

22. **缺幂等键**
    Given 已登录 manage
    When POST 策略且无 `Idempotency-Key`
    Then 400 `M3_REPLENISH_IDEMPOTENCY_REQUIRED`。

23. **库位已挂其他策略**
    Given 库位 `replenish_strategy_id` 指向策略 A
    When PUT 策略 B 的 locations 包含该库位
    Then 409 `M3_REPLENISH_LOCATION_BOUND`。

24. **改派回池**
    Given 任务 `in_progress`、`picked_qty=2`
    When reassign
    Then `pending`，`operator_id` 空，`picked_qty` 仍为 2，在途不回冲。

25. **已下架不可退回**
    Given `picked_qty=2`
    When return
    Then 422 `M3_REPLENISH_RETURN_BLOCKED`。

26. **product scope 空位仍生成**
    Given 拣选位已挂 product 策略、该商品无库存行、来源充足
    When 巡检
    Then 按可用量 0 生成任务，并插入目标库存行 `on_hand=0`、`in_transit=任务量`。

27. **无策略波次缺口用默认动线**
    Given 目标为零拣位、该商品无任何 enabled+wave_gap 策略
    When 缺口引擎 demand=5、可用量=0
    Then 生成 `wave_gap` 任务，`strategy_id` 空，来源形态 `storage`。

28. **1 小时无进度不自动取消**
    Given `in_progress` 且 `last_progress_at` 已过 1 小时
    When 超时作业运行
    Then 任务仍 `in_progress`，写出 H4 `replenishment_no_progress`。

29. **未下架不可确认**
    Given 任务 `in_progress`、`picked_qty=0`，且 §4.2 可下架量 `>= 0`（挂起谓词不成立）
    When confirm 任意正数
    Then 422 `M3_REPLENISH_STATE_INVALID`，在手与在途不变。

30. **category scope_ref 必须是特殊药品分类字典项**
    Given 已登录 manage
    When POST 策略 `scope_type=category` 且 `scope_ref` 不是 `special_drug_category` 字典项
    Then 404 `M3_REPLENISH_SCOPE_NOT_FOUND`。

31. **挂起后可送达已下架量**
    Given 任务 `suspended`、`picked_qty=4`、`qty=10`，来源 `qty_frozen` 仍使 §4.2 可下架量 < 0
    When confirm 4 且目标位 6 维通过
    Then `done_qty=4`、`picked_qty=0`、`status=suspended`；禁止再 pick。不因冻结再次挂起或 422。

## 12. 测试层级（本阶段必做）

| 层级 | 本阶段 | 覆盖 |
|---|---|---|
| L1 | **必做** | `validate_replenish_route`、可用量公式、任务量取整、状态迁移、scope 优先级、容器质量锁跳过、命中集合 |
| L2 | **必做** | OpenAPI 路径/请求体与本节 API 表一致；`just openapi-sync` |
| L3 | **必做** | GWT 2、4、7、8、26 的 postgres 流程 |
| L4 | **必做** | GWT 1、6、9、11、12、15、17、18、19、20、22、23、25、29、30、31 |
| L5 | **必做** | 生成/确认/取消后双字段+在手+流水+审计同行 |
| L6 | **必做** | GWT 5 并领取；两任务抢同一来源批次 |
| L7 | 本阶段不做 | 留给容量基线，不阻塞 P2 功能冻结 |
| L8 | **必做** | manage/execute 矩阵；跨货主 404 |
| L9 | 本阶段不做兼容层 | 首版前直接改契约，无双读 |
| L10 | **必做** | 巡检连续 3 次失败、urgent 超时、确认完成写 tracing/告警/事件总线 |
| L11 | **必做** | 生成/claim/pick/confirm/cancel 同键重放；异体 409 |

写操作最低集：L1+L2+L3+L4+L5+L8+L11；并发资源加 L6；跨模块事件加 L10。

## 13. 实施边界

| 层 | 新建/改 | 不可做 |
|---|---|---|
| domain | `validate_replenish_route`、`available_qty`、`task_qty`、状态守卫、命中优先级 | IO、`FOR UPDATE`、时间源直读 |
| inventory 上下文 | `reserve_replenish_in_tx` / `confirm_replenish_in_tx` / `release_replenish_in_tx` | 新开库存 Service |
| service | 巡检、缺口、领取确认编排、调库存命令、写事件总线 | 散落 SQL、HTTP、决定水位公式 |
| repository | 策略/组/任务存取、来源行锁 | 决定是否补货、直接改 `qty_*`、写事件总线 |
| handler | 鉴权、幂等头、错误映射 | 算量、状态机 |
| web-admin | 策略页、大盘页、feature hooks | 裸 fetch |
| pda-mobile | 补货作业四步 | 生产启动绕过 ADR-0027 门禁；本阶段按现有 PDA 工程约束落地 |
| 迁移 | `replenishment_tasks`、策略水位 CHECK、权限种子、字典 `replenishment_task` | 改 P1 锁/6 维语义；建 IoT 四表；改出库行列 |

## 14. US-M3-012 验收对照

| US-M3-012 | 本规范落点 |
|---|---|
| #1 策略/组/预览/权限 | §3、§6、§8、§10.1 挂接 |
| #2 双触发 | §10.1、§10.2；等待标记 = 事件总线 |
| #3 生成+双字段+FEFO | §4.5、§4.8、§10 |
| #4 PDA 领取/下架/送达/转换 | §5、§6 pick/confirm |
| #5 部分执行与整托释箱 | §5、GWT 8、GWT 16 |
| #6 取消 | §5 cancel、GWT 9–10 |
| #7 大盘列与操作 | §8 列清单与 Dialog |
| #8 幂等+冻结挂起 | §6 L11、`suspended`、GWT 13/17 |
| #9 推送/不打断/超时/改派/双事件 | §5、§9、§10.3 |
| #10 动线与路径序 | §4.11、§8 PDA 排序 |

## 15. 实施时必须同步的治理件

阶段 D 第一批实现票必须改、不得留到「以后」：

1. `docs/error-codes.md` 登记 §6.2 全部码，过 `check_error_codes`。
2. `governance/quality-matrix.toml` 的 `US-M3-012.api_paths` 扩到 §6 全表；`reason` 改为实施中/已实施。
3. 系统字典 `document_type` 增加 `replenishment_task`；M-CG 至少一条该类型规则，否则生成 422 `M3_REPLENISH_NUMBERING_UNAVAILABLE`。
4. `auth_permissions` 种子 + 角色授权。
5. 管理端菜单 `m3-replenishment-strategies` / `m3-replenishment-tasks` + `page-query-core-fields.json`。
6. OpenAPI paths + `just openapi-sync`。
7. H4 告警定义种子登记 §6.3 六个 `event_type`。

以上任一项都不属于「以后再说」：未完成则对应验收未过。

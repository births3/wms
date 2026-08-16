# 空间、库位类型、容器质量锁、补货策略与自动化设备领域模型规范 (Storage Location & Quality Lock Model)

本文档定义了 WMS 仓储空间层级、库区库位参数属性、三大作业类型、容器三类质量锁与 M1 系统字典复用机制、库存表在途模型、独立补货引擎以及 AGV/PTL/IoT 设备中台的领域规范。

---

## 1. 空间与属性分层架构

```
🏢 仓库 (Warehouse)
  └── 库区 (Zone) ── [环境温区、品类大区准入、大类功能专区]
        └── 库位 (Location) ── [作业类型、容器管理、安全特控、容量参数、补货水位、AGV与PTL绑定]
              └── 📦 容器 (LPN Container) ── [三类质量锁: 合格/隔离/不合格 + M1 系统字典原因]
```

### 1.1 库区属性 (Zone Attributes)
- `temperature_zone`: 常温 (`normal_10_30`), 阴凉 (`cool_le_20`), 冷藏 (`cold_2_8`), 冷冻 (`freeze_le_minus_20`), 超低温 (`ultra_cold_minus_80`)
  - **基线同步**：既有 CHECK 值域为 `frozen/cold/cool/normal`，Phase 1 同步基线时改 CHECK 并迁移存量数据（`normal→normal_10_30`、`cool→cool_le_20`、`cold→cold_2_8`、`frozen→freeze_le_minus_20`），另新增 `ultra_cold_minus_80`；既有 `frozen` 语义即为 ≤ -20℃ 冷冻，迁移后含义不变。
- `quality_color`: `qualified_green` (合格区), `quarantine_yellow` (隔离区), `unqualified_red` (不合格品区)
  - 质量分区是**容器质量锁移库目标校验的依据**（见 §2.1）：隔离锁仅允许移至 `quarantine_yellow` 库区，不合格锁仅允许移至 `unqualified_red` 库区。
- `allowed_categories`: `["drug", "medical_device", "health_food", "disinfectant", "hazardous", "cosmetics"]`（药 / 械 / 保健品 / 消杀 / 危险品 / 日化，即非药大类）
- `is_external_use_zone`: 是否外用药专区
- `is_fragrant_zone`: 是否易串味专区
- `is_special_drug_zone`: 是否特殊药品专库（毒麻精放）

### 1.2 库位属性 (Location Attributes)
- `location_type`:
  - `storage`: 存储位 / 储备托盘位 / 高架位
  - `case_pick`: 箱拣位 / 整箱拣选位
  - `piece_pick`: 零拣位 / 拆零流利位
- `allows_container`:
  - `storage` 位 = `true`（支持托盘/LPN 容器化管理，位容一体）
  - `case_pick` / `piece_pick` 位 = `false`（散货管理，上架自动解绑脱离原容器）
- `mix_product_policy`: `single_product_only` (一品一位), `restricted_mix` (受控混品)
- `mix_batch_policy`: `single_batch` (单一批次), `multi_batch` (多批次共存)
  - **与容器类型策略的优先级**：库位策略为基座，容器类型策略（ADR-0047 按货主+类型的两布尔）为收紧层，**任一禁止即禁止**（取严格）。即：库位允混且容器类型禁混 → 禁混；库位禁混则不论容器类型一律禁混。
- `current_owner_id`: 当前占用货主 ID（多货主隔离：非空闲库位清空前禁止其他货主存入；**基线同步**：既有 `bound_owner_id` 改名而来，语义不变）
- `lock_status`: `normal` (正常), `lock_in` (禁入), `lock_out` (禁出), `lock_all` (全锁)
  - **与既有 `status` 的关系（基线同步）**：作业锁定只认 `lock_status`；既有 `status` 值域收敛为 `available/occupied/disabled`（物理/档案状态），存量 `locked` 迁移为 `lock_status='lock_all'`，不再作为独立 status 值。
- `replenish_strategy_id`: 补货策略引用（Min-Max 数值不存库位字段，见 §5.3）
- `pick_zone_level`: `gold` (黄金拣选层), `normal` (普通层), `deep` (偏远层)
- `is_agv_managed`: 是否为 AGV 搬运移动货架位（Phase 1 建字段预留，不接硬件）
- `agv_pod_code`: 所属 AGV 移动货架/背篓编码 (如 `POD-001`)
  - AGV 格口编码与三坐标的对应：`POD[货架号]-F[层]-[格位]` ↔ `row_no`=货架号、`layer_no`=层、`column_no`=格位（`POD01-F2-03` = row 01 / layer 2 / column 03）。
  - 电子标签（PTL）地址**不落在库位字段**，统一收敛到 `location_device_bindings` 绑定表（见 §6），绑定角色 `ptl_light`；绑定表 Phase 1 建表预留。

---

## 2. 容器三类质量锁与 M1 系统字典维护机制 (Container Quality Locks)

### 2.1 质量锁三类分类与作业约束

| 质量锁类别 | 编码 | 适用状态 | 作业约束 | 允许移库/上架目标库区（校验依据 `zone.quality_color`） |
|---|---|---|---|---|
| **合格锁** | `qualified` | 正常合格品 | 允许上架合格区、正常波次拣选、出库复核装车 | `qualified_green` 合格区（或任一未设质量分区的普通库区） |
| **隔离锁** | `quarantine` | 隔离待检、温控异常、退货待查、抽检 | **禁止拣选与出库**；仅允许移库/上架至隔离库区 | `quarantine_yellow` 隔离区 |
| **不合格锁** | `rejected` | 破损、过期、检验不合格、召回 | **绝对禁止出库与拣选**；仅允许移库/上架至不合格品库区 | `unqualified_red` 不合格品区 |

**与批次库存质量状态的联动**：容器质量锁是容器级状态，其下各批次库存行（`inventory_batches.status`，值域对齐既有 M3 `inventory_quality_status` 字典）按锁类别联动映射：`quarantine ⇒ quarantined`（隔离）、`rejected ⇒ unqualified`（不合格）、`qualified ⇒ qualified`。加锁/换原因/解锁时对容器下所有库存行同步状态并写入库存流水；上架 6 维校验（ADR-0048 决策 4 第③步）优先读容器质量锁（容器管理位），散货位读批次 `status`，两处任一非 `qualified` 即强阻断。

**触发源仅人工**：加锁/换原因/解锁均为人工操作（管理端容器页 + PDA 扫码），不设系统自动加锁；温控超标、召回令等场景由运营人员依据证据（温控记录、召回文件）人工发起。操作需要 H1 权限点 `m1.quality-lock.manage`（加锁/换原因/解锁），无权限拒绝操作。

### 2.2 复用 M1 系统字典维护锁原因 (`system_dictionary_items`)

无需新造配置表，直接复用 M1 现有系统字典分类与项（字典分类字段名以既有表为准，为 `dict_code`）：
- **隔离原因字典分类**：`dict_code = "container_quarantine_reason"`（预置温控异常、包装破损待检、销退待验、例行抽样等）
- **不合格原因字典分类**：`dict_code = "container_rejected_reason"`（预置药品过期、破损泄漏、检验不合格、药监召回等）
- 运营人员直接在管理端【系统字典】菜单下维护字典项，锁原因关联 `system_dictionary_items.item_code`。

**锁事件表为纯审计表，只 INSERT，禁止 UPDATE/DELETE**（项目审计原则）。加锁、换原因、解锁均为新事件行；当前锁状态冗余在容器主档 `lpn_containers`（`current_lock_category` / `current_lock_reason_item_code`），由解锁/加锁事务同事务更新：

```sql
-- 容器质量锁事件表（纯审计，只 INSERT）
CREATE TABLE container_quality_lock_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    container_id UUID NOT NULL REFERENCES lpn_containers(id),
    container_code VARCHAR(64) NOT NULL,
    event_type VARCHAR(16) NOT NULL,               -- 'lock' | 'change_reason' | 'release'
    lock_category VARCHAR(32),                     -- 'qualified', 'quarantine', 'rejected'（release 事件为 NULL）
    reason_dict_item_code VARCHAR(64),             -- 关联 M1 system_dictionary_items.item_code
    reason_desc TEXT,
    evidence_urls JSONB DEFAULT '[]',              -- 现场照片/药检单附件
    quality_liaison_id UUID REFERENCES quality_liaison_orders(id),  -- 挂接 M-QL 质量联系单（rejected 必填，quarantine 选填）
    operated_by UUID NOT NULL,                     -- 加锁/换原因/解锁操作人
    witness_id UUID,                               -- 双人见证（加锁/解锁按 GSP 双人作业要求）
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    note TEXT
);
```

- 当前锁推导：`lpn_containers.current_lock_category` 为权威字段（与事件表同事务维护）；事件表仅用于审计追溯与解锁留痕，查询历史直接 `SELECT ... WHERE container_id = $1 ORDER BY occurred_at`。
- `qualified` 是默认无锁状态，不需要生成加锁事件；从隔离/不合格解除即 `release` 事件回到 `qualified`，`lock_category` 记 `NULL`。
- 双人见证：加锁与解锁事件必须记录 `witness_id`，缺少见证人时事务拒绝提交。
- **M-QL 挂接**：加锁事件必须（`rejected`）/ 可以（`quarantine`）关联 `quality_liaison_orders` 质量联系单（对齐 `stock_adjustment_orders.quality_liaison_id` 先例，`related_document_type = 'container_quality_lock'`、`related_document_no = container_code`）；**解锁前置条件：关联 M-QL 已办结（`closed`），未办结禁止解锁**。

---

## 3. 库位批量生成向导 (Batch Location Generator)

PC 管理端提供向导式批量生成器，支持规则定义与一键生成：
- **普通静态货架规则**：`[前缀]-[巷道:01..05]-[排架:01..10]-[层:01..04]-[格位:01..06]`
- **AGV 移动货架规则**：`POD[货架号:01..50]-F[层:1..5]-[格位:01..08]`
- **属性批量填充**：生成时直接批量赋予作业类型（`location_type`）、温区及容器开关。
- **导入导出**：支持 Excel 标准格式批量导入与全仓导出。

---

## 4. 核心库存表与在途补货设计 (Inventory Table Schema)

库存核心表原生承载在途补货量，单条 SQL 毫秒级完成实时可用量计算：

```sql
CREATE TABLE inventory_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL,
    zone_id UUID NOT NULL,
    location_id UUID NOT NULL,                  -- 对应 AGV 货架格口或固定库位
    owner_id UUID NOT NULL,                     -- 货主 ID (多货主隔离)
    product_id UUID NOT NULL,                   -- 商品 ID
    batch_no VARCHAR(64) NOT NULL,              -- 生产批号
    production_date DATE,                       -- 生产日期
    expiry_date DATE NOT NULL,                  -- 效期
    container_lpn VARCHAR(64),                  -- 托盘容器码 (存储位有值，零拣/箱拣为 NULL)

    -- 核心实时平衡字段
    qty_on_hand NUMERIC(12, 4) NOT NULL DEFAULT 0,              -- 在手物理库存
    qty_allocated NUMERIC(12, 4) NOT NULL DEFAULT 0,            -- 已分配占用量 (出库占用)
    qty_replenish_in_transit NUMERIC(12, 4) NOT NULL DEFAULT 0, -- 补货在途量 (正在补向该库位，目标侧)
    qty_replenish_out_transit NUMERIC(12, 4) NOT NULL DEFAULT 0, -- 补货下架在途量 (该库位正被补货取走，来源侧)
    qty_frozen NUMERIC(12, 4) NOT NULL DEFAULT 0,               -- 质检/质量冻结量

    status VARCHAR(32) NOT NULL DEFAULT 'qualified',            -- qualified, quarantined, unqualified（对齐 M3 字典）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**与既有基线的差异与迁移清单（Phase 1 同步基线，v1 前不做兼容过渡）**：

| 既有字段 | 新模型 | 迁移说明 |
|---|---|---|
| `product_code` (TEXT) | `product_id` (UUID) | 关联商品主档；既有库存行按商品主档回填 ID |
| `quality_status` (TEXT，值域走 M3 `inventory_quality_status` 字典) | `status` (`qualified/quarantined/unqualified`) | 改名对齐既有字典三态（`loss_deducted`/`pending_destruction` 保留在字典中）；同步改 M3 召回/质检作业引用 |
| `qty_locked` | `qty_frozen` | 语义不变（质检/质量冻结），改名同步改 M3 作业引用 |
| `location_code` (TEXT) | `location_id` (UUID 外键) + 冗余 `zone_id` | zone 可由 location 推导，冗余仅用于避免跨表 join；`zone_id` 必须与 location 所属 zone 一致 |
| （无） | `container_lpn` | 已有 lpn_container 切片部分落地，本文档确认纳入统一模型 |
| （无） | `qty_replenish_in_transit` / `qty_replenish_out_transit` | 补货在途双字段，见 §5 |

唯一约束相应调整为 `UNIQUE (owner_id, product_id, batch_no, location_id, status)`（整托场景下 `container_lpn` 不参与唯一约束，同托同品同批加量走行锁合并）。

---

## 5. 独立补货系统设计 (Replenishment Subsystem)

### 5.1 补货策略配置与触发模式
1. **日常安全水位 Min-Max 驱动**：闲时/夜间巡检，当目标拣选位 `qty_on_hand + qty_replenish_in_transit <= min_safety_threshold` 时，自动生成从高层存储位下架补货任务至 `max_replenish_target`；
2. **出库波次缺口即时驱动**：波次算单时若发现拣选位可用量不足以满足订单，即时触发高优先级波次紧急补货任务；
3. **批次锁定与动线**：按 `FEFO`（近效期先出）锁定来源存储位托盘，送达拣选位扫码/拍灯确认后，在途量转为在手散件量，释放原托盘。

### 5.2 在途双字段与实时可用量公式

补货任务生成时在**同一事务**内：目标拣选位 `qty_replenish_in_transit` 原子 +Δ（避免重复触发补货），来源存储位 `qty_replenish_out_transit` 原子 +Δ（锁定来源在手量，防重复下架）；补货上架确认时两字段同时回冲，目标位 `qty_on_hand` +Δ、来源位 `qty_on_hand` −Δ。

实时可用量公式（单条 SQL，无需聚合任务表）：

```
拣选位可用量  = qty_on_hand − qty_allocated − qty_frozen + qty_replenish_in_transit
存储位可下架量 = qty_on_hand − qty_allocated − qty_frozen − qty_replenish_out_transit
```

`qty_frozen` 仅承载质检/质量冻结（含容器质量锁联动冻结），与补货在途互不混淆。

### 5.3 补货策略表（Min-Max 数值唯一归属）

Min-Max 数值只存策略表（按货主 + 库位组/品类配置），库位仅挂 `replenish_strategy_id` 引用，不存数值；改一处策略即对整组拣选位生效：

```sql
CREATE TABLE replenishment_strategies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,                     -- 货主（多货主隔离，策略按货主配置）
    strategy_code VARCHAR(64) NOT NULL,
    strategy_name VARCHAR(128) NOT NULL,
    scope_type VARCHAR(16) NOT NULL,            -- 'location_group' | 'category'：按库位组或按品类
    scope_ref UUID NOT NULL,                    -- 库位组 ID 或品类 ID
    location_type VARCHAR(16) NOT NULL,         -- 'case_pick' | 'piece_pick'（只对拣选位生效）
    min_safety_threshold NUMERIC(12, 4) NOT NULL,  -- 低于此值触发补货
    max_replenish_target NUMERIC(12, 4) NOT NULL,  -- 补到为止
    trigger_modes TEXT[] NOT NULL DEFAULT '{min_max, wave_gap}',  -- 驱动模式：安全水位/波次缺口
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, strategy_code)
);
```

- `warehouse_locations.replenish_strategy_id` 引用本表（可空，未挂策略的拣选位不参与 Min-Max 巡检）。
- Min-Max 巡检判定（目标拣选位）：`qty_on_hand + qty_replenish_in_transit <= min_safety_threshold` 触发，补货目标 `max_replenish_target`；波次缺口即时驱动按同一表配置。

---

## 6. AGV、电子标签与通用硬件设备中台架构 (IoT / WCS Layer)

通过 4 张通用中台表实现任意设备（AGV、PTL、DWS、RFID、立库堆垛机）的插件化接入：

```
1. iot_devices              ── 登记所有物理设备实例 (IP、端口、协议、厂商、状态)
2. location_device_bindings ── 库位与设备点位解耦映射 (绑定角色: ptl_light, rfid_antenna)
3. wcs_tasks                ── 统一下发硬件指令 (搬运货架 pod_move, 亮灯 ptl_light_on, 分拣 sorter_divert)
4. iot_event_logs           ── 接收硬件事件回传 (拍灭按键、DWS 称重体积数据、RFID 批量扫描 EPC)
```

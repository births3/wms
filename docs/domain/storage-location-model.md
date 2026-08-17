# 空间、库位类型、容器质量锁、补货策略与自动化设备领域模型规范 (Storage Location & Quality Lock Model)

本文档定义了 WMS 仓储空间层级、库区库位参数属性、三大作业形态、容器三类质量锁与 M1 系统字典复用机制、库存表在途模型、独立补货引擎以及 AGV/PTL/IoT 设备中台的领域规范。

---

## 1. 空间与属性分层架构

```
🏢 仓库 (Warehouse)
  └── 库区 (Zone) ── [环境温区、品类大区准入、大类功能专区]
        └── 库位 (Location) ── [作业形态、容器管理、安全特控、容量参数、补货水位、AGV与PTL绑定]
              └── 📦 容器 (LPN Container) ── [三类质量锁: 合格/隔离/不合格 + M1 系统字典原因]
```

### 1.1 库区属性 (Zone Attributes)
- `temperature_zone`: 常温 (`normal_10_30`), 阴凉 (`cool_le_20`), 冷藏 (`cold_2_8`), 冷冻 (`freeze_le_minus_20`), 超低温 (`ultra_cold_minus_80`)
  - **基线同步**：既有 CHECK 值域为 `frozen/cold/cool/normal`，Phase 1 同步基线时改 CHECK 并迁移存量数据（`normal→normal_10_30`、`cool→cool_le_20`、`cold→cold_2_8`、`frozen→freeze_le_minus_20`），另新增 `ultra_cold_minus_80`；既有 `frozen` 语义即为 ≤ -20℃ 冷冻，迁移后含义不变。
  - **温区匹配规则（6 维校验②）**：目标库位温区范围必须**被商品存储温区范围包含**（库位温区上限 ≤ 商品温区上限 且 库位温区下限 ≥ 商品温区下限）。例：商品要求 `cold_2_8` → 仅可上 `cold_2_8` 库区；要求 `normal_10_30` → 可上 `normal_10_30` 或 `cool_le_20`（10-20 ⊂ 10-30，更凉不违规）；要求 `cool_le_20` → 不可上 `normal_10_30`（上限 30 超 20）也不可上 `cold_2_8`（下限 2 低于 10 常规阴凉下限，按实际温度区间判定）。
- `quality_color`: `qualified_green` (合格区), `quarantine_yellow` (隔离区), `unqualified_red` (不合格品区)
  - 质量分区是**容器质量锁移库目标校验的依据**（见 §2.1）：隔离锁仅允许移至 `quarantine_yellow` 库区，不合格锁仅允许移至 `unqualified_red` 库区。
- `allowed_categories`: `["drug", "medical_device", "health_food", "disinfectant", "hazardous", "cosmetics"]`（药 / 械 / 保健品 / 消杀 / 危险品 / 日化，即非药大类）
- `is_external_use_zone`: 是否外用药专区
- `is_fragrant_zone`: 是否易串味专区
- `is_special_drug_zone`: 是否特殊药品专库（毒麻精放）

### 1.2 库位属性 (Location Attributes)
- `location_type`（**作业形态**，与 ADR-0048 标题"库位三级作业形态"统一用语）:
  - `storage`: 存储位 / 储备托盘位 / 高架位
  - `case_pick`: 箱拣位 / 整箱拣选位
  - `piece_pick`: 零拣位 / 拆零流利位
  - **概念注记（消除歧义）**：**整托（glossary #72）是作业粒度**——"以整个容器为一次作业对象"，容器可为托盘/周转箱/保温箱/出库箱等任意类型，**不限于托盘，也不专属存储位**；**存储位（glossary #73）是位置形态**——承载容器化库存的库位。两者正交：整托上架/移库/出库发生在存储位，整托与散货是作业粒度对，存储位/箱拣位/零拣位是位置形态三分。约束"存储位仅接受容器粒度上架"（6 维⑤）是位置形态设计，不改变整托定义。
  - **多义词注记**：本文档中"**锁定**"按语境区分——① 容器质量锁（§2，质量管控）；② 库位锁定 `lock_status`（§1.2，作业禁入/禁出）；③ 批次行锁（SQL `FOR UPDATE`，并发控制）；④ 来源在手量锁定（`qty_replenish_out_transit`，账务预留）。"**在途**"按语境区分——① 补货在途双字段（账务口径，§5.2）；② 容器物理流转状态 `in_transit`（ADR-0047）；③ 收货/ASN 在途（M2 既有）。引用时如无上下文请带限定词。
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
- `pick_sequence_no` / `putaway_sequence_no`: **拣选/上架路径顺序字段**——拣选任务 `route_sequence` 与 PDA 上架推荐按此排序（NULL 回落 `location_code` 升序，兼容存量）；**路径策略以字段为唯一权威**（M4-010 升序/S 型等退化为字段排序，不另做动态派生）；批量生成向导按巷道-排-层-格自然序自动生成（支持巷道方向翻转参数），Excel 导入可覆盖
- `pick_zone_level`: `gold` (黄金拣选层), `normal` (普通层), `deep` (偏远层)
- `is_agv_managed`: 是否为 AGV 搬运移动货架位（Phase 1 建字段预留，不接硬件）
- `agv_pod_code`: 所属 AGV 移动货架/背篓编码 (如 `POD-001`)
  - AGV 格口编码与三坐标的对应：`POD[货架号]-F[层]-[格位]` ↔ `row_no`=货架号、`layer_no`=层、`column_no`=格位（`POD01-F2-03` = row 01 / layer 2 / column 03）。
  - 电子标签（PTL）地址**不落在库位字段**，统一收敛到 `location_device_bindings` 绑定表（见 §6），绑定角色 `ptl_light`；绑定表 Phase 1 建表预留。

### 1.3 商品侧空间属性 (Product Space Attributes)

6 维校验⑥（外用/易串味互斥）需要商品侧标记作为校验依据，`products` 扩展两字段（Phase 1 同步基线）：

- `is_external_use`: 是否外用药（与库区 `is_external_use_zone` 互斥校验：外用药仅可上外用药专区，非外用不可上）
- `is_fragrant`: 是否易串味商品（与库区 `is_fragrant_zone` 互斥校验：易串味仅可上易串味专区，普通商品不可上与易串味混位）

---

## 2. 容器三类质量锁与 M1 系统字典维护机制 (Container Quality Locks)

### 2.1 质量锁三类分类与作业约束

| 质量锁类别 | 编码 | 适用状态 | 作业约束 | 允许移库/上架目标库区（校验依据 `zone.quality_color`） |
|---|---|---|---|---|
| **合格锁** | `qualified` | 正常合格品 | 允许上架合格区、正常波次拣选、出库复核装车 | `qualified_green` 合格区（默认值，未设质量分区的库区均为合格区） |
| **隔离锁** | `quarantine` | 隔离待检、温控异常、退货待查、抽检 | **禁止拣选与出库**；仅允许移库/上架至隔离库区 | `quarantine_yellow` 隔离区 |
| **不合格锁** | `rejected` | 破损、过期、检验不合格、召回 | **绝对禁止出库与拣选**；仅允许移库/上架至不合格品库区 | `unqualified_red` 不合格品区 |

**与批次库存质量状态的联动（因-果关系）**：容器质量锁是**管控动作（因）**，批次质量状态是**库存行表现（果）**——容器加锁，其下各批次库存行（`inventory_batches.status`，值域对齐既有 M3 `inventory_quality_status` 字典五值：`qualified/quarantined/unqualified/loss_deducted/pending_destruction`）按锁类别联动映射：`quarantine ⇒ quarantined`（隔离）、`rejected ⇒ unqualified`（不合格）、`qualified ⇒ qualified`；`loss_deducted`（报损扣减）与 `pending_destruction`（待销毁）由报损/销毁流程管理，不参与质量锁联动。**反向不成立**：批次状态可脱离容器锁单独存在（散货轻量路径，M3 质检直接置状态，无容器锁）；容器锁一定伴随批次联动。加锁/换原因/解锁时对容器下所有库存行同步状态并写入库存流水；上架 6 维校验（ADR-0048 决策 4 第③步）优先读容器质量锁（容器管理位），散货位读批次 `status`，两处任一非 `qualified` 即强阻断。四概念对照（容器质量锁/批次质量状态/库位锁定 `lock_status`/数量冻结 `qty_frozen`）见 §1.2 多义词注记。

**加锁前置状态与解锁回写（防止覆盖竞态）**：
- **加锁前置**：仅 `in_use` 的容器可加锁（含**未上架容器**：验收/移入容器时发现异常可先锁，无需等待上架）；`idle`（空容器）、`in_transit`、`recycling`、`shipped` 状态禁止加锁（无内容物可锁或流转中不可锁）。
  - **未上架容器的联动语义**：有库存行则联动批次状态；无库存行（未上架）则仅容器级锁、批次联动为空操作，**上架时由 6 维③ 读容器质量锁强阻断兜底**；波次算单口径不变（未上架库存无库存行，不可见、不参与分配）。
- **加锁时已分配占用的处置**：加锁联动时，对容器下批次行 `qty_allocated > 0` 的已分配量**释放分配并标记订单行"等待重新分配"**（对齐补货取消的波次联动语义）；已释放的分配不得再拣选（拣选校验读容器质量锁强阻断兜底）。释放分配与状态联动同事务；**波次重算单为异步通知**——加锁事务提交后发出重算单事件（复用既有事件/消息机制），由波次模块异步消费，不在加锁事务内同步调用波次算单（避免大事务与模块耦合）。
- **解锁回写规则**：解锁只回写**仍处于本锁联动状态**的批次行（`quarantined`/`unqualified` 且由本锁设置）；锁期间被质检/召回等其他流程变更过的行**不回写、不覆盖**，解锁操作完成后提示人工复核这些行。禁止无条件将全部行回写 `qualified`（会抹掉锁期间产生的质检结论）。

**触发源仅人工**：加锁/换原因/解锁均为人工操作（管理端容器页 + PDA 扫码），不设系统自动加锁；温控超标、召回令、养护异常（M3-004 在库养护发现外观/温控/破损异常）等场景由运营人员依据证据（温控记录、召回文件、养护记录）人工发起。操作需要 H1 权限点 `m1.quality-lock.manage`（加锁/换原因/解锁），无权限拒绝操作；养护/质检异常记录可跳转"创建联系单并加锁"入口。

**适用边界（粒度与散货）**：
- **容器质量锁是容器（整托）粒度**：锁 = 整托全量隔离处置。个别件异常（整托内单箱破损等）应先拆托（异常件拆出单独容器）或按批次行 `qty_frozen`/质检状态部分处理，不直接锁整托；是否锁整托由运营按异常性质决策。
- **散货加锁 = 装箱容器化（完整管控路径）**：箱拣位/零拣位的散货需要加锁管控时，**先移入容器（周转箱）→ 容器加锁 → 上架质量区存储位**；装箱加锁走容器质量锁全流程（事件表/双人见证/M-QL 挂接/锁-区域约束）。**未装箱或装箱后未上架到存储位 → 无库存行，波次分配不到**（同未上架容器语义）。
- **散货轻量路径（日常质检，不走容器锁）**：散货异常仅需状态管控时走既有 M3 质检流程（批次 `status` 置 `quarantined`/`unqualified` + `qty_frozen` 部分冻结，审批源为 M3 质检/养护流程）；6 维校验③ 散货位读批次 `status` 即为此路兜底。两条路径由运营按异常性质选择：全量管控/审计要求 → 装箱容器化；部分件/临时管控 → 轻量质检状态。

**锁-区域映射与流转逻辑（加锁即物理隔离）**：

**① 锁类别 ↔ 区域强约束**（容器所在库区的 `zone.quality_color` 必须与锁类别匹配）：

| 容器质量锁 | 允许所在区域 | 禁止所在区域 |
|---|---|---|
| `qualified` | 合格区（默认，未设质量分区的库区均为合格区） | 隔离区、不合格品区（合格品不得滞留质量区） |
| `quarantine` | 仅隔离区 `quarantine_yellow` | 合格区、不合格品区（两质量区互斥：待检≠不合格） |
| `rejected` | 仅不合格品区 `unqualified_red` | 合格区、隔离区 |

**② 加锁时位置处置**：加锁事务成功（含未上架容器）→ **同事务生成隔离移库任务**（移库类型 `lock_move`，**复用 M3-006 库内移库作业流**，携带 `lock_move` 类型标记与质量锁权限校验，不可与普通移库混淆）；目标 = 锁类别对应区域，系统推荐该区空闲位；允许暂缓移库（当前作业不可中断等），但**暂缓期间容器禁止一切作业**；超时未移库（默认 2 小时，可配置）告警质量管理员（H4 企微 + 告警中心）；移库到位校验目标区 `quality_color` 匹配（6 维③ 的移库形态）。

**③ 解锁后位置处置**：解锁回 `qualified` → 同事务生成移回合格区移库任务（推荐原库位或系统推荐位）；移出前该容器仅允许移库作业、不参与新波次分配。

**④ 质量区算单排除**：`zone.quality_color ∈ {quarantine_yellow, unqualified_red}` 的库位**不参与波次算单分配**（解锁后仍在质量区也不可被分单，直至移回合格区）。

**④-3 订单/波次带锁感知（带锁库存只波次到处置场景）**：
- **普通销售/补货订单**：波次算单**分配不到任何带锁库存**——容器质量锁非 `qualified` 或批次 `status` 非 `qualified` 的库存一律排除（质量区排除 + 未上架无行 + 本条状态过滤三层兜底）；
- **处置类单据**（不合格品出库/隔离处置/报损销毁转出）：允许波次到对应带锁库存，且仍受锁-区域约束（不合格锁库存仅从不合格品区转出，走报损/销毁流程）；处置类单据的订单行携带锁标记（关联 `container_quality_lock_events` 或批次状态），波次按锁类别匹配对应处置类型。
- **生效时序与审批解耦**：**加锁提交即生效**（锁状态过滤自加锁提交起拦截），M-QL 质量审批是**处置结论路径**而非加锁前置——审批期间锁持续有效，普通订单始终不可分配；解锁才要求审批完成（`closed` 前置）。
- **加锁时在途拣选的收口**：加锁联动释放分配时，对已下发拣选任务（拣选单已生成/拣选中）——任务标记"质量锁中断"，对应订单行释放回波次重新算单，PDA 端任务失效提示；作业员扫容器被 6 维③ 强阻断，被中断任务库存不扣减；已物理出库完成的部分属加锁前合法行为，凭库存流水审计，不追溯。

**④-2 库位形态约束（加锁容器禁上散货位）**：`quarantine`/`rejected` 加锁容器的目标位必须同时满足 **质量区（对应 `quality_color`）∩ 存储位（`location_type='storage'` 且 `allows_container=true`）**——加锁容器保持容器粒度（整托追踪），**禁止上箱拣位（`case_pick`）与零拣位（`piece_pick`）**（散货位上架自动解绑，将失去容器级管控）；6 维⑤ 包装粒度防呆叠加执行。
质量区（隔离区/不合格品区）应同时规划两种形态库位：**容器位**（承接加锁容器）与**散货位**（承接批次状态异常的散货，走 M3 质检流程）；散货异常商品移入质量区散货位不涉及容器锁。

**⑤ 区域内作业矩阵**：

| 作业 | 合格品·合格区 | 隔离锁·隔离区 | 不合格锁·不合格品区 |
|---|---|---|---|
| 上架/移库进入 | ✓ | ✓ | ✓ |
| 区内移库 | ✓ | ✓ | ✓ |
| 波次分配 / 拣选 / 出库 | ✓ | ✗ | ✗ |
| 报损/销毁转出 | — | — | ✓（转 M3 报损/销毁流程） |
| 锁变更（换原因） | — | ✓ | ✓ |

**⑥ 一致性校验与对账**：批次 `status` ↔ 所在 `zone.quality_color` 强约束——`qualified` 批次不得位于质量区；`quarantined` 不得位于合格/不合格区；`unqualified` 不得位于合格/隔离区；加锁、移库、解锁各环节校验；**治理对账脚本**（规划：`scripts/governance/` 下新增质量锁一致性检查，纳入 gov-t1 或独立定时任务）扫描全部库存行与所在库区匹配，发现错位告警。
**错位修复路径**：对账/告警发现错位后，由质量管理员在管理端人工处置（重新加锁/解锁/移库至正确区域），修复动作走正常质量锁流程与审计（事件表 + 双人见证），禁止绕过流程直接改库位或状态。

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
    lpn_code VARCHAR(64) NOT NULL,                 -- 冗余主档码（与 lpn_containers.lpn_code 同名，便于追溯）
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

CREATE INDEX IF NOT EXISTS container_quality_lock_events_container_idx
    ON container_quality_lock_events (container_id, occurred_at DESC);
```

- 当前锁推导：`lpn_containers.current_lock_category` 为权威字段（与事件表同事务维护）；事件表仅用于审计追溯与解锁留痕，查询历史直接 `SELECT ... WHERE container_id = $1 ORDER BY occurred_at`。
- `qualified` 是默认无锁状态，不需要生成加锁事件；从隔离/不合格解除即 `release` 事件回到 `qualified`，`lock_category` 记 `NULL`。
- 双人见证：加锁与解锁事件必须记录 `witness_id`（见证人），且见证人与操作人（`operated_by`）必须为不同用户（GSP 双人作业）；缺见证人或见证人重复时事务拒绝提交。
- **M-QL 挂接（审批源）**：加锁/解锁为库存状态变更，审批源 = M-QL 质量联系单（`approval_source=M-QL` + 单号）；加锁事件必须（`rejected`）/ 可以（`quarantine`）关联 `quality_liaison_orders`（对齐 `stock_adjustment_orders.quality_liaison_id` 先例，`related_document_type = 'container_quality_lock'`、`related_document_no = lpn_code`）；`release` 事件携带同一 `quality_liaison_id` 留痕（校验通过后写入）。
- **解锁前置 = 审批走完（closed 或 rejected 均可）**：关联 M-QL 状态为 **`closed`**（处置办结，正常解锁）或 **`rejected`**（质量部门不同意加锁/处置不成立，回退解锁）时允许解锁，其余状态（`pending_approval`/`approved`/`pending_erp_sync`/`landed`/`sync_failed`）禁止；`rejected` 解锁同样双人见证，回写规则不变（只回写本锁联动状态行），`release` 事件 `note` 记录驳回原因与审批人；`rejected` 后如需其他处置（改锁类别/转报损）先解锁再按新结论重新发起，不原地改单。**由此保证：审批无论同意还是驳回，锁都能随质量结论闭环解除，不存在"审批不同意但锁解不掉"的死锁。**
- **外键约定**：新表外键统一单列引用目标表 PK（`REFERENCES xxx(id)`），对齐既有先例（`inventory_movements.batch_id`、`stock_adjustment_orders.quality_liaison_id`）；跨货主引用校验由 service 层 owner_id 校验兜底，不设复合外键。
- **操作编排与权限链**：先建 M-QL（需 `mql.quality-liaison.write` 权限）→ 再加锁挂单号（需 `m1.quality-lock.manage` 权限）；管理端容器页提供"创建联系单并加锁"联动入口（一次录入，两步完成），不要求加锁操作员同时持有两权限。

---

## 3. 库位批量生成向导 (Batch Location Generator)

PC 管理端提供向导式批量生成器，支持规则定义与一键生成：
- **普通静态货架规则**：`[前缀]-[巷道:01..05]-[排架:01..10]-[层:01..04]-[格位:01..06]`
- **AGV 移动货架规则**：`POD[货架号:01..50]-F[层:1..5]-[格位:01..08]`
- **属性批量填充**：生成时直接批量赋予作业形态（`location_type`）、温区及容器开关。
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

    status VARCHAR(32) NOT NULL DEFAULT 'qualified',            -- qualified, quarantined, unqualified, loss_deducted, pending_destruction（对齐 M3 字典）
    recall_flag BOOLEAN NOT NULL DEFAULT FALSE, -- 召回标记（保留既有，M3 召回流程引用）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 1           -- 乐观锁（保留既有）
);
```

**与既有基线的差异与迁移清单（Phase 1 同步基线，v1 前不做兼容过渡）**：

| 既有字段 | 新模型 | 迁移说明 |
|---|---|---|
| `product_code` (TEXT) | `product_id` (UUID) | 关联商品主档；既有库存行按商品主档回填 ID |
| `quality_status` (TEXT，值域走 M3 `inventory_quality_status` 字典) | `status` | 改名对齐既有字典五值（`qualified/quarantined/unqualified/loss_deducted/pending_destruction`）；同步改 M3 召回/质检作业引用 |
| `qty_locked` | `qty_frozen` | 语义不变（质检/质量冻结），改名同步改 M3 作业引用 |
| `location_code` (TEXT) | `location_id` (UUID 外键) + 冗余 `zone_id` | zone 可由 location 推导，冗余仅用于避免跨表 join；`zone_id` 必须与 location 所属 zone 一致 |
| `recall_flag` / `version` | 保留（schema 中未省略） | 召回标记与乐观锁继续使用，M3 召回/质检作业引用不改名 |
| （无） | `warehouse_id` | 新增冗余列（与 `zone_id` 同理由，可由 location 推导），迁移回填 |
| （无） | `container_lpn` | 已有 lpn_container 切片部分落地，本文档确认纳入统一模型 |
| （无） | `qty_allocated` | 新增分配占用冗余列：与既有 `inventory_allocations` 分配流水**同事务维护**（分配/释放时同步增减），算单直接读库存行、不再 join 分配表；既有分配数据按行汇总回填 |
| （无） | `qty_replenish_in_transit` / `qty_replenish_out_transit` | 补货在途双字段，见 §5 |

唯一约束相应调整为 `UNIQUE (owner_id, product_id, batch_no, location_id, status)`（整托场景下 `container_lpn` 不参与唯一约束，同托同品同批加量走行锁合并）。

**索引同步变更**（与改名/新增字段一致，实施时同步重建）：
- `inventory_batches_owner_product_batch_idx` → `(owner_id, product_id, batch_no)`（算单按商品过滤）
- `inventory_batches_owner_location_status_idx` → `(owner_id, location_id, status)`（锁状态过滤/质量区查询；算单三层过滤的 `status != 'qualified'` 排除走此索引）
- `inventory_batches_owner_expiry_idx` 保留 `(owner_id, expiry_date)`（FEFO 来源选择）
- 新增建议：`(owner_id, zone_id, status)`（质量区算单排除的 zone 维度过滤）

---

## 5. 独立补货系统设计 (Replenishment Subsystem)

### 5.1 补货策略配置与触发模式
1. **日常安全水位 Min-Max 驱动**：巡检按**时段+频率双参数**执行（白天每 60 分钟 / 夜间每 15 分钟，可配置）；当目标拣选位可用量口径 `qty_on_hand − qty_allocated − qty_frozen + qty_replenish_in_transit <= min_safety_threshold` 时，按策略动线（§5.3）自动生成补货任务至 `max_replenish_target`（**与 §5.2 可用量公式同一口径**，避免账面够但被分配完导致巡检不触发）；
2. **出库波次缺口即时驱动**：波次算单时若发现拣选位可用量不足以满足订单，**缺口量 = 订单需求 − 当前可用量（含在途）**，缺口 > 在途量才触发 urgent 补货任务（`urgent_qty = 缺口 − 在途量`）；巡检任务的在途量已自然覆盖缺口时不重复生成（**在途双字段是唯一权威**）；
3. **批次锁定与动线**：**来源选择按 FEFO**（近效期批次先补到拣选位，拣选位按订单 FEFO 出库），锁定来源批次行；送达拣选位扫码/拍灯确认后，在途量转为在手散件量，释放原托盘；
4. **数量计算（双约束 + 包装取整）**：目标量 = `max_replenish_target − 当前可用量`；实际任务量 = `min(目标量, 来源可下架量)`，按最小包装粒度取整（尾数不足 1 包装留待下轮）；来源可下架量 = `qty_on_hand − qty_allocated − qty_frozen − qty_replenish_out_transit`；
5. **目标位校验**：生成时复用 6 维校验②③⑥（温区匹配/容器质量锁状态/容量防呆——`max_replenish_target` 不超目标位剩余容量与混品上限），任一不过则该位跳过本轮并记巡检日志；
6. **生成失败处理**：每次失败写巡检日志（目标位/商品/原因）；同一目标位同因连续 3 次失败触发告警（H4 企微 + 大盘高亮），防"静默缺货"。

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
    scope_type VARCHAR(16) NOT NULL,            -- 'location_group' | 'category' | 'product'：库位组/品类/单商品
    scope_ref UUID NOT NULL,                    -- 库位组 ID / 品类 ID / product_id
    location_type VARCHAR(16) NOT NULL,         -- 'case_pick' | 'piece_pick'（只对拣选位生效）
    source_type VARCHAR(16) NOT NULL DEFAULT 'storage',  -- 动线来源形态：storage / case_pick
    target_type VARCHAR(16) NOT NULL,           -- 动线目标形态：case_pick / piece_pick
    min_safety_threshold NUMERIC(12, 4) NOT NULL,  -- 低于此值触发补货
    max_replenish_target NUMERIC(12, 4) NOT NULL,  -- 补到为止
    trigger_modes TEXT[] NOT NULL DEFAULT '{min_max, wave_gap}',  -- 驱动模式：安全水位/波次缺口
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, strategy_code),
    CHECK (scope_type IN ('location_group', 'category', 'product')),
    CHECK (source_type IN ('storage', 'case_pick')),
    CHECK (target_type IN ('case_pick', 'piece_pick'))
    -- 动线合法组合（storage→case_pick / storage→piece_pick / case_pick→piece_pick）由代码校验，CHECK 只限单字段值域
);
```

策略 scope 的 `location_group` 类型需要库位组实体（按组批量挂策略与批量改水位）：

```sql
CREATE TABLE replenishment_location_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,
    group_code VARCHAR(64) NOT NULL,
    group_name VARCHAR(128) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, group_code)
);

CREATE TABLE replenishment_location_group_members (
    group_id UUID NOT NULL REFERENCES replenishment_location_groups(id),
    location_id UUID NOT NULL,
    PRIMARY KEY (group_id, location_id)
);
```

- `warehouse_locations.replenish_strategy_id` 引用策略表（可空，未挂策略的拣选位不参与 Min-Max 巡检）。
- Min-Max 巡检判定（目标拣选位，可用量口径）：`qty_on_hand − qty_allocated − qty_frozen + qty_replenish_in_transit <= min_safety_threshold` 触发，补货目标 `max_replenish_target`；波次缺口即时驱动按同一表配置。
- **scope 维度与优先级**：`product`（scope_ref=商品 ID，精确到商品，药品场景必备）> `category` > `location_group`（最粗兜底）；同目标位多策略命中时取最精确维度。
- **补货动线（可配置）**：`source_type → target_type` 合法组合——`storage→case_pick`（整箱补货，整托动线）、`storage→piece_pick`（拆零补货）、`case_pick→piece_pick`（二级动线，可选启用）；`source_type='case_pick'` 时来源为散货位，下架按箱取件（无 LPN）；动线校验叠加 GSP 强制项（温区/品类/容器质量锁）。

### 5.4 补货任务实体与状态机 (Replenishment Task)

任务在触发时同事务内生成并更新在途双字段（见 §5.2）；任务表仅记录作业过程，不参与算量。**调度复用**：巡检/触发调度复用既有 M-TE 任务引擎调度骨架，补货任务实体与状态机独立，不混入 M-TE 单据；作业权限：PC 侧 `m3.replenishment.manage`（策略配置/大盘操作），PDA 侧 `m3.replenishment.execute`（领取执行）：

```sql
CREATE TABLE replenishment_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,
    task_no VARCHAR(64) NOT NULL,               -- 编号（走 M-CG 编号规则）
    trigger_mode VARCHAR(16) NOT NULL,          -- 'min_max' | 'wave_gap' | 'manual'
    priority VARCHAR(16) NOT NULL DEFAULT 'normal',  -- 'normal' | 'urgent'（波次缺口任务为 urgent）
    strategy_id UUID REFERENCES replenishment_strategies(id),
    source_location_id UUID NOT NULL,           -- 来源存储位
    source_batch_id UUID NOT NULL,              -- 来源批次（FEFO 锁定）
    source_lpn_id UUID,                         -- 来源容器（整托下架，散货为 NULL）
    target_location_id UUID NOT NULL,           -- 目标拣选位
    product_id UUID NOT NULL,
    batch_no VARCHAR(64) NOT NULL,
    qty NUMERIC(12, 4) NOT NULL CHECK (qty > 0),      -- 任务数量
    picked_qty NUMERIC(12, 4) NOT NULL DEFAULT 0,     -- 已下架未送达量（PDA 下架时登记，确认时回冲）
    done_qty NUMERIC(12, 4) NOT NULL DEFAULT 0,       -- 已完成数量（支持部分执行）
    status VARCHAR(16) NOT NULL DEFAULT 'pending',    -- pending / in_progress / done / cancelled
    operator_id UUID,                           -- PDA 作业员（领取时写入）
    confirmed_at TIMESTAMPTZ,                   -- 目标位确认完成时间
    cancel_reason TEXT,
    created_by TEXT NOT NULL,                   -- 触发来源（巡检引擎/波次号/人工用户名）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, task_no)
);

CREATE INDEX IF NOT EXISTS replenishment_tasks_owner_status_target_idx
    ON replenishment_tasks (owner_id, status, target_location_id);
CREATE INDEX IF NOT EXISTS replenishment_tasks_owner_source_batch_idx
    ON replenishment_tasks (owner_id, source_batch_id);
```

**状态机**：`pending`（已生成，双字段已 +Δ）➜ `in_progress`（PDA 领取，写 `operator_id`）➜ `done`（目标位确认完成，双字段回冲 + 在手转换）；`cancelled` 仅可从 `pending`/`in_progress` 由人工或超时触发（回冲双字段，来源解冻）。`done_qty = qty` 时任务才置 `done`；部分执行后剩余量仍由原任务承担。**取消前置：`done_qty = 0`**（未开始下架）；已部分执行的任务不可取消（下架的物理货已在流转），需完成剩余量或人工介入处理。

**来源选择（FEFO）与作业顺序**：
- **来源选择按 FEFO**：按目标商品从来源候选（策略动线内）选**效期最近**的批次锁定（近效期先补到拣选位，拣选位按订单 FEFO 出库）；任务生成时对 `source_batch_id` 行加行锁（`SELECT ... FOR UPDATE`）并校验来源可用量（§5.2 公式 > 0），不足则跳过改选下一候选；一任务一批次，来源批次不足时按 FEFO 顺序生成多个任务。
- **作业顺序不按 FEFO**：效期由订单分配决定，补货作业顺序不影响出库效期——PDA 任务列表排序 = **urgent 置顶 → 目标位路径（`pick_sequence_no`）顺路 → 任务号**。
- **整托 vs 拆托**：任务量 ≥ 整托量 80%（参数可配）→ 整托下架（搬托拆箱，`source_lpn_id` 有值，完成后释放容器）；明显小于整托 → 拆托取件（托留原位，任务完成不释放容器）；两模式在手扣减口径一致。

### 5.5 PDA 补货作业流程

**下发与领取规则**：
- **下发**：urgent 任务生成即推送提醒（PDA 红点/声音），普通任务靠列表拉取（推送失败回落列表不阻塞）；
- **领取范围**：按作业员绑定的库区/库位组领取（一次一个任务，完成后再领）；urgent 任务可跨区领取（紧急时任何补货员可接）；同任务同时只允许一个作业员领取（乐观锁 version 校验）；
- **urgent 插队**：列表置顶**不打断当前作业**（打断有货损/找货风险）；10 分钟未领取告警主管，20 分钟未领取自动取消（回冲双字段）+ 通知波次侧缺口未补；
- **作业超时**：领取后 1 小时无进度（`picked_qty` 无变化）告警主管，可强制改派；不自动取消（货可能已在下架途中，自动取消会悬账）；
- **改派与退回**：改派重置 `operator_id`（记流水）；退回回 `pending` + 原因（来源货不对/目标位异常），可重新领取；来源货不对时来源侧触发人工核查。

1. **领取任务**：PDA 补货任务列表（按 §5.4 作业顺序：urgent 置顶 → 目标位路径顺路 → 任务号）领取 → `pending` ➜ `in_progress`，写 `operator_id`。
2. **来源下架**：扫描来源库位/容器码 → 校验与任务 `source_*` 匹配；下架数量（可少于任务量）→ 来源侧不扣在手（已由 `qty_replenish_out_transit` 锁定），任务 `picked_qty += 下架量` 登记中间态（支撑 PDA 断网重连后恢复作业）；`picked_qty > qty` 拒绝（超量下架拦截）。
3. **送达确认**：扫描目标拣选位码 → 校验目标位未被占用/无质量锁/品类温区匹配（复用 6 维校验②③⑥）→ 确认完成。
4. **账面转换（同事务）**：来源位 `qty_on_hand −qty`、`qty_replenish_out_transit −qty`；目标位 `qty_on_hand +qty`、`qty_replenish_in_transit −qty`；任务 `picked_qty −= qty`、`done_qty += qty`；写入库存流水（movement_type = `replenish`）。整托全量补货完成后释放来源容器（`idle`，释放前校验该 LPN 在来源位无剩余在手量）。
5. **重复提交防重**：确认接口按任务 + 幂等键（Idempotency-Key）重放，version 乐观锁兜底。
6. **完成联动（两个异步事件）**：任务 `done` 后——① 发波次重算单事件（订单行"等待补货"重新算单，见 wave-model §6）；② 目标位水位重判（可用量重新计算，仍 < min 且来源有量则由下一轮巡检自然补发，无需任务级联动）。

### 5.6 PC 补货策略与任务大盘

- **策略配置页**：策略 CRUD（Min-Max/触发模式/生效范围）、库位组维护（`replenishment_location_groups` + 成员）、生效范围预览（命中拣选位列表与当前水位）。
- **任务大盘**：
  - 列表：任务号/触发模式/优先级/来源/目标/数量/状态/作业员/时间；按状态、库位、货主过滤；
  - 操作：**取消**（仅 `pending`/`in_progress` 且 `done_qty = 0`，回冲双字段）、**重派**（重置 `operator_id` 释放任务）、**手动发起**（选来源存储位/批次/目标拣选位/数量，生成 `trigger_mode='manual'` 任务）、**超时告警**（`urgent` 任务超时未领取高亮并自动取消回冲）；
  - **波次联动**：波次缺口任务生成时对应订单行标记"等待补货"；任务 `done` 后波次重新算单纳入拣选；任务取消则缺口订单行拆单或降级待下一波次。

### 5.7 并发与一致性

- **来源锁定**：任务生成与下架确认均以 `qty_replenish_out_transit` + 批次行锁为准；同一批次被两个任务同时锁定时，后生成者校验来源可用量不足则失败（等待下一巡检周期），不排队不覆盖。
- **来源位动态占用**：任务生成后来源位被移库/质检隔离（`qty_frozen` 增加）导致可用量不足 → 确认时按 version 乐观锁校验，冲突则任务挂起并告警人工复核。
- **目标位冲突**：确认时校验目标位当前质量状态与容量，不满足 PDA 强阻断（复用 6 维校验）。
- **取消回冲**：取消/超时任务必须回冲双字段，防止在途量悬空（`qty_replenish_in_transit` 残留会导致水位误判不再触发）；取消前校验 `picked_qty = 0`（已下架未送达的货不可随任务取消，需人工处置）。
- **与既有 M3 作业锁互斥**：任务生成与送达确认时，目标位处于盘点锁/养护锁等既有 M3 作业锁状态则跳过或阻断（复用 M3 既有锁语义，不另行造锁）；来源位被移库/盘点占用的处理同 §5.2 可用量口径。
- **幂等与重放**：生成接口按幂等键防重（双字段只 Δ 一次）；确认接口 version 乐观锁防重复确认。

---

## 6. AGV、电子标签与通用硬件设备中台架构 (IoT / WCS Layer)

通过 4 张通用中台表实现任意设备（AGV、PTL、DWS、RFID、立库堆垛机）的插件化接入：

```
1. iot_devices              ── 登记所有物理设备实例 (IP、端口、协议、厂商、状态)
2. location_device_bindings ── 库位与设备点位解耦映射 (绑定角色: ptl_light, rfid_antenna)
3. wcs_tasks                ── 统一下发硬件指令 (搬运货架 pod_move, 亮灯 ptl_light_on, 分拣 sorter_divert)
4. iot_event_logs           ── 接收硬件事件反馈 (拍灭按键、DWS 称重体积数据、RFID 批量扫描 EPC)
```

以下 §6.1-§6.6 给出四表字段级 schema、指令-事件闭环、PTL 拍灯业务流程、AGV 货到人账务联动、设备生命周期与 v1 范围建议。

### 6.1 中台表 schema（字段级设计）

**`iot_devices`（设备主档）**：登记所有物理设备实例；设备为仓库级共享物理资产，不按货主隔离：

```sql
CREATE TABLE iot_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL,                  -- 所属仓库
    device_code VARCHAR(64) NOT NULL,            -- 设备编码（现场唯一，如 AGV-01 / PTL-03 / DWS-01）
    device_type VARCHAR(16) NOT NULL,            -- 设备类型: agv / ptl_light / dws / rfid_antenna / stacker
    vendor VARCHAR(64),                          -- 厂商
    model VARCHAR(64),                           -- 型号
    protocol VARCHAR(16) NOT NULL,               -- 通讯协议: http / tcp / modbus_tcp / mqtt 等
    ip_address VARCHAR(64),                      -- 设备 IP 地址
    port INT,                                    -- 设备端口
    extra_config JSONB NOT NULL DEFAULT '{}',    -- 厂商私有参数（点位偏移、IO 映射等）
    online_status VARCHAR(16) NOT NULL DEFAULT 'offline',  -- 在线状态: online / offline / disabled
    last_heartbeat_at TIMESTAMPTZ,               -- 最近心跳时间（心跳判定在线）
    enabled BOOLEAN NOT NULL DEFAULT TRUE,       -- 启停开关（停用后不再下发新指令）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (warehouse_id, device_code),
    CHECK (device_type IN ('agv', 'ptl_light', 'dws', 'rfid_antenna', 'stacker')),
    CHECK (online_status IN ('online', 'offline', 'disabled')),
    CHECK (port IS NULL OR (port > 0 AND port < 65536))
);

CREATE INDEX IF NOT EXISTS iot_devices_warehouse_type_idx
    ON iot_devices (warehouse_id, device_type);
CREATE INDEX IF NOT EXISTS iot_devices_warehouse_status_idx
    ON iot_devices (warehouse_id, online_status);
```

**`location_device_bindings`（库位-设备点位绑定）**：库位与设备点位解耦映射；**PTL 地址只落本表（`point_address`），不落库位字段**（§1.2）；AGV 车辆本体不按库位绑定（由 RCS 维护），绑定角色仅 `ptl_light` / `rfid_antenna` 两类点位设备：

```sql
CREATE TABLE location_device_bindings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL,                  -- 所属仓库
    location_id UUID NOT NULL,                   -- 库位（AGV 格口或固定库位）
    device_id UUID NOT NULL REFERENCES iot_devices(id),
    binding_role VARCHAR(16) NOT NULL,           -- 绑定角色: ptl_light / rfid_antenna
    point_address VARCHAR(64),                   -- 设备点位地址（如 PTL 灯位地址/地址码；只落本表不落库位字段）
    valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- 绑定生效时间
    valid_to TIMESTAMPTZ,                        -- 绑定有效期截止（NULL = 长期有效/当前生效）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (binding_role IN ('ptl_light', 'rfid_antenna'))
);

CREATE UNIQUE INDEX location_device_bindings_active_uidx
    ON location_device_bindings (location_id, binding_role)
    WHERE valid_to IS NULL;                      -- 同一库位同一角色同时只有一条生效绑定
CREATE INDEX IF NOT EXISTS location_device_bindings_device_idx
    ON location_device_bindings (device_id, valid_to);
```

- **与 `warehouse_locations` 字段的关系**：`is_agv_managed` / `agv_pod_code` 标识"该库位是 AGV 移动货架格口"（货架级属性，随库位档案建），绑定表登记"该库位点位的具体设备实例"（点位级映射）；两者互补不重复——PTL/RFID 设备点位只经绑定表寻址，AGV 车辆调度不经绑定表。
- 解绑采用软解绑（置 `valid_to` 保留历史绑定链），不物理删除；设备故障时绑定失效与降级规则见 §6.5。

**`wcs_tasks`（指令任务）**：统一下发硬件指令；任务随业务事务生成（业务账不动，见 §6.2）：

```sql
CREATE TABLE wcs_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,                      -- 货主（多货主隔离；指令服务于货主业务）
    task_no VARCHAR(64) NOT NULL,                -- 编号（走 M-CG 编号规则）
    task_type VARCHAR(16) NOT NULL,              -- 指令类型: pod_move / ptl_light_on / ptl_light_off / sorter_divert / dws_weigh / rfid_scan
    device_id UUID NOT NULL REFERENCES iot_devices(id),  -- 目标设备（pod_move 为 RCS 网关实例，具体车辆由 RCS 分派）
    location_id UUID,                            -- 关联库位（格口/拣选位；sorter_divert 为 NULL 关联分拣口）
    business_ref_type VARCHAR(32),               -- 业务来源类型（putaway / replenish / pick 等）
    business_ref_no VARCHAR(64),                 -- 业务单号（上架单/补货任务号/波次号）
    payload JSONB NOT NULL DEFAULT '{}',         -- 指令载荷（ptl_light_on: {"qty":5,"color":"green"}；pod_move: {"pod_code":"POD-01","target_station":"ST-01"}）
    status VARCHAR(16) NOT NULL DEFAULT 'pending',  -- 状态机: pending/sent/executing/succeeded/failed/timeout
    ack_payload JSONB DEFAULT '{}',              -- 设备回执载荷（同步回执/终态回执）
    error_code VARCHAR(32),                      -- 失败错误码（设备侧或协议层）
    error_message TEXT,                          -- 失败原因描述
    retry_count INT NOT NULL DEFAULT 0,          -- 已重试次数
    max_retries INT NOT NULL DEFAULT 3,          -- 最大重试次数（超过进入 failed 终态人工介入）
    idempotency_key VARCHAR(128) NOT NULL,       -- 幂等键（业务动作 ID + 指令类型，防重发重复执行）
    sent_at TIMESTAMPTZ,                         -- 下发时间
    finished_at TIMESTAMPTZ,                     -- 终态时间（succeeded / failed）
    created_by TEXT NOT NULL,                    -- 触发来源（业务模块/人工用户名）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 1,           -- 乐观锁（回执/重试并发更新）
    UNIQUE (owner_id, task_no),
    UNIQUE (idempotency_key),
    CHECK (task_type IN ('pod_move', 'ptl_light_on', 'ptl_light_off', 'sorter_divert', 'dws_weigh', 'rfid_scan')),
    CHECK (status IN ('pending', 'sent', 'executing', 'succeeded', 'failed', 'timeout')),
    CHECK (retry_count >= 0 AND retry_count <= max_retries)
);

CREATE INDEX IF NOT EXISTS wcs_tasks_owner_status_idx
    ON wcs_tasks (owner_id, status, updated_at);
CREATE INDEX IF NOT EXISTS wcs_tasks_device_status_idx
    ON wcs_tasks (device_id, status);
CREATE INDEX IF NOT EXISTS wcs_tasks_business_ref_idx
    ON wcs_tasks (owner_id, business_ref_type, business_ref_no);
```

**`iot_event_logs`（硬件事件日志）**：接收设备事件反馈；事件为追加式证据流，只 INSERT 禁止 UPDATE/DELETE（对齐项目审计原则）：

```sql
CREATE TABLE iot_event_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL,                  -- 所属仓库
    device_id UUID NOT NULL REFERENCES iot_devices(id),  -- 上报设备
    event_type VARCHAR(16) NOT NULL,             -- 事件类型: ptl_press / rfid_batch / dws_result / heartbeat
    task_id UUID REFERENCES wcs_tasks(id),       -- 关联指令任务（heartbeat 等无任务事件为 NULL）
    location_id UUID,                            -- 事件点位库位
    payload JSONB NOT NULL DEFAULT '{}',         -- 事件载荷（ptl_press: {"press_qty":5}；rfid_batch: {"epcs":[...]}；dws_result: {"weight_g":3520,"volume_cm3":2200,"pass":true}；heartbeat: {"battery":85}）
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- 设备侧事件时间
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- WMS 接收时间
    CHECK (event_type IN ('ptl_press', 'rfid_batch', 'dws_result', 'heartbeat'))
);

CREATE INDEX IF NOT EXISTS iot_event_logs_device_time_idx
    ON iot_event_logs (device_id, received_at);
CREATE INDEX IF NOT EXISTS iot_event_logs_task_idx
    ON iot_event_logs (task_id);
CREATE INDEX IF NOT EXISTS iot_event_logs_location_time_idx
    ON iot_event_logs (location_id, received_at);
```

### 6.2 指令-事件闭环（指令下发与账务确认）

**双通道模型**：下行指令走 `wcs_tasks`（WMS → 设备/RCS 网关），上行确认走 `iot_event_logs` 异步事件 + `wcs_tasks.ack_payload` 同步回执两种形态；**业务动作只有收到确认回执才落账**，未确认前业务账不动。

**状态机**：

`pending`（业务事务内生成，业务账未动）➜ `sent`（派发器下发至设备/RCS 网关，写 `sent_at`）➜ `executing`（收到开始回执）➜ `succeeded`（收到成功回执且校验通过，账务确认同事务落账）；`failed`（收到失败回执或校验失败）/ `timeout`（超时未收到终态回执）为异常态，未超 `max_retries` 自动重试回 `sent`（`retry_count` +1），重试耗尽进入终态 `failed`（人工介入）。

**回执两种形态**：
- **同步回执**：设备/RCS 网关直接返回（如 `pod_move` 完成/失败）→ 校验后直接推进 `wcs_tasks` 状态；
- **异步事件**：`ptl_press` / `rfid_batch` / `dws_result` 经 `iot_event_logs` 到达 → 事件处理进程按 `task_id` 匹配任务 → 校验载荷 → 同事务推进状态并落账。

**指令 ↔ 事件 ↔ 账务确认对应规则**（三类枚举相互对应，缺一不可）：

| 指令类型 (`wcs_tasks.task_type`) | 确认回执/事件 | 账务确认动作 |
|---|---|---|
| `ptl_light_on` | `ptl_press`（拍灯即确认） | 上架/补货/拣选数量落账（§6.3） |
| `dws_weigh` | `dws_result` | DWS 称重通过才接收上架/复核，重量与预估校验 |
| `rfid_scan` | `rfid_batch` | 读到目标 EPC 集合才确认下架/复核 |
| `pod_move` | RCS 同步回执 | 纯物理位移不落库存账（§6.4），仅推进任务状态与格口可用性 |
| `sorter_divert` | 分拣到位同步回执 | 分拣确认，驱动出库波次进度 |
| `ptl_light_off` | 灭灯同步回执 | 不落账（闭环收尾） |

**超时/失败重试策略**：
- 自动重试上限 `max_retries = 3`，间隔递增退避（1 分钟 / 5 分钟 / 15 分钟）；重试仅允许从 `sent` / `executing` / `timeout` 发起，每次重试 `retry_count` +1 且 `version` 乐观锁递增；
- 重试按 `idempotency_key` 幂等：同业务动作同指令类型只生成一条 `wcs_tasks`，重复下发由任务唯一性 + 设备侧协议幂等字段兜底；
- 重试耗尽 → 终态 `failed` → **人工介入路径**：管理端异常任务大盘（失败任务/错误码/回执载荷）→ 人工可【重发】（重置 `retry_count` 重新入队）/【作废】（仅未落账任务，作废时补偿业务账并记录原因）/【跳过确认】（现场已人工完成，凭证据补录账务，记录操作人）。

**并发与一致性规则**：
- 回执更新用 `version` 乐观锁 + 状态前置校验：仅 `pending` / `sent` / `executing` 可接收回执，终态重复回执幂等忽略；
- 事件→任务匹配按 `task_id`；事件处理进程消费 `iot_event_logs` 需按事件 ID 幂等（重复投递忽略）；
- 账务确认与任务状态推进同事务：落账业务校验失败 → 任务回 `failed` 不回账，人工介入；
- 未关联任务的设备事件（如 `heartbeat`）仅记日志与在线判定，不落账。

### 6.3 PTL 拍灯业务流程细化（货到人 / 拣选拍灯）

适用场景：货到人工作站（AGV 格口上架/补货/拣选拍灯）与普通拣选位亮灯提示。流程：

1. **任务生成**：业务动作（上架、补货送达、拣选）在业务事务内生成 `wcs_tasks`（`task_type='ptl_light_on'`，`payload` 含提示数量与目标库位），业务账不动（补货在途双字段在补货任务生成时已 Δ，亮灯不重复 Δ）；
2. **亮灯下发**：派发器下发 → `sent` → 设备亮绿灯提示待放入数量 → `executing`；
3. **作业员放货**：按灯显示数量将货放入格口/拣选位；
4. **拍灯按键**：放货完成拍灯 → 设备上报 `iot_event_logs` 事件 `ptl_press`（含 `task_id`、拍灯数量、时间）；
5. **账务确认（拍灯即确认）**：事件处理进程校验事件与任务匹配 → **同事务落账**（上架 `qty_on_hand` +Δ；补货目标位在途回冲 + 在手转换；拣选扣减并释放分配）→ `wcs_tasks.status` ➜ `succeeded`；
6. **闭环收尾**：账务确认后下发 `ptl_light_off` 灭灯（或设备自动灭灯），任务终态。

**数量差异处理**：
- 拍灯数量 = 提示数量 → 正常确认；
- 拍灯数量 ≠ 提示数量 → 以拍灯数量为实际确认数量落账（拍灯即确认），同时记录差异并告警人工复核；差异超阈值（±20% 或绝对值 > 10，可配置）强阻断不落账，任务转人工处理；
- 超时未拍灯 → `timeout` → 自动重试亮灯 1 次 → 仍无响应人工介入（人工清点实际数量后凭证据补录）。

**并发与一致性规则**：
- 同一 PTL 设备同一时刻仅一个未终态 `ptl_light_on` 任务（亮灯互斥，防串灯）；
- 同任务重复拍灯事件幂等忽略（已 `succeeded` 不再落账，事件仍记录）；
- 事件到达与落账同事务，落账失败任务回 `failed`；
- 无匹配任务的 `ptl_press` 事件（设备自报/串灯）挂起短窗口等待任务，超窗告警人工处理。

### 6.4 AGV 货到人账务联动

**WMS 与 RCS 对接边界**：AGV 本体（路径规划、避障、充电、车况）由 RCS（Robot Control System）调度管理，**WMS 不直接控制 AGV**；WMS 仅经 `wcs_tasks`（`task_type='pod_move'`，payload 含货架编码与目标工作站）向 RCS 下发搬运需求，RCS 以同步回执反馈搬运结果（完成/失败/超时），WMS 据此推进任务状态与格口可用性。

**格口库位是逻辑位置**：AGV 格口库位的 `row_no` / `layer_no` / `column_no` 对应 POD / F / 格位（§1.2），**账面 location 坐标是逻辑标识，不随货架物理移动变化**；货架当前物理位置（库区/工作站）由 RCS 维护，WMS 侧仅以 `pod_move` 任务状态推断"搬运中 / 已到位"。

**搬运中库存可用性（格口临时不可达标记）**：
- `warehouse_locations` 增加 `agv_unreachable_at TIMESTAMPTZ`（临时不可达标记）：`pod_move` 进入 `executing` 时置当前时间，任务终态（`succeeded` / `failed` / 作废）时清除；
- 不可达期间：该货架全部格口库位禁止出库分配、拣选、上架与移库（**账面在手量不变，仅作业可用性隔离**）；
- 到位（`succeeded`）后标记清除，格口恢复可用（工作站作业）；
- 校验兜底：活跃 `pod_move` 与不可达标记不一致（有任务无标记 / 有标记无任务）时管理端告警；
- 不采用"运行时由活跃 `pod_move` 任务推导"方案：库存可用性不应依赖任务表实时推导（慢查询且易漏判），显式标记更稳。

**账务规则**：
- `pod_move` 纯物理位移，不产生任何库存账变；
- 格口内作业（拍灯 / 人工确认 / RFID）到达账务确认时才落账（§6.2 规则）；
- 搬运中禁止对格口库存做任何账务动作（含移库、报损取样），需待到位后执行。

**并发规则**：
- 同一货架同一时刻仅一个未终态 `pod_move` 任务（一托一搬，防重复调度）；
- 格口作业确认（拣选/上架/补货落账）前置校验不可达标记，不可达强阻断；
- 搬运完成回执与格口作业落账可并发，以后者前置校验为准。

### 6.5 设备生命周期（注册 / 启停 / 心跳 / 离线告警 / 绑定失效）

1. **注册**：设备档案录入 `iot_devices`（仓库、编码、类型、厂商、型号、协议、IP/端口），按库位建立 `location_device_bindings`（角色、点位地址、有效期）；绑定前设备须在线并通过自检；
2. **启停**：`enabled` 开关控制设备参与派发：停用后不再下发新指令、活跃任务停止重试并告警人工介入；历史任务与事件不受影响；
3. **心跳与在线判定**：设备周期上报 `heartbeat` 事件 → `last_heartbeat_at` 更新；超过心跳阈值（默认 90 秒，可配置）未上报 → `online_status='offline'` → 触发离线告警（H4 企微通知 + 管理端告警中心）；设备恢复心跳或人工确认后回 `online`；
4. **离线/故障影响面**：按绑定表反查该设备绑定库位集合，管理端展示受影响库位与待作业任务；
5. **绑定失效与降级人工确认**：
   - 设备离线/故障时绑定不自动删除（保留历史绑定链），作业侧按设备在线状态判定降级；人工解绑才置 `valid_to`；
   - **该库位设备不可用 → 降级人工确认**：PTL 灯不可用 → 该库位拣选/上架降级为人工扫码确认（作业员 PDA 扫码核数，WMS 直接落账并记录"降级人工确认"与原因）；RFID 天线不可用 → 复核/下架降级人工扫码；AGV 不可用 → 不派发 `pod_move`（货到人工作站停用）；
   - 绑定有效期到期（`valid_to` 到达）视同无绑定（走降级路径），运营在管理端续绑；
6. **恢复与换绑**：设备恢复在线并通过自检后，新任务优先走自动确认；解绑/换绑操作需 H1 权限点（`m1.device-bind.manage`，Phase 3 预留）并留审计。

### 6.6 v1 范围建议（Phase 1/2 只建结构与预留，Phase 3 实际接入）

- **Phase 3 为软硬件集成**（RCS / PTL / DWS / RFID 厂商联调、指令-事件闭环、账务联动与人工介入），**v1 前只建表结构与字段预留**，见 ADR-0048 决策 5 "Phase 1 直接落地预留字段与绑定表"；
- **Phase 1**：`is_agv_managed` / `agv_pod_code` 字段与 `location_device_bindings` 绑定表随基础档案建齐（仅建字段与表，不接硬件）；
- **Phase 2（v1 前）**：§6.1 其余三张表（`iot_devices` / `wcs_tasks` / `iot_event_logs`）与 `warehouse_locations.agv_unreachable_at` 列结构建齐（含索引与 CHECK/UNIQUE 约束）：`iot_devices` 仅允许档案登记、`wcs_tasks` 不暴露指令生成入口、`iot_event_logs` 无写入来源、`agv_unreachable_at` 默认 NULL 不启用；
- **Phase 3**：设备注册/绑定管理页面、指令派发器与事件处理进程、心跳/离线告警、重试与人工介入、AGV/RCS 与 PTL 厂商对接、账务联动上线；
- 预留期不产生空转任务与孤儿事件；v1 前直接同步基线，v1 后不再迁库（对齐 ADR-0038 与 ADR-0048 基线同步策略）。

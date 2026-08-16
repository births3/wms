# ADR-0048：药品上架多维度空间属性校验、库位三级作业形态、容器三类质量锁与补货策略

- 状态：Accepted
- 日期：2026-08-16
- 决策者：项目主人、仓储架构组、GSP合规质量组
- 关联：US-M1-006、US-M2-005、US-M2-008、US-M3-009、US-M-VR-002、ADR-0047

## 背景

在大型医药物流中心与多货主 GSP 仓储运营中，存在药品（内服/外用/冷链/特药/饮片/易串味）、医疗器械（一/二/三类/IVD）与非药品（保健品/消杀/危险品/日化）的多品类严格物理隔离要求。同时，周转容器在流转过程中可能发生破损、温控超标或质量待查，需要具备完善的容器质量锁机制；入库整托、B2B 整箱与 B2C 散盒拆零动线并存，未来仓库还将接入 AGV 自动化货架（货到人系统）与电子标签（PTL）。现场需要清晰界定空间属性分工、复用 M1 系统字典的容器质量锁原因维护、独立补货策略体系、库存表在途补货字段以及通用硬件设备中台设计。

## 候选方案

1. **方案 A（仅库区管控与静态状态）**：状态写死在数据库枚举中，加锁原因不可配置。缺点是无法适应多货主与复杂的质量追溯流程。
2. **方案 B（独立新建原因配置表）**：为质量锁单独建一套配置表。缺点是重复造轮子，与 M1 既有系统字典能力割裂。
3. **方案 C（库区定环境大类 + 库位定作业形态 + 容器质量锁复用 M1 系统字典 + 通用 IoT 设备中台，推荐）**：库区定义温区与品类准入大区，库位定义作业类型（存储/箱拣/零拣）；容器支持合格、隔离、不合格三类质量锁，锁原因直接复用 M1 `system_dictionary_items` 系统字典；补货独立配置，硬件插件化接入。

## 决策

1. **空间与属性解耦**：
   - **库区（Zone）**：承载环境温区（常温/阴凉/冷藏/冷冻/超低温）、品类大区准入（`allowed_categories`：药/械/保健品/消杀/危险品/日化）、质量分区（`quality_color`：合格区/隔离区/不合格品区，是容器质量锁移库目标校验依据）与大功能专区；温区枚举 v1 前同步基线为 `normal_10_30 / cool_le_20 / cold_2_8 / freeze_le_minus_20 / ultra_cold_minus_80`（既有 `frozen/cold/cool/normal` 迁移）；
   - **库位（Location）**：承载作业类型（`location_type: storage | case_pick | piece_pick`）、容器开关（`allows_container`）、安全特控（双锁柜、外用专位、易串味专位）、多货主隔离（`current_owner_id`，既有 `bound_owner_id` 改名）、三重锁定（`lock_in` 禁入 / `lock_out` 禁出 / `lock_all` 全锁，既有 `status='locked'` 迁移为 `lock_all`）、混品/混批策略（库位为基座、容器类型策略为收紧层，任一禁止即禁止）与 ABC 动销层级（`pick_zone_level`）。
2. **容器三类质量锁 (Container Quality Locks) 与系统字典复用**：
   - **质量锁三类分类**：
     - `qualified`（合格锁）：正常合格品，允许正常流转、上架、拣选与出库；
     - `quarantine`（隔离锁）：待检、温控异常、退货待查等，强行禁止拣选与出库，仅允许移入隔离库区（目标校验依据 `zone.quality_color = quarantine_yellow`）；
     - `rejected`（不合格锁）：明确破损、过期、召回等，绝对禁止正常流转，仅允许移入物理隔离加锁的不合格品库区（目标校验依据 `zone.quality_color = unqualified_red`）。
   - **锁原因直接复用 M1 系统字典**（分类字段名以既有表为准，为 `dict_code`）：
     - 隔离原因字典：`container_quarantine_reason`（预置温控异常、包装破损待检、销退待验、例行抽样等）；
     - 不合格原因字典：`container_rejected_reason`（预置药品过期、破损泄漏、检验不合格、药监召回等）；
     - 运营人员直接在管理端【系统字典】菜单下维护字典项，无需新造配置表。
   - **触发源仅人工加锁**：加锁/换原因/解锁均为人工操作（管理端容器页 + PDA 扫码操作），不做系统自动加锁；温控超标、召回令等场景由运营人员依据证据（温控记录、召回文件）人工发起。
   - **操作权限**：新增 H1 权限点 `m1.quality-lock.manage`（加锁/换原因/解锁），无权限用户禁止操作；操作日志进审计事件表。
   - **锁事件表为纯审计表（只 INSERT，禁止 UPDATE/DELETE）**：加锁/换原因/解锁均插入新事件行，当前锁冗余在容器主档（`lpn_containers.current_lock_category`）同事务维护；加锁与解锁必须双人见证（`witness_id` 与 `operated_by` 必须为不同用户），缺见证人或见证人重复拒绝提交。
   - **挂接 M-QL 质量联系单**：加锁事件关联 `quality_liaison_orders` 质量联系单（`rejected` 必填、`quarantine` 选填；对齐 `stock_adjustment_orders.quality_liaison_id` 既有先例，`related_document_type = 'container_quality_lock'`）；**解锁前置条件：关联 M-QL 已办结（`closed`），未办结禁止解锁**。
   - **与批次库存质量状态联动**：容器质量锁 `quarantine ⇒ inventory_batches.status='quarantined'`、`rejected ⇒ 'unqualified'`、`qualified ⇒ 'qualified'`（值域对齐既有 M3 `inventory_quality_status` 字典），加锁/换原因/解锁时同步容器下所有库存行并写入库存流水。
   - **加锁前置与解锁回写**：仅 `in_use` 且挂载库存行的容器可加锁（`idle`/`in_transit`/`recycling`/`shipped` 禁止）；解锁只回写仍处于本锁联动状态的批次行，锁期间被质检/召回等其他流程变更过的行不回写、提示人工复核，禁止无条件回写 `qualified`。
3. **容器管理模式**：
   - 存储位（`storage`）：容器化管理（`allows_container = true`，位容一体，按托盘/LPN 追踪）；
   - 箱拣位（`case_pick`）与零拣位（`piece_pick`）：散货管理（`allows_container = false`，上架自动解绑脱离原容器，释放周转箱）。
4. **上架 6 维正交校验流水线（叠加于既有上架事务，非替换）**：
   - PDA 上架或系统推荐库位时，在既有上架逻辑（LPN 绑定、散件互斥、行锁加量，见 ADR-0047）之前执行前置校验：① 品类大区隔离校验 ➜ ② 温区环境匹配校验 ➜ ③ 容器质量锁状态校验（含目标库区 `zone.quality_color` 匹配，散货位读批次 `status`）➜ ④ 特药双人核验（叠加既有 M-VR 双人策略动态查询，见 US-M2-005 验收 8）➜ ⑤ 包装粒度作业形态防呆（存储位仅接受容器粒度上架，散件先组托或转拣选位；拣选位仅接受散货，容器上架自动解绑）➜ ⑥ 外用易串味互斥与容量防呆；③ 通过后进入既有 LPN/互斥/加量事务。
   - **6 维为固定业务骨架**：属 GSP 合规强制校验，硬编码实现，不允许配置关闭或弱化；仅 ④ 的双人策略经 M-VR 规则引擎动态扩展（US-M-VR-002）。
   - 质量锁处于隔离/不合格状态时，严禁上架到正常合格品货位，强阻断报错。
5. **AGV 自动化货架与电子标签（PTL）架构**：
   - 库位编码绑定在移动货架格口上（如 `POD01-F2-03`），库位扩展 `is_agv_managed`、`agv_pod_code`（货架编码）；电子标签（PTL）地址不落库位字段，统一收敛到 `location_device_bindings` 绑定表（绑定角色 `ptl_light`）；
   - **Phase 1 直接落地预留**：`is_agv_managed`/`agv_pod_code` 字段与 `location_device_bindings` 绑定表随基础档案一起建（仅建字段与表，不接硬件），v1 后不再迁库；
   - 支持“货到人”拍灯作业：WMS 下发任务 ➜ RCS 调度 AGV 搬运货架至工作站 ➜ 电子标签亮绿灯提示待放入数量 ➜ 作业员拍灯确认完成账面更新。
6. **库存表原生承载补货在途双字段 (In-Transit Stock on Inventory Table)**：
   - 库存表（`inventory_batches`）直接维护目标侧 `qty_replenish_in_transit`（补货在途量）与来源侧 `qty_replenish_out_transit`（补货下架在途量）；
   - 补货任务生成时**同一事务内**原子增加目标库位 `qty_replenish_in_transit` 并增加来源库位 `qty_replenish_out_transit`（锁定来源在手量，防重复下架）；补货上架确认时双字段同时回冲、在途量原子转为在手量（`qty_on_hand`），保证高并发算单与可用量计算无需聚合任务表；`qty_frozen` 仅承载质检/质量冻结，与补货在途互不混淆。
7. **独立补货策略系统**：
   - 独立菜单维护补货策略（日常安全水位 Min-Max 触发 + 出库波次缺口即时触发双重驱动），按 FEFO 策略从高层存储位下架补货至箱拣/零拣位；
   - **Min-Max 数值只存补货策略表**（按货主 + 库位组/品类配置 `min_safety_threshold` / `max_replenish_target`，支持批量改一组拣选位），库位字段不存数值，仅挂 `replenish_strategy_id` 引用；
   - **补货任务实体**：`replenishment_tasks` 表承载作业过程（任务号走 M-CG、触发模式、优先级、来源/目标、数量、部分执行、状态机 `pending/in_progress/done/cancelled`），任务生成与确认同事务维护在途双字段；PDA 领取→来源下架→送达确认→账面转换五步作业流程；PC 提供策略配置页与任务大盘（取消/重派/超时告警）；并发规则（来源行锁、取消回冲、幂等防重）见模型 §5.4-§5.7。
8. **库位批量生成向导**：
   - PC 端支持按规则向导一键批量生成库位编码（静态高架及 AGV 移动货架格口）并赋默认属性，同时支持标准 Excel 模板导入导出双轨；
   - AGV 格口编码与三坐标对应：`POD[货架号]-F[层]-[格位]` ↔ `row_no`/`layer_no`/`column_no`（如 `POD01-F2-03` = row 01 / layer 2 / column 03）。
9. **分期实施里程碑**：
   - **Phase 1**：基础档案模型、M1 字典挂载容器质量锁原因、批量生成向导、库存表在途量与 6 维正交上架校验基石、容器质量锁（人工加锁 + 权限 + M-QL 挂接）、AGV/PTL 预留字段与设备绑定表；
   - **Phase 2**：PC 独立补货策略与任务大盘、后端双重补货引擎与 PDA 补货作业（任务实体/状态机/并发规则见模型 §5.4-§5.7）；
   - **Phase 3**：AGV 货架调度与 PTL 电子标签驱动软硬件集成。

## 后果

- 基础档案库区、库位与容器表扩展作业类型、容器质量锁、安全特控、容量参数、多货主与 AGV/PTL 字段。
- 质量管理闭环：容器具备可配置的合格/隔离/不合格三级质量锁，锁原因直接通过 M1 系统字典无缝维护，零冗余。
- 核心库存表增加 `qty_replenish_in_transit` / `qty_replenish_out_transit` 双字段，提升高并发下的实时可用量计算效率。
- 新增独立的【补货策略配置】与【补货任务监控】前端管理界面与调度引擎。
- **Phase 1 基线同步清单**（v1 前直接改基线，不做兼容过渡）：① `warehouse_zones.temperature_zone` 与 `products.storage_condition` 两处温区 CHECK 值域同步迁移至五温区新编码；② `warehouse_locations.status` 值域收敛为 `available/occupied/disabled`，存量 `locked` 迁移为 `lock_status='lock_all'`，`bound_owner_id` 改名 `current_owner_id`；③ `inventory_batches` 按 §4 迁移清单改名/新增字段（`product_code→product_id`、`quality_status→status`、`qty_locked→qty_frozen`、新增 `zone_id/container_lpn` 与在途双字段），并同步改 M3 召回/质检作业引用；④ 新增 `container_quality_lock_events` 纯审计表（含 `quality_liaison_id` 挂接列）与 `lpn_containers` 当前锁冗余字段；⑤ 新增补货策略表（含库位组两张表）与 `warehouse_locations.replenish_strategy_id` 引用、`location_device_bindings` 绑定表与 AGV 预留字段。

## 关联与参考

- GSP 药品经营质量管理规范（第八十三条、第八十四条、第八十五条）
- `docs/domain/storage-location-model.md`
- `docs/adr/0047-lpn-container-invariants.md`

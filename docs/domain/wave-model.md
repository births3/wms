# 波次与出库箱领域模型规范 (Wave & Outbound Box Model)

本文档定义 WMS 出库波次的四层领域模型：**波次模板（可配置）→ 波次 → 出库箱（绑定周转箱）→ 拣选任务**，覆盖集单、波次计算、分箱、缺口补货联动、复核、集货、装箱装车与发运全流程。

---

## 1. 波次模板 (Wave Template)

### 1.1 模板层级与类型

- **层级**：货主级模板 + 平台默认模板（继承-覆盖）；订单精确匹配（货主+仓库+单据类型）优先，无匹配回落平台默认模板，不做权重竞争。
- **类型**（模板必选属性，**每类型独立模板**，一个模板只对应一种类型）：
  - `customer`（客户订单，默认）
  - `replenish`（补货）
  - `return_putaway`（退货上架）
  - `difference`（差异处理）
  - `disposal`（处置类：不合格品出库/隔离处置/报损销毁——算单只允许带锁库存，锁-区域约束内作业，与普通波次不混编）
- **GSP 强制常量（不进模板字段，系统锁死）**：同温区合单、同货主合单、带锁库存排除、质量区排除、温区分箱（冷藏/冷冻/超低温各自成箱）。

### 1.2 模板字段清单

| 字段组 | 内容 | 可配 |
|---|---|---|
| 基本信息 | 模板编码/名称、启停、适用范围（货主） | ✓ |
| 波次类型 | customer/replenish/return_putaway/difference/disposal | 必选 |
| 合单条件组 | 组1 客户维度（同客户 OR 同收货地址）；组2 配送维度（同配送路线，可关）；组3 品类维度（同品类组，可关；精麻单独成波=系统强制）——**组间 AND、组内 OR** | ✓（组2/组3 可关） |
| 容量上限 | max_orders=100、max_order_lines=1000、max_units=10000、max_weight_kg=1000、max_volume_cm3=500万、max_pick_tasks=500、max_time_window_h=24（默认值沿用 M4-002） | ✓ |
| 触发方式 | 定时（默认 30 分钟）/ 手动 / 订单阈值（≥N 单）；可组合 | ✓ |
| 订单筛选 | 进入本模板的订单条件：单据类型/时效等级/订单优先级 | ✓ |
| 算单策略 | 分配顺序（库位升序/FEFO/最优路径）；缺口处理（见 §6）；是否允许部分订单失败 | ✓ |
| 库存口径 | 可用量公式（见 storage-location-model §5.2）；排除项默认全开 | ✓ |
| 缺口补货联动 | 是否触发 urgent 补货任务、等待窗口（默认 30 分钟） | ✓ |
| 拣选任务策略 | pick_mode（PIECE_FULL 整件/PIECE_LOOSE 零件）、路径策略（升序/S 型/最短路径）、任务粒度 | ✓ |
| 拣选入箱模式 | `pick_container_mode`：`direct_box`（直装入出库箱）/ `bound_tote`（绑定周转箱）/ `mixed`（按订单属性规则选择） | ✓ 默认 bound_tote |
| 复核模式 | `review_mode`：`box`（按出库箱汇总核对）/ `tote`（按周转箱逐箱核对） | ✓ |
| 集货/装车扫码 | `staging_scan_mode` / `loading_scan_mode`：`mandatory`（逐箱扫）/ `batch`（批量）/ `none`（免扫） | ✓ 默认集货 batch、装车 mandatory |
| 随货同行单维度 | H9 随货同行单集合维度：`order`（默认，按订单）/ `wave`（按波次集合） | ✓ |
| 特殊标记 | 波次优先级默认值（normal/urgent）、精麻/冷链强制单独成波开关 | ✓ |
| 分箱参数 | 箱规字典引用（自定义箱规）；商品装箱参数（allow_box_split / max_units_per_box）见 §3 | ✓ |

### 1.3 变更管控与快照

- **变更直接生效，仅留审计**（不强制审批）；GSP 强制项不在模板字段内，天然不可改。
- **创建即快照**：波次创建时全字段快照（JSONB）存入波次实例，进行中波次不受模板后续变更影响；变更只影响之后创建的波次。

```sql
CREATE TABLE wave_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID,                            -- NULL = 平台默认模板
    template_code VARCHAR(64) NOT NULL,
    template_name VARCHAR(128) NOT NULL,
    wave_type VARCHAR(32) NOT NULL,           -- customer/replenish/return_putaway/difference/disposal
    fields JSONB NOT NULL DEFAULT '{}',       -- §1.2 全部可配字段（含版本语义的字段集合）
    version BIGINT NOT NULL DEFAULT 1,        -- 模板版本（快照引用）
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, template_code)              -- 货主模板：owner_id 非空参与唯一；平台默认模板经下方部分唯一索引约束
);

-- 平台默认模板（owner_id IS NULL）同一 template_code 仅一条
CREATE UNIQUE INDEX IF NOT EXISTS wave_templates_global_code_uidx
    ON wave_templates (template_code) WHERE owner_id IS NULL;
```

---

## 2. 波次 (Outbound Wave)

### 2.1 流程

**集单**（多订单选中/定时自动）→ **产生波次号**（M-CG）→ **波次计算**（算单：库存可用量校验、锁定分配、分箱生成出库箱、预绑周转箱、生成拣选任务）。

- **订单原子性**：一个订单只能属于一个波次（既有库级约束 `UNIQUE(outbound_order_id)`，v1 前保持）；波次内订单行可缺口挂起，但订单不跨波次。
- 波次创建 = 模板快照固化（`template_id` + 全字段 JSONB 快照）。

### 2.2 状态机

```
pending（创建，集单完成）→ allocating（算单中）→ released（已下发，拣选任务可见）
  → picking（拣选中，任务进度聚合）→ reviewing（复核中）→ completed（发运完成）
异常：partial_failed（缺口行挂起，波次继续）/ cancelled（未开始拣选可取消，事务内全量回冲）
```

（既有实现仅 draft/released/cancelled 且 draft 无写路径；本状态机为 v1 前基线同步定稿。）

### 2.3 表结构（既有扩展）

```sql
ALTER TABLE outbound_waves
    ADD COLUMN wave_type VARCHAR(32) NOT NULL DEFAULT 'customer',  -- 模板类型
    ADD COLUMN template_id UUID REFERENCES wave_templates(id),
    ADD COLUMN template_snapshot JSONB NOT NULL DEFAULT '{}',       -- 创建时全字段快照
    ADD COLUMN priority VARCHAR(16) NOT NULL DEFAULT 'normal';      -- normal/urgent
-- 状态值域由代码状态机约束（既有 status TEXT，无 CHECK）
```

---

## 3. 出库箱与周转箱绑定 (Outbound Box & Bound Tote)

### 3.1 出库箱 = 容器（LPN 合一）

出库箱是波次计算的产物，**复用作 `lpn_containers` 的 `outbound_box` 类型容器**（不新建主档表）：**box_no 与 lpn_code 同一取号**（扫箱号 = 扫 LPN，作业对象统一）。

### 3.2 绑定规则

- 出库箱绑定 **1..N 个周转箱**（`tote` 类型容器，可循环复用）；一个周转箱同一时刻只绑定一个出库箱。
- **一条箱明细只能绑定一个周转箱**（`container_lines` 行不跨周转箱；订单行可拆成多个箱明细行，但每个明细行固定在一个周转箱）。
- 预绑定：波次计算时按出库箱预分配周转箱（容器可用池），拣选直接扫已绑定周转箱入货；周转箱不足时可改绑。

### 3.3 出库箱状态机（10 态）

```
generated（波次计算生成）→ [pending_packing（等待分装/绑定，绑定模式）→ packing（分装/绑定中）]
  → ready_to_pick（货齐可拣）→ picking（拣选中）→ picked（已拣）
  → reviewed（已复核）→ staged（已集货）→ shipped（已发运）
异常：exception（复核/装箱发现异常，人工处理）
```

直装模式跳过 `pending_packing/packing` 两态（generated 直接可拣）；绑定模式下两态在分装/绑定完成前保持。

### 3.4 拣选入箱双模式（pick_container_mode）

| 模式 | 作业 | 货齐等待粒度 |
|---|---|---|
| `direct_box` 直装 | 拣选直接入出库箱（明细挂 order_line_id） | **箱级等待**（箱内任一缺口 → 整箱待补货） |
| `bound_tote` 绑定 | 拣选入周转箱（收集多任务）→ 绑定出库箱 → 发货前装箱 | **任务级等待**（缺口行任务标记等待，齐货任务照常拣） |

### 3.5 表结构

```sql
-- lpn_containers 扩展（出库箱专用列，其他容器类型为 NULL）
ALTER TABLE lpn_containers
    ADD COLUMN wave_id UUID,
    ADD COLUMN outbound_order_id UUID,
    ADD COLUMN box_type VARCHAR(32),           -- 箱规字典引用（自定义箱规）
    ADD COLUMN box_status VARCHAR(32),         -- §3.3 出库箱作业状态（仅 outbound_box 类型使用）
    ADD COLUMN bound_box_id UUID REFERENCES lpn_containers(id);  -- 周转箱 → 出库箱绑定（tote 使用）

-- 通用容器明细表（内容物：拣选周转箱/出库箱/保温箱；存储托盘可不用，经 inventory_batches.container_lpn 反查）
CREATE TABLE container_lines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    container_id UUID NOT NULL REFERENCES lpn_containers(id),
    product_id UUID NOT NULL,
    batch_no VARCHAR(64) NOT NULL,
    qty NUMERIC(12, 4) NOT NULL CHECK (qty > 0),
    source_ref_type VARCHAR(32),               -- 来源：pick_task / putaway / packing
    source_ref_no VARCHAR(64),
    order_line_id UUID,                        -- 出库箱场景引用 outbound_order_lines；拣选周转箱可为 NULL
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (container_id, product_id, batch_no, order_line_id, source_ref_no)  -- 同箱同品同批可多行（不同订单行/来源），明细行不跨周转箱由绑定规则保证
);

CREATE INDEX IF NOT EXISTS container_lines_container_idx ON container_lines (container_id);
CREATE INDEX IF NOT EXISTS container_lines_order_line_idx ON container_lines (order_line_id) WHERE order_line_id IS NOT NULL;
```

---

## 4. 拣选任务 (M-TE Pick Task)

- **既有任务实体不变**（行级 + 库位 + `route_sequence` 路径排序，pick_mode 分 PIECE_FULL/PIECE_LOOSE），扩展：

```sql
ALTER TABLE outbound_pick_tasks
    ADD COLUMN box_id UUID REFERENCES lpn_containers(id);  -- 所属出库箱（PDA 按箱聚合显示）
```

- PDA 扫箱号 = 显示箱内任务列表（按 route_sequence 排序）。
- 缺口处理：直装模式箱级等待、绑定模式任务级等待（§3.4）；补货完成（§6）后任务可拣。

---

## 5. 复核 / 集货 / 装箱 / 发运

### 5.1 复核（review_mode 模板字段）

- **按出库箱**：扫箱号 → 聚合绑定周转箱全部明细核对 → 全部核对完成箱置 `reviewed`；
- **按周转箱**：逐箱核对 → 全部绑定周转箱核对完成箱置 `reviewed`；
- 复核发现问题箱置 `exception` 单独处理，不阻塞其他周转箱继续核对。

### 5.2 集货

- **集货区用库位体系承载**：`warehouse_locations` 新增 `location_type='staging'`（集货位），复用 zone/容量/货主能力；
- 集货位号编排：`STG-{wave_no}-{route}-{seq}`（按波次+路线，同车次箱聚相邻位，装车按位序）；
- 复核通过 → 扫箱入集货位（箱 `location_id` 更新为集货位）；支持同波次内换位（记录流水），跨波次禁止；
- 扫码强度：`staging_scan_mode` 参数（默认 batch：按集货位整批流转）。

### 5.3 装箱 + 装车（发货前，装车台合并执行）

- 按绑定关系把周转箱货装入出库箱 → **装箱确认**：`container_lines` 明细转移至出库箱 + 数量校验（转出=转入）+ 周转箱清空回 `idle`（可循环复用）；
- 装箱确认成功 → 出库箱 `box_status='shipped'` 且容器生命周期 `container.status='shipped'`（随货发出，两状态同事务联动，对齐 ADR-0047 容器状态机）→ 直接装车；失败 → `exception` 人工处理；
- 装车核对：`loading_scan_mode`（默认 mandatory 逐箱核对）。

### 5.4 发运与随货同行单

- H9 随货同行单集合维度 = 模板参数（默认按订单，可切按波次集合）；
- 发运时按随货同行单核对装箱清单装车（装箱清单 = 出库箱 + 箱明细）。

---

## 6. 算单与缺口补货联动

- **算单口径**：可用量 = `qty_on_hand − qty_allocated − qty_frozen + qty_replenish_in_transit`（拣选位，见 storage-location-model §5.2）；排除质量区库位、带锁库存、盘点中批次（默认全开，模板可配）；
- **缺口处理（模板字段三策略）**：
  - `wait_and_split`（默认）：缺口订单行标记"等待补货"、出库箱/任务标记待补货，波次继续下发；补货 done 后异步重算单纳入（事件驱动，见 storage-location-model §2.1 异步通知）——**重算单目标：原波次未完成（未进入 completed）则补算回原波次，原波次已完成则并入下一波次或临时补波**；
  - `hold_wave`：波次挂起等待补货（urgent 场景，等待窗口模板可配，默认 30 分钟，超时转 wait_and_split）；
  - `fail_order`：整订单失败回滚（现状语义，可配置为某货主默认）；
- **urgent 补货联动**：波次算单缺口即时触发 `priority='urgent'` 补货任务（storage-location-model §5.1），任务 done 后订单行重新算单。

---

## 7. 与既有基线的差异与迁移（v1 前同步基线）

| 项 | 现状 | 目标 | 说明 |
|---|---|---|---|
| `outbound_waves` | 仅 wave_no/status（draft/released/cancelled，draft 无写路径，无 completed） | 加 wave_type/template_id/template_snapshot/priority，状态机 §2.2 | 状态值域代码约束，无 CHECK 变更 |
| `outbound_pick_tasks` | 行级任务，无箱维度 | 加 `box_id` | 库表加列 |
| 波次逻辑 | 写死（30 分钟定时+同客户合单+整单失败+库位升序） | 波次模板承载（预置平台默认模板复刻现状，行为零漂移） | 新表 wave_templates + 配置 |
| 出库箱 | 无（M4-009 箱规则/M4-011 合并拆单仅故事层） | outbound_box 容器 + container_lines + 绑定 | lpn_containers 加 5 列、新表 container_lines |
| 波次类型 | M4-002 v9 规划 4 种（客户/补货/退货上架/差异，均待补） | 5 种（+处置类），每类型独立模板 | 模板字段 |
| M-VR 波次分组 | M4-002 声明合单规则走 M-VR"波次分组"场景 | **由波次模板承载，M-VR 不再承担波次分组** | 需同步更新 M4 故事与规则引擎范围声明 |
| 随货同行单维度 | H9 按截单集合 | 模板参数（order 默认 / wave） | 参数化 |
| 集货位 | 无 | `location_type='staging'` | warehouse_locations 枚举扩展 |

---

## 关联与参考

- `docs/adr/0048-putaway-storage-zone-attribute-validation.md`（波次缺口补货/带锁感知/重算单异步通知）
- `docs/domain/storage-location-model.md`（可用量公式/补货任务/质量区排除/锁-区域约束）
- `docs/domain/user-stories-m4-outbound-order.md` / `user-stories-m4-outbound-pick.md`（波次规划/拣选/复核故事，待按本文档同步）
- `docs/adr/0047-lpn-container-invariants.md`（容器/LPN 不变量，出库箱复用容器主档）

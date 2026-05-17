# GSP 业务规则字段注册表（Business Rules Registry）

> 时间：2026-05-17
> 版本：v1
> 文档层级：L3 设计（业务模块设计细节）
> 关联：[gsp-field-traceability.md](gsp-field-traceability.md) 字段追溯；[gsp-field-coding-standards.md](gsp-field-coding-standards.md) 编码规范

---

## 1. 目的

医药 WMS 的"专项性"很大程度体现在**业务规则与字段的强耦合**。本注册表把医药行业的关键业务规则（FIFO / 近效期 / 冷链断链 / 追溯码 / 特殊药品）显式登记，规定每条规则用到哪些字段、如何计算、何时触发。

> 与 [gsp-field-traceability.md](gsp-field-traceability.md) 的区别：traceability 矩阵是"GSP 法规 → 字段"的合规追溯；本注册表是"业务规则 → 字段 + 计算 + 触发"的实现说明。

---

## 2. 规则索引

| ID | 规则名 | 涉及字段 | 触发场景 | GSP 关联 |
|----|----|----|----|----|
| BR-1 | FIFO 拣货排序 | 有效期 / 入库时间 / 批号 | 拣货时自动排序 | 7.94 |
| BR-2 | 近效期判定 | 有效期 / 当前日期 | 每日 / 入库时 | 7.100 |
| BR-3 | 冷链断链检测 | 温度 / 时间戳 / 温区上下限 | 实时 / 30 秒采样 | 冷-3, 冷-7, 冷-8 |
| BR-4 | 追溯码反查 | 追溯码 / 批号 / 关联单据 | 监管查询 / 召回 | 追-1 ~ 追-6 |
| BR-5 | 特殊药品双人复核 | special_drug_category / 双人策略命中规则 ID / 第二操作人 | 麻精毒放操作时 | 特-3, 特-4, 特-5 |
| BR-6 | 验收双人签字 | 收货员 / 验收员 / 验收结论 | 收货验收时 | 6.84, 6.87 |
| BR-7 | 召回流程 | 召回标记 / recall_id / 批号 / 客户 | 监管召回令 / 厂家召回 | 不-4, 不-5 |
| BR-8 | 库存状态机 | 库存状态 / 审批源 / 操作人 | 状态变更时 | 7.95, 8.114, 不-1 |

---

## 3. BR-1: FIFO 拣货排序

### 3.1 规则描述

GSP 7.94 要求按"先进先出 + 近效期优先"原则发货。但仅按"先入库"会导致近效期商品堆积，故 WMS 按以下复合排序：

```
ORDER BY
    expire_date ASC,           -- (1) 主排序：有效期早的先出
    created_at  ASC,           -- (2) 次排序：入库时间早的先出（同效期时）
    batch_no    ASC            -- (3) 兜底排序：批号字典序（同效期同入库时间，极端罕见）
```

### 3.2 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `expire_date` | DATE | 主排序键 |
| `created_at` | TIMESTAMPTZ | 入库时间，次排序键 |
| `batch_no` | VARCHAR(20) | 批号，兜底排序键 |
| `qty_available` | NUMERIC(15,3) | 可用数量（>0 才参与拣货）|
| `status` | VARCHAR(20) | 库存状态（必须 = `可用`）|
| `warehouse_id`, `owner_id`, `product_id` | BIGINT | 拣货范围限定 |

### 3.3 SQL 实现

```sql
SELECT inventory_id, batch_no, expire_date, qty_available
FROM inventory
WHERE warehouse_id = $1
  AND owner_id = $2
  AND product_id = $3
  AND status = '可用'
  AND qty_available > 0
ORDER BY expire_date ASC, created_at ASC, batch_no ASC
LIMIT $4;
```

### 3.4 例外情况

- **指定批号拣货**（监管要求 / 客户指定）：忽略 FIFO，按指定批号拣
- **冷链特殊处理**（冷-9）：先出预计冷链时间最长的批次
- **召回中的批号**（recall_flag=true）：跳过，不参与 FIFO

### 3.5 治理验证

- 故事 US-M4-003 拣选必须有此 SQL 模式
- 单测覆盖：3 批次同效期不同入库时间 / 同效期同入库时间不同批号

---

## 4. BR-2: 近效期判定

### 4.1 规则描述

```
近效期 = (有效期 - 当前日期) <= 阈值（默认 30 天，可配置）
```

阈值在 M1-008 配置中心定义：`config.expire_warning_days = 30`（按货主可覆盖）。

### 4.2 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `expire_date` | DATE | 有效期 |
| `expire_warning_days` | INT | 阈值（货主级配置）|
| `is_near_expiry` | BOOLEAN | 计算字段（generated column 或物化视图）|

### 4.3 计算

```sql
-- 方案 A: PostgreSQL generated column
ALTER TABLE inventory ADD COLUMN is_near_expiry BOOLEAN
    GENERATED ALWAYS AS (expire_date - CURRENT_DATE <= 30) STORED;

-- 方案 B: 视图（支持货主级阈值动态变化）
CREATE VIEW v_inventory_with_warning AS
SELECT i.*, c.expire_warning_days,
       (i.expire_date - CURRENT_DATE <= c.expire_warning_days) AS is_near_expiry
FROM inventory i
LEFT JOIN owner_config c ON c.owner_id = i.owner_id;
```

WMS 选方案 B（支持配置级灵活性）。

### 4.4 触发场景

- 每日凌晨 02:00 跑预警 cron → 推送企微（M3-009）
- 入库时即时计算 → 入库批次直接打"近效期"标签
- 拣货时按 FIFO 排序 → 自然把近效期排在前面

### 4.5 例外情况

- 麻精毒放：阈值 60 天（监管要求提前处理）
- 中药饮片：阈值 90 天（销售周期长）
- 配置中心 `expire_warning_days_by_category` 按品类覆盖

---

## 5. BR-3: 冷链断链检测

### 5.1 规则描述

冷链药品在运输或存储过程中，温度连续超出温区上下限超过阈值时间，判定为"断链"。

```
断链条件：连续 N 个采样点（默认 N=3，每点间隔 30 秒）
        且每点温度超出 (temp_min, temp_max) 范围
```

### 5.2 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `temp_celsius` | NUMERIC(5,2) | 当前温度采样值 |
| `recorded_at` | TIMESTAMPTZ | 采样时间（精度到秒）|
| `temp_min`, `temp_max` | NUMERIC(5,2) | 温区上下限（来自冷链区配置）|
| `consecutive_exceed_count` | INT | 连续超标次数（计算字段）|
| `chain_broken_at` | TIMESTAMPTZ | 断链发生时间（断链事件）|

### 5.3 实时检测算法

```rust
fn detect_chain_break(samples: &[TempSample], min: f64, max: f64) -> Option<ChainBreakEvent> {
    let mut consecutive = 0;
    for s in samples.iter().rev() {  // 从最新往前看
        if s.temp < min || s.temp > max {
            consecutive += 1;
            if consecutive >= 3 {
                return Some(ChainBreakEvent {
                    started_at: samples[samples.len() - consecutive].recorded_at,
                    detected_at: s.recorded_at,
                    max_exceed: ...,
                });
            }
        } else {
            break;  // 中断连续性
        }
    }
    None
}
```

### 5.4 触发动作

| 严重度 | 触发条件 | 动作 |
|----|----|----|
| 警告 | 单点超标 | 仅记录温度日志 |
| 一般断链 | 连续 3 点超标 | 推送企微 + 锁定批次（status='加锁'）|
| 严重断链 | 连续 10 点超标 / 超出 ±5℃ | 锁定 + 触发质量联系单（M-QL）+ 通知质管 |

### 5.5 配置项

- `chain_break_threshold_count = 3`（采样点数）
- `chain_break_threshold_minutes = 1.5`（连续 1.5 分钟超标，与 30 秒间隔×3 等价）
- `chain_break_severe_max_delta = 5.0`（严重超标阈值）

---

## 6. BR-4: 追溯码反查

### 6.1 规则描述

GSP 追-6：通过追溯码必须能反向查到完整流转链路（生产 → 入库 → 流转 → 出库 → 销售）。

### 6.2 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `trace_code` | VARCHAR(32) | 追溯码（17/20 位）|
| `batch_no` | VARCHAR(20) | 关联批号 |
| `inventory_id` | BIGINT | 当前所在库存记录 |
| `transaction_id` | BIGINT | 关联流水（每次状态变化产生一条）|
| `parent_trace_code` | VARCHAR(32) | 父级追溯码（如箱码 → 单品码）|

### 6.3 反查 SQL

```sql
WITH RECURSIVE trace_lineage AS (
    SELECT trace_code, parent_trace_code, batch_no, transaction_id, 1 as depth
    FROM trace_code_track
    WHERE trace_code = $1

    UNION ALL

    SELECT t.trace_code, t.parent_trace_code, t.batch_no, t.transaction_id, l.depth + 1
    FROM trace_code_track t
    JOIN trace_lineage l ON t.trace_code = l.parent_trace_code
)
SELECT * FROM trace_lineage ORDER BY depth;
```

### 6.4 反查响应时间

- 监管现场查询：< 3 秒（单码）
- 召回反查：< 30 秒（万级码）

需配合追溯码索引：`CREATE INDEX idx_trace_code ON trace_code_track (trace_code);`

---

## 7. BR-5: 特殊药品双人复核

### 7.1 规则描述

GSP 特-3 / 特-4 / 特-5：麻醉、精神一类、医疗用毒性、放射性药品在收货验收 / 出库 / 销毁等关键操作必须双人复核。

### 7.2 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `special_drug_category` | VARCHAR(20) | 药品特殊管理类别 |
| `dual_person_rule_id` | BIGINT | 命中的双人策略规则 ID |
| `first_operator_user_id` | BIGINT | 第一操作人 |
| `second_operator_user_id` | BIGINT | 第二操作人（≠ first_operator）|
| `dual_person_at` | TIMESTAMPTZ | 双人复核完成时间 |

### 7.3 触发判定

```
IF special_drug_category IN ('麻醉','精神一类','医疗用毒性','放射性')
   AND operation IN ('验收','出库','销毁','库内调拨')
THEN 必须双人复核
```

### 7.4 约束

- `second_operator_user_id != first_operator_user_id`（数据库 CHECK）
- 两人必须均有对应岗位权限（应用层校验）
- 操作时间间隔 ≤ 5 分钟（防止伪造）
- 审计记录：两人姓名 / 工号 / IP / 设备

### 7.5 例外情况

- 系统盘点（`审批源 = 系统盘点`）：盘盈盘亏在阈值内自动调整，不强制双人
- 紧急情况（值班单人）：可跳过但必须 24h 内补签 + 审批

---

## 8. BR-6: 验收双人签字

### 8.1 规则描述

GSP 6.84 / 6.87：所有药品收货验收必须有"收货员 + 验收员"两个角色签字。

### 8.2 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `receiver_user_id` | BIGINT | 收货员（卸货 / 数量核对）|
| `verifier_user_id` | BIGINT | 验收员（质量 / 资质审核）|
| `verify_conclusion` | VARCHAR(20) | 验收结论（合格 / 不合格 / 待复核）|
| `verify_signed_at` | TIMESTAMPTZ | 验收签字时间 |

### 8.3 约束

- `receiver_user_id != verifier_user_id`（不允许自验自收）
- 验收员必须 `has_role('验收岗')`
- 不合格批次：`verify_conclusion = '不合格'` → 自动 `status = '不合格'` → 拒收处理

---

## 9. BR-7: 召回流程

### 9.1 规则描述

监管召回令或厂家召回通知后，必须：
1. 锁定全仓所有命中批号库存
2. 按销售记录反查已售出客户
3. 通知客户回收 / 替换
4. 销毁回收品 + 留档审计

### 9.2 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `recall_flag` | BOOLEAN | 是否被召回 |
| `recall_id` | BIGINT | 召回记录主键 |
| `recall_reason` | VARCHAR(64) | 召回原因（质量问题 / 监管令 / 主动召回）|
| `recall_started_at` | TIMESTAMPTZ | 召回启动时间 |
| `recalled_qty` | NUMERIC(15,3) | 已回收数量 |

### 9.3 触发流程

```
监管令到达 / 厂家通知
    ↓
M-RC-001 创建召回单（含 product_id, batch_no, 召回范围）
    ↓
[锁库存] UPDATE inventory SET status='加锁', recall_flag=true
         WHERE batch_no = $1 AND product_id = $2
    ↓
[反查客户] SELECT customer_id, qty_sold FROM outbound_history WHERE batch_no = $1
    ↓
[通知] 推送客户 / 配送召回通知单
    ↓
[回收销毁] M-SA-001 销毁记录（销毁原因码=召回）
    ↓
[关闭] 召回单 closed_at + 审计追溯链路完整
```

---

## 10. BR-8: 库存状态机

### 10.1 状态枚举

| 状态 | 含义 |
|----|----|
| `可用` | 待销售 / 待出库的合格库存 |
| `预占` | 订单分配占用，未实际扣减 |
| `加锁` | 质量联系单 / 召回 / 养护异常锁定 |
| `不合格` | 验收不合格 / 养护检出问题 |
| `待销毁` | 不合格 + 已批准销毁 |
| `质冻` | 监管要求质量冻结 |
| `已分配` | 已拣货完成，待复核出库 |

### 10.2 状态转换图

```
[新入库] → 可用
可用 ↔ 预占（订单分配 / 取消）
可用 → 加锁（质量联系单 / 召回 / 养护异常）
可用 → 不合格（验收不合格 / 养护异常严重）
加锁 → 可用（解锁 / 复检通过）
加锁 → 不合格（复检不通过）
不合格 → 待销毁（销毁审批通过）
待销毁 → [删除]（销毁完成 + 审计留档）
任何状态 → 质冻（监管令）
预占 → 已分配 → [出库]
```

### 10.3 涉及字段

| 字段 | data_type | 用途 |
|----|----|----|
| `status` | VARCHAR(20) | 当前状态 |
| `previous_status` | VARCHAR(20) | 上一状态（流水账记录）|
| `status_changed_at` | TIMESTAMPTZ | 变更时间 |
| `status_change_reason` | VARCHAR(64) | 变更原因 |
| `approval_source` | VARCHAR(20) | 审批源（必填，详见 check_approval_source_chain.py）|

### 10.4 约束

- 状态变更必须 INSERT 一条 `inventory_transaction` 流水（不能直接 UPDATE 库存）
- `approval_source` 必填（治理脚本强制）
- `审批源 = 验收` 时只能从 `[新建] → 可用 / 不合格`
- `审批源 = 质量联系单` 时可以 `加锁 / 解锁`
- `审批源 = 库存调整审批` 时可以 `不合格 → 待销毁`

---

## 11. 治理脚本

| 脚本 | 校验项 |
|----|----|
| [check_approval_source_chain.py](../../scripts/governance/check_approval_source_chain.py) | 触发库存状态变更的故事必须声明审批源（BR-8）|
| `check_business_rules_registry.py` | （Wave 1 待实现）每条规则在故事中有对应实现引用 |

---

## 12. 与其他文档的关系

```
gsp-business-rules-registry.md（本文档）
  │
  ├─→ gsp-field-traceability.md §6     （字段技术属性数据）
  ├─→ gsp-field-coding-standards.md    （字段编码规范）
  ├─→ docs/domain/user-stories-m3-inventory-query.md      （状态机实现 BR-8）
  ├─→ docs/domain/user-stories-m4-outbound-order.md       （FIFO 实现 BR-1）
  ├─→ docs/domain/user-stories-m5-cold-chain.md     （冷链断链 BR-3）
  ├─→ docs/domain/user-stories-mtc-traceability-code.md  （追溯码反查 BR-4）
  ├─→ docs/domain/user-stories-mql-quality-liaison.md    （召回流程 BR-7）
  └─→ docs/compliance/gsp-special-drugs.md          （特殊药品 BR-5）
```

---

## 13. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-17 | v1 | 初版：8 条核心业务规则（FIFO / 近效期 / 冷链断链 / 追溯码 / 特殊药品 / 验收 / 召回 / 状态机），每条含字段清单 + SQL/Rust 实现 + 触发条件 + 约束 + 例外 |

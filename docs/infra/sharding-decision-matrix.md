# 分库分表决策矩阵（基于真实生产数据）

> 时间：2026-05-17
> 版本：v1
> 数据源：`docs/legacy-analysis/{1,2,3,4,5,6}.csv`（Oracle WMS 生产环境真实数据）
> 文档层级：基础设施层
> 关联：[docs/legacy-analysis/legacy-comparison-matrix.md](../legacy-analysis/legacy-comparison-matrix.md) / [docs/infra/technical-specs.md](technical-specs.md) H10 / [docs/domain/clarifications.md](../domain/clarifications.md) #42

---

## 1. 背景

clarifications #42 初版数据量评估基于估算（3 年 ~151M 行），结论"PostgreSQL 单机足够"。但 v25 后获取业务方现有 Oracle WMS 真实数据后发现：

- 现有单系统已积累 **4.84 亿行**（721 张有数据的表）
- 8 张表 > 10M 行；最大 86M 行（GSP 追溯码追踪）
- 业务方**已实施分区**（PURGE_IDX 系列暗示按 WHSE+时间分区）
- **本项目预估严重低估**：追溯码 9M（预估）vs 86M（实际），~10x 偏差

本文档基于真实数据重新做分库分表决策。

---

## 2. 决策原则

### 2.1 PostgreSQL 性能阈值（经验值）

| 表规模 | 推荐方案 | 性能特征 |
|------|--------|--------|
| < 1M 行 | 单表 + 基础索引 | 任意查询毫秒级 |
| 1M - 10M 行 | 单表 + 复合索引 + 定期 VACUUM | 大部分查询毫秒级 |
| 10M - 100M 行 | **PostgreSQL 原生分区**（声明式分区，按时间或哈希）| 范围查询走分区裁剪；P95 < 100ms |
| 100M - 1B 行 | 分区 + 部分查询路径分库 / 读写分离 | 查询模式优化关键 |
| > 1B 行 | **真分库分表**（Citus / Vitess / 自管 sharding）| 必须按业务键拆 |

### 2.2 分区 vs 分库 vs 分表的判别

```
单表 → 分区（同一物理库，逻辑拆分）→ 分表（多物理库，应用层路由）→ 分库（多服务器，跨库 join 不可行）
```

**本项目目标客户场景**（10 仓 × 5 货主 × 10K SKU × 3000 单/日）：
- 业务方现有单仓系统已达 4.84 亿行，本项目支持 10 仓即 ~5-50 亿行（10 倍放大）
- **PostgreSQL 单机+分区** 是合理上限；**多仓 > 5 时考虑 Citus 扩展**

---

## 3. 表级决策矩阵（基于真实数据 + 分级粒度策略 v1.1）

### 3.0 分级粒度策略

不同表按"写入频率 × 保留期 × 查询模式"分 3 档：

| 档位 | 适用 | 5 年分区数 | 维护频率 |
|------|----|---------|--------|
| 🟦 **按月**（细粒度）| 高频写入 + 短期保留（≤ 30 天） | ≈ 12 / 年 | 每周/月维护 |
| 🟩 **按季度**（推荐默认）| 中频写入 + GSP 长期保留 | ≈ 4 / 年 | 每季维护 |
| 🟨 **按年**（粗粒度）| 低频写入 + 整体规模适中 | ≈ 1 / 年 | 每年维护 |

### 3.1 按月分区（🟦 高频日志类，2 张表）

| 本项目表 | 现有 Oracle 对照 | 现有量 | 保留期 | 分区策略 |
|---------|--------------|------|------|--------|
| `erp_message_log` | MSG_LOG / WMSFE_MSG_LOG | 49M | 7 天 | 范围分区按 created_at 月份；超 7 天分区自动 detach 归档冷存储；用于故障重放 |
| `pix_log` | P_ANALYZE_PIX_LOG | 24M | 7 天 | 同上 |

> 高频写入（每秒/分钟级）+ 短保留：按月足够（30 天 ≤ 1 个分区，归档简单）。

### 3.2 按季度分区（🟩 GSP 长期表，5 张超大表 — 推荐默认）

| 本项目表 | 现有 Oracle 对照 | 现有量 | 5 年预估（10 仓）| 单季度量 | 决策 | 分区策略 |
|---------|--------------|------|---------|--------|------|--------|
| `traceability_code_track` | C_GSP_NBR_TRKG | 86M | ~860M | 43M/季 | 🔴 必须 | 按 created_at 季度（每年 4 个分区）+ 可选 owner_id 子分区 |
| `productivity_track` | PROD_TRKG_TRAN | 86M | ~860M | 43M/季 | 🔴 必须 | 按季度；保留 ≥ 5 年（GSP）|
| `carton_packing_dtl` | SPL_CARTON_DTL_PACKING | 82M | ~820M | 41M/季 | 🔴 必须 | 按季度 |
| `audit_trail` | LABOR_MSG / MSG_LOG 总和 | 64M | ~640M | 32M/季 | 🔴 必须 | 按季度；保留 ≥ 5 年（GSP）；放射性 ≥ 30 年走分级保留 |
| `inventory_transaction` | PIX_TRAN | 23M | ~230M | 12M/季 | 🔴 必须 | 按季度 |

> 中频写入（每分钟级）+ GSP 5 年保留：单季度 32-43M 在 PG 单分区性能优秀区间；查询多按月/季汇总，与分区粒度天然对齐。

### 3.3 按年分区或单表（🟨 低频/中等表，4 类）

| 本项目表 | 现有 Oracle 对照 | 现有量 | 决策 | 策略 |
|---------|--------------|------|------|----|
| `task` | TASK_DTL + TASK_HDR | 5M | 🟡 按年 | 范围分区按 created_at 年份；已完成任务 1 年后归档 |
| `lpn_detail` | LPN_DETAIL + OUTPT_LPN_DETAIL | 12M | 🟡 按年 | 同上；保留近 2 年活跃 |
| `traceability_code_repo` | STAGING_TRAC_CODG_SN | 9M | 🟡 按年 | 范围分区按 created_at 年份（写入低频）|
| `stats_snapshot` | STATS_SNAPSHOT + EVENT | 5M | 🟡 按年 | 同上 |

### 3.4 单表足够的常规表（多数 1-5M 行）

| 表族 | 现有 Oracle 对照 | 决策 | 索引设计 |
|------|--------------|------|--------|
| 出库订单 | ORDER_LINE_ITEM (3.9M) + OUTPT_ORDER_LINE_ITEM (3.8M) | ⚪ **单表 + 索引** | (owner_id, status, created_at) / (customer_id, created_at) |
| 拣货单 | PKT_DTL (3.9M) + PKT_HDR (1.3M) | ⚪ **单表** | (wave_id, status) |
| 库存 | （现存量级 < 5M） | ⚪ **单表** | (owner_id, product_code, batch_no, location_id) |
| 容器 LPN | LPN (5.3M) + OUTPT_LPN (4.3M) | ⚪ **单表** | (status, owner_id) |
| 规则配置 | RULE_HDR (671K) / RULE_SEL_DTL (2.1M) | ⚪ **单表** | (rule_type, enabled) |

### 3.5 完全不分区的小静态表

主数据表（商品 / 客户 / 供应商 / 仓库 / 库位 / 货主 / 用户 / 角色）通常 < 100K 行，单表 + 主键索引足够。

### 3.6 放射性药品 30 年保留特殊处理（v1.1 审计补充）

放射性药品台账保留期 30 年（《放射性药品管理办法》），如果按季度分区会产生 **30 × 4 = 120 个分区/表**，PostgreSQL 单表 100+ 分区会面临元数据缓存压力（query planning 开销 + autovacuum 扫描时间）。

**分级保留策略**：

| 时间窗口 | 分区粒度 | 分区数 | 理由 |
|--------|--------|------|------|
| 0-5 年（活跃期）| 季度 | 20 | 高频查询，与默认 GSP 表对齐 |
| 5-30 年（归档期）| 年 | 25 | 仅审计现场偶发查询，按年足够 |
| 30 年后 | 自动 detach 删除 | — | 法规允许 |

**实施方式**：5 年到期后批量重组旧季度分区为年分区（pg_partman 不直接支持自动重组，需自定义维护脚本，Wave 4+ 实施时落地）。

**触发条件**：业务方启动放射性药品承运业务时（详见 [docs/domain/clarifications.md](../domain/clarifications.md) v25 业务方答复）。如不承运，本节策略不激活。

---

### 3.7 分级粒度的可视总结

```
🟦 按月（2）                  🟩 按季度（5 — 默认推荐）         🟨 按年（4）
─────────────────────────    ──────────────────────────────    ───────────────────
erp_message_log              traceability_code_track            task
pix_log                      productivity_track                 lpn_detail
                             carton_packing_dtl                 traceability_code_repo
                             audit_trail                        stats_snapshot
                             inventory_transaction
─────────────────────────    ──────────────────────────────    ───────────────────
高频写入 + 7 天保留           中频写入 + GSP 5 年保留            低频 + 中等规模
单分区 ~14M                  单分区 32-43M（性能优秀）          单分区 5-12M
```

---

## 4. 分库决策

### 4.1 当前结论：**不分库**

理由：
- PostgreSQL 单机分区可承受 1-10 亿行（按月分区+索引）
- 跨库 join 在 WMS 业务中频繁（库存 ↔ 订单 ↔ 容器 ↔ 任务），分库代价高
- 多仓多货主隔离通过 `(owner_id, warehouse_id)` 字段实现，逻辑隔离即可

### 4.2 触发分库的阈值（监控告警）

满足以下任一条件时考虑启动分库 / Citus 扩展：

| 指标 | 阈值 | 触发动作 |
|------|-----|--------|
| 任一表行数 | > 500M | 评估水平分库或 Citus 分片 |
| PG 单机磁盘 | > 1TB | 同上 + 数据归档加速 |
| 单查询 P95 | > 500ms（关键路径）| 索引优化 → 仍不达标考虑分库 |
| 写入 QPS | > 5000（持续）| 评估读写分离或 Citus |
| 备份耗时 | > 8 小时 | 分库 / 增量备份策略 |
| 仓库数 | > 10 | 评估按 warehouse_id 分库 |

### 4.3 备选方案（达到阈值后）

1. **PostgreSQL 读写分离**（流复制 + 主从）：缓解读压力，写仍单点
2. **Citus 分布式扩展**：按 owner_id / warehouse_id 哈希分片，对应用透明
3. **业务表水平分库**：按业务模块（追溯码 / 库存流水 / 审计追踪）独立物理库
4. **冷热分离**：热数据 PG，冷数据归档到对象存储（S3 兼容）+ 列存（ClickHouse / DuckDB）只读查询

---

## 5. 分区策略详细设计

### 5.1 按季度分区 SQL 示例（GSP 长期表默认）

```sql
-- 主表（声明式分区，按 created_at 季度）
CREATE TABLE audit_trail (
    id BIGSERIAL,
    owner_id BIGINT NOT NULL,
    warehouse_id BIGINT NOT NULL,
    actor_user_id BIGINT,
    action_type VARCHAR(64) NOT NULL,
    object_type VARCHAR(64) NOT NULL,
    object_id BIGINT NOT NULL,
    old_value JSONB,
    new_value JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- 季度子分区（每年 4 个）
CREATE TABLE audit_trail_2026_q1 PARTITION OF audit_trail
    FOR VALUES FROM ('2026-01-01') TO ('2026-04-01');
CREATE TABLE audit_trail_2026_q2 PARTITION OF audit_trail
    FOR VALUES FROM ('2026-04-01') TO ('2026-07-01');
CREATE TABLE audit_trail_2026_q3 PARTITION OF audit_trail
    FOR VALUES FROM ('2026-07-01') TO ('2026-10-01');
CREATE TABLE audit_trail_2026_q4 PARTITION OF audit_trail
    FOR VALUES FROM ('2026-10-01') TO ('2027-01-01');

-- 索引（声明式分区会自动传播）
CREATE INDEX ON audit_trail (owner_id, warehouse_id, created_at);
CREATE INDEX ON audit_trail (object_type, object_id);

-- 5 年后归档：detach 旧季度分区 + 移到归档介质
ALTER TABLE audit_trail DETACH PARTITION audit_trail_2021_q1;
```

### 5.2 按月分区 SQL 示例（高频日志类）

```sql
CREATE TABLE erp_message_log (
    id BIGSERIAL,
    owner_id BIGINT NOT NULL,
    message_type VARCHAR(64) NOT NULL,
    payload JSONB,
    status VARCHAR(32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

CREATE TABLE erp_message_log_2026_01 PARTITION OF erp_message_log
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
-- ...

-- 7 天保留：超过 7 天的分区自动 drop
-- pg_partman 配置 retention = '7 days'
```

### 5.3 按年分区 SQL 示例（低频中等表）

```sql
CREATE TABLE task (
    id BIGSERIAL,
    task_type VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL,
    assignee_user_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

CREATE TABLE task_2026 PARTITION OF task
    FOR VALUES FROM ('2026-01-01') TO ('2027-01-01');
-- 1 年后已完成任务归档
```

### 5.4 自动化分区管理（pg_partman）

> **接口约定**：`pg_partman` v4+ 的 `p_interval` 接受 PostgreSQL interval 字面量（如 `'3 months'`），而非关键字（`'quarterly'` 等不存在）；`p_type` 始终为 `'range'`。

```sql
-- GSP 长期表用季度分区（推荐默认）
SELECT partman.create_parent(
    p_parent_table => 'public.audit_trail',
    p_control => 'created_at',
    p_type => 'range',
    p_interval => '3 months',     -- 季度
    p_premake => 2                -- 提前创建 2 个季度分区
);
UPDATE partman.part_config
SET retention = '5 years', retention_keep_table = true
WHERE parent_table = 'public.audit_trail';

-- 高频日志用月分区
SELECT partman.create_parent(
    p_parent_table => 'public.erp_message_log',
    p_control => 'created_at',
    p_type => 'range',
    p_interval => '1 month',
    p_premake => 3
);
UPDATE partman.part_config
SET retention = '7 days', retention_keep_table = false
WHERE parent_table = 'public.erp_message_log';

-- 低频中等表用年分区
SELECT partman.create_parent(
    p_parent_table => 'public.task',
    p_control => 'created_at',
    p_type => 'range',
    p_interval => '1 year',
    p_premake => 1
);
```

### 5.5 货主级子分区（10+ 货主时启用）

```sql
-- 季度分区 + LIST 子分区按 owner_id（仅在 owner 数 ≥ 10 时启用）
CREATE TABLE traceability_code_track (...) PARTITION BY RANGE (created_at);

CREATE TABLE traceability_code_track_2026_q1 PARTITION OF traceability_code_track
    FOR VALUES FROM ('2026-01-01') TO ('2026-04-01')
    PARTITION BY LIST (owner_id);

CREATE TABLE traceability_code_track_2026_q1_owner_1 PARTITION OF traceability_code_track_2026_q1
    FOR VALUES IN (1);
-- ...
```

---

## 6. 分区键选择规则（v1.1 审计补充）

分区键的选择直接影响分区裁剪能力和报表查询效率。本项目按下表统一约定：

| 表 | 分区键 | 理由 |
|----|------|----|
| `audit_trail` | `created_at` | 审计追踪的"业务时间"等于"系统插入时间"，无歧义 |
| `traceability_code_track` | `created_at` | 追溯码状态变更时间，与查询模式对齐 |
| `productivity_track` | `created_at` | 作业绩效按操作时间汇总 |
| `carton_packing_dtl` | `created_at` | 装箱时间即业务时间 |
| `inventory_transaction` | `effective_date` | **特殊**：库存事务的"生效日期"可能与"插入时间"不同（如倒冲、补单），GSP 报表按 effective_date 汇总；如未来支持，则建立 `(effective_date, created_at)` 复合分区表达式索引 |
| `erp_message_log` / `pix_log` | `created_at` | 日志写入时间即关心时间 |

**统一约定**：

1. **优先 `created_at`**：所有表默认 `created_at` 作为分区键
2. **业务时间 ≠ 系统时间** 时（如库存事务、倒冲、跨日补单）：增加 `effective_date` 字段，分区键改为 `effective_date`，但需保证插入时同时设置 created_at 和 effective_date
3. **不可空**：分区键字段必须 NOT NULL（否则会进入 default 分区，丢失裁剪能力）
4. **不可更新**：分区键字段在记录创建后不应更新（更新会触发 UPDATE 跨分区，性能差且需要特殊配置）
5. **报表查询规约**：所有时间范围查询必须显式提供分区键的 range（如 `WHERE created_at >= '...' AND created_at < '...'`），避免全表扫描

---

## 7. 约束限制（v1.1 审计补充）

PostgreSQL 声明式分区表存在若干约束，与单表设计不同：

### 7.1 全局唯一约束

**限制**：UNIQUE INDEX / PRIMARY KEY 必须**包含分区键**。

```sql
-- ❌ 错误：单列 UNIQUE 在分区表无法直接生效
CREATE TABLE audit_trail (
    id BIGSERIAL,
    created_at TIMESTAMPTZ NOT NULL,
    ...,
    UNIQUE (id)        -- ERROR: must include partition key
) PARTITION BY RANGE (created_at);

-- ✅ 正确方案 1：复合主键（含分区键）
CREATE TABLE audit_trail (
    id BIGSERIAL,
    created_at TIMESTAMPTZ NOT NULL,
    ...,
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- ✅ 正确方案 2：BIGSERIAL 序列保障全局唯一，不创 UNIQUE 约束（信任序列）
CREATE TABLE audit_trail (
    id BIGSERIAL NOT NULL,        -- 序列保障跨分区唯一
    created_at TIMESTAMPTZ NOT NULL,
    ...
) PARTITION BY RANGE (created_at);
-- 业务层用 id 反查时，必须同时提供时间范围（或全表扫描）
```

**本项目约定**：分区表统一用方案 1（`PRIMARY KEY (id, created_at)` 或 `(id, effective_date)`）；既保证全局唯一，又允许分区裁剪。

### 7.2 外键约束

**限制**：分区表可作为外键的引用方（PARENT），但作为 referencing 表（CHILD）的支持有限。

- ✅ `inventory_transaction` 作为引用方（其他表外键到它）：支持
- 🟡 `audit_trail` 作为引用方时：必须包含分区键的复合外键（cost 较高）
- ⚪ **本项目策略**：分区表之间不建外键，业务层逻辑保证关联完整性；非分区表可正常建外键

### 7.3 跨分区 UPDATE

PostgreSQL 11+ 支持跨分区 UPDATE，但分区键字段更新会触发 DELETE+INSERT，性能差。

**本项目约定**：分区键字段（`created_at` / `effective_date`）写入后**禁止更新**；如业务必须修正时间，走"作废 + 重建"路径。

### 7.4 索引传播

PostgreSQL 11+ 声明式分区在主表创建索引时**自动传播到子分区**。

```sql
-- 主表创建索引 → 所有子分区自动有此索引
CREATE INDEX idx_audit_trail_owner_warehouse_created
    ON audit_trail (owner_id, warehouse_id, created_at);
```

新增分区时索引自动创建，无需手工同步。

### 7.5 分区数量上限（性能拐点）

PostgreSQL 单表分区数实践阈值：

| 分区数 | 性能影响 |
|------|--------|
| < 100 | 无明显影响 |
| 100-500 | query planning 时间略增（毫秒级）|
| 500-2000 | planning 显著增加（数十毫秒）|
| > 2000 | 不推荐（autovacuum / pg_dump 时间不可接受）|

**本项目预期**：
- 季度分区表（`audit_trail` 等 5 年×4 = 20 分区）：性能优秀
- 月分区表（`erp_message_log` 7 天保留 ≈ 1-2 分区）：极少
- 年分区表：1 分区/年，无影响
- 放射性药品台账（30 年特殊处理后 ≤ 30 分区）：可控



| Wave | 实施内容 |
|------|--------|
| Wave 1（H 层）| H10 备份策略 + 监控告警接入；不强制分区（小数据量）|
| Wave 2（业务底座）| M1.a / M2 schema 含 `created_at` 字段索引；不分区 |
| Wave 3（核心业务）| M3 库存流水表 schema 设计含分区接口（暂不实际分区） |
| **Wave 4（完整闭环）**| 启用 audit_trail / inventory_transaction / traceability_code_track 三大表的月度分区；接入 pg_partman 自动管理 |
| Wave 5（增值）| M-TE 任务表 / M-PK 装箱明细表分区；评估是否需要 Citus 扩展 |
| 触发分库（可能在 Wave 5 后） | 按 §4.2 阈值监控；达到阈值后 ADR 决策 |

---

## 9. 监控指标（持续评估）

| 指标 | 频率 | 告警阈值 |
|------|-----|--------|
| 各表行数 | 日 | 单表 > 50M 触发评估 |
| PG 数据卷大小 | 日 | > 500GB 提醒；> 1TB 告警 |
| 关键 SQL P95 | 实时 | > 500ms 告警 |
| 写入 QPS | 实时 | > 3000 提醒；> 5000 告警 |
| 备份耗时 | 每次备份 | > 4 小时提醒；> 8 小时告警 |
| 分区维护任务 | 每周 | 失败 → 立即告警 |

---

## 10. 与其他文档的关系

| 文档 | 关系 |
|------|------|
| `docs/legacy-analysis/legacy-comparison-matrix.md` | 本文档的数据基础（现有 Oracle 真实表+行数）|
| `docs/infra/technical-specs.md` H10 数据库备份 | 分区与备份策略联动；按月分区 + 月级备份 |
| `docs/domain/clarifications.md` #42 数据量评估 | 本文档替代原简要评估 |
| `docs/compliance/gsp-ch5-warehouse-management.md` | 分级保留矩阵（5 年 / 30 年）实施依赖分区 |

---

## 11. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-17 | v1 | 初版：基于真实 Oracle 数据（4.84 亿行）的分库分表决策 + 5 张超大表分区策略 + 分库阈值告警 |
| 2026-05-17 | v1.1 | 分级粒度策略：高频日志按月（2 张）/ GSP 长期表按季度（5 张默认）/ 低频中等表按年（4 张）；SQL 示例同步 |
| 2026-05-17 | v1.2 | v1.1 审计修复：(1) pg_partman interval 改为 PG 标准字面量 '3 months'/'1 month'/'1 year' (2) clarifications #42 同步分级策略 (3) 加 §3.6 放射性 30 年特殊处理（5 年内季度 / 5-30 年按年）(4) 加 §6 分区键选择规则（created_at vs effective_date）(5) 加 §7 约束限制（全局唯一 / 外键 / 跨分区 UPDATE / 索引传播 / 分区数上限）|

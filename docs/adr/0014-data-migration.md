# ADR-0014：数据迁移策略（legacy Oracle WMS → wms）

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0001 / ADR-0007 / docs/legacy-analysis/legacy-comparison-matrix.md

---

## 背景

软件设计审计 §4 维度 11 识别数据迁移缺口：

- legacy-analysis/legacy-comparison-matrix.md 已识别现有 Oracle WMS 规模：
  - 2260 张表（721 张有数据）
  - **4.84 亿行**（484M）
  - 8 张 > 10M 行（C_GSP_NBR_TRKG 86M / PROD_TRKG_TRAN 86M / SPL_CARTON_DTL 82M / 其他）
- 已识别"近 1 年迁移 + 旧数据归档"粗略原则
- **但缺**：
  - 详细迁移方案（工具 / 时序 / 切换）
  - 数据校验规则（迁移前后对账）
  - 灰度切换（双写 → 切读 → 关老库）
  - 回滚预案
  - 增量迁移机制（迁移期间老库还在写）

---

## 候选方案

### 方案 A（推荐）：CDC 增量迁移 + 双写灰度切换

阶段：
1. **快照迁移**（Wave 4 末）：导出近 1 年业务流水 + 全量主数据
2. **增量同步**（Wave 4 ~ Wave 5）：CDC（GoldenGate / Debezium）把老库变化实时同步到新库
3. **双写并行**（Wave 5 W5.D）：业务侧同时写老库 + 新库，对账无差异 ≥ 7 天
4. **切读到新库**（按货主灰度，每周 1-2 个货主）
5. **关闭老库写**（最后一个货主切完 + 30 天观察期）
6. **老库只读保留**（5 年，GSP 合规）

### 方案 B：停机一次性切换

**否决**：医药仓 24/7 不能停机；数据量 1 亿+ 行无法在 4 小时窗口内迁完。

### 方案 C：业务方人工录入（不迁老数据）

**否决**：4.84 亿行历史数据是 GSP 5 年审计证据，必须迁移。

---

## 决策

**采用方案 A：CDC 增量 + 双写灰度**。

### 6 阶段时序

| 阶段 | 时点 | 动作 | 持续 | 关键产出 |
|---|---|---|---|---|
| **1. 准备** | Wave 0-3 | 字段对照矩阵 + 迁移工具选型 + 演练环境 | 2 个月 | docs/legacy-analysis 详细 mapping |
| **2. 快照** | Wave 4 启动时 | 全量主数据 + 近 1 年业务流水导入新库 | 1-2 周 | 全量数据已对账 |
| **3. 增量同步** | Wave 4 实施期 | CDC 实时同步 Oracle → PG | 持续 | 延迟 < 5 分钟 |
| **4. 双写并行** | Wave 5 W5.D | API 层双写老/新库 + 异步对账 | 2-4 周 | 差异率 < 0.01% |
| **5. 灰度切读** | Wave 5 W5.D 末 | 按货主切流量到新库 | 每周 1-2 货主 | 全部货主切完 |
| **6. 关老库** | Wave 5 末 + 30 天 | 关闭老库写入；只读保留 5 年 | — | GSP 合规归档 |

### 数据迁移分类（依据 legacy-comparison-matrix §6）

| 数据类型 | 迁移策略 | 量级估算 |
|---|---|---|
| **主数据**（商品/客户/供应商/仓库/库位）| 完整迁移，按 ERP 主数据为准重新对齐 | < 1M 行 |
| **当前库存**（M3 库存表）| 全量迁移 | ~ 5M 行 |
| **库存流水**（PIX_TRAN 等 23M）| 仅近 1 年；归档保留只读 | ~ 30M 行 |
| **追溯码**（C_GSP_NBR_TRKG 86M）| 仅近 1-2 年；旧数据归档 | ~ 30M 行 |
| **订单**（OUTPT_ORDERS / ORDER_* 共 4M）| 近 1 年；按 GSP 5 年保留要求归档 | ~ 4M 行 |
| **消息日志**（MSG_LOG / WMSFE_MSG_LOG）| **不迁移**（保留 7 天即可，新系统从零开始）| 0 |
| **任务表**（TASK_*）| 不迁移（已完成任务无需迁移）| 0 |
| **统计快照**（STATS_*）| 不迁移（新系统重新生成）| 0 |
| **配置表**（RULE_* / CD_MASTER_*）| 完整迁移 | < 100K 行 |
| **归档表**（WMARCH_*）| 不迁移（独立归档介质保留）| 0 |

**预估迁移数据量**：约 **1 亿行**（主数据 + 1 年业务流水 + 当前库存）。

### 工具选型

| 用途 | 工具 | 理由 |
|---|---|---|
| 快照导入 | `sqlx migrate` + Python ETL（pandas + sqlalchemy）| 控制力强，跨 Oracle/PG 兼容 |
| CDC 增量 | Debezium（开源）+ Kafka | 主流，支持 Oracle CDC（LogMiner）|
| 字段对照 | docs/legacy-analysis 已有表 | 对照规则数据驱动 |
| 对账 | Python 脚本 + 抽样比对 + 全量哈希 | 简单可靠 |

### 字段映射规则

来自 legacy-comparison-matrix §5（已存在）：

```
Oracle               wms PostgreSQL
---------------      -----------------
WHSE              →  warehouse_id
TC_COMPANY_ID     →  owner_id
CD_MASTER_ID      →  warehouse_config_id
SKU_ID            →  product_code (M1-001 商品编码)
CNTR_NBR          →  lpn_code
GSP_NBR           →  traceability_code
PKT_CTRL_NBR      →  wave_id / pick_task_id
BATCH_NBR         →  batch_no
STAT_CODE         →  status
I_O_FLAG          →  direction
PROC_STAT_CODE    →  process_status
MOD_DATE_TIME     →  updated_at
CREATE_DATE_TIME  →  created_at
USER_ID           →  user_id
```

> 完整字段对照表（含 153+ 字段，且本 ADR 涉及的 v3.1 P0 新字段如 country_of_origin / USCC 等，需 ERP 提供新数据源）维护在 docs/legacy-analysis/field-mapping.md（Wave 4 启动前补全）。

### 数据校验规则

每张表迁移完成后必须通过以下 4 维校验：

| 维度 | 校验内容 | 工具 |
|---|---|---|
| **行数** | Oracle 行数 == PG 行数（按时间窗口）| `count(*)` SQL |
| **聚合** | 按货主+日期聚合金额/数量 一致 | SUM/COUNT GROUP BY |
| **抽样** | 随机抽 10000 条逐字段比对 | Python 脚本 |
| **业务一致性** | 当前库存余额 = 入库 − 出库 − 调整 | 业务规则 SQL |

任意一维不通过 → 阻塞下一阶段。

### 灰度切换策略

按**货主级灰度**（不是按表 / 按 region），原因：
- wms 是多货主隔离架构，每个货主独立可切
- 切错只影响单一货主，不全局炸库
- 货主签合同同意切换，减少法律风险

灰度顺序：
1. **批次 1**（最小货主，1-2 个）：观察 7 天 → 提交风险报告
2. **批次 2**（中等货主，3-5 个）：观察 14 天
3. **批次 3**（大货主）：分批逐个切

每批切换前**必须**：
- 全量数据已同步（CDC 延迟 < 5 分钟）
- 双写测试通过 7 天
- H-AL 告警阈值收紧（critical 级别错误率告警阈值降到 0.001%）
- 业务方签字确认

### 回滚预案

**触发条件**（任一）：
- 切读后 1 小时内 critical 错误率 > 0.1%
- 数据校验脚本发现差异率 > 0.5%
- 业务方主动要求回滚

**回滚步骤**（< 30 分钟）：
1. API 层路由切回老库（保留 H8 ERP 防腐层桥接）
2. 双写继续保持（不停）
3. 写 H2 审计："货主 X 回滚到 Oracle，原因 Y"
4. 排查根因 → 修复 → 重新切换

**老库写状态**：双写阶段（阶段 4）+ 灰度切读阶段（阶段 5）老库**保持可写**作为回滚兜底。

---

## 后果

### 正面

- **业务无感切换**：除短暂双写性能开销外，业务方不感知迁移
- **GSP 合规**：5 年保留通过老库只读 + 新库归档共同保障
- **可回滚**：任一阶段可回到上一态，最长 RTO 30 分钟
- **风险隔离**：按货主灰度，单一故障不全局影响

### 负面

- **资源开销**：双写期 + CDC 期 DB 资源 ×1.5
- **应对**：双写仅 2-4 周，可接受
- **运维复杂度**：Debezium / Kafka 引入，团队需要培训

### 风险

- **CDC 延迟过高**：> 5 分钟会导致灰度期间数据不一致
- **应对**：CDC 监控（Prometheus）+ 延迟超 1 分钟 H-AL 告警
- **Oracle LogMiner 兼容性**：老 Oracle 版本可能不支持
- **应对**：Wave 4 启动前演练；不行则改用 Oracle GoldenGate（商业）

---

## 实施约束

1. Wave 4 启动前必须完成 docs/legacy-analysis/field-mapping.md 详细字段对照
2. 演练环境必须能复现生产数据 1/10 规模 + 完整 CDC 链路
3. 切换前**必须**业务方签字（每个货主单独签），写 H2 审计
4. 老库 5 年只读保留**不可压缩**（GSP 7.107）
5. 新增字段（v3.1 country_of_origin / USCC 等）从切换日起由 ERP 提供，老数据按"未知"标记
6. 迁移工具的所有操作必须写 H2 审计（actor=`system-migration`）

---

## 参考

- Debezium: https://debezium.io/
- Oracle LogMiner: https://docs.oracle.com/en/database/oracle/oracle-database/19/sutil/oracle-logminer-utility.html
- "Strangler Fig" 渐进迁移模式（Martin Fowler）

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：6 阶段时序 + 工具选型 + 字段对照规则 + 4 维校验 + 货主级灰度 + 回滚预案 |

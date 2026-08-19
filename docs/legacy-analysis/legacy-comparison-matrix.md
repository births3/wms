# 现有 Oracle WMS vs 本项目表对照矩阵

> 时间：2026-05-17
> 版本：v1
> 数据源：Oracle WMS 生产环境导出 CSV（2026-05-16）；原始 CSV 不入 Git，按 `export-scripts.sql` 可重新导出
> 文档层级：L4 评审/对标记录
> 关联：[docs/infra/sharding-decision-matrix.md](../infra/sharding-decision-matrix.md) / [docs/domain/clarifications.md](../domain/clarifications.md) #42

---

## 1. 现有 Oracle WMS 规模总览

| 维度 | 数据 |
|------|-----|
| 总表数 | 2260（含视图、元数据；721 张有数据） |
| 总行数 | 4.84 亿（484M） |
| 超大表（> 10M）| 8 张 |
| 大表（1M-10M）| 30 张 |
| 已归档表 | `WMARCH_*` 系列（业务方已实施归档） |

**核心架构识别**：Manhattan WMOS 派生（`PIX_TRAN` / `PKT_HDR` / `LPN` / `WAVE_QUEUE_PKT` 等是 Manhattan 标准表名）+ 中国本地化（`C_GSP_*` / `Z_WZ_*` / `C_QI_*`）。

---

## 2. 业务模块分组与本项目对照

| 前缀 | 现有表族 | 数据规模 | 本项目映射 | 覆盖状态 |
|------|--------|--------|----------|--------|
| `C_GSP_*` | GSP 追溯码（中国本地化）| 86M + 7.5M 归档 | M-TC 追溯码模块 | ✅ 已覆盖 |
| `PROD_TRKG_*` | 作业绩效跟踪 | 86M | M-TE 任务引擎 + M6-003 | ✅ 已覆盖 |
| `SPL_CARTON_*` | 装箱明细 | 82M | M-PK 包装站 | ✅ 已覆盖 |
| `MSG_*` / `WMSFE_MSG_*` | 系统消息日志 | 28M + 21M + 14M | H4 通知 + H8 ERP 防腐层 | 🟡 已补 US-H8-003，待实现 |
| `P_ANALYZE_PIX_*` | 接口处理日志 | 24M | H8 ERP 防腐层日志 | 🟡 已补 US-H8-003，待实现 |
| `PIX_TRAN` | 库存事务接口（PIX = Perpetual Inventory eXchange）| 23M | M3 库存流水 + H8 反馈 | ✅ 已覆盖 |
| `LABOR_*` | 人工绩效消息 | 14M + 5.4M | M-TE + M6-003 | ✅ 已覆盖 |
| `STAGING_TRAC_CODG_SN` | 药品追溯码暂存 | 9.4M | M-TC + M-PM 待映射队列 | ✅ 已覆盖 |
| `OUTPUT_*` / `OUTPT_*` | 已出货数据 | 8.6M + 5.6M + 3.8M | M4 出库（按状态过滤）| ⚪ 不分独立表 |
| `ALLOC_INVN_DTL` | 库存分配明细 | 6.6M | M4-002 波次锁定 | ✅ 已覆盖 |
| `LPN_*` / `OUTPT_LPN_*` | 容器（License Plate Number）| 6.5M + 5.5M + 5.3M + 4.3M | M1-004a 容器 LPN | ✅ 已覆盖 |
| `ORDER_*` / `OUTPT_ORDER_*` | 订单 + 输出订单 | 3.9M + 3.8M + 3.7M | M4-001 出库订单 | ✅ 已覆盖 |
| `PKT_*` | 拣货单（Pickticket）| 3.9M + 1.3M | M4-002 波次 + M4-003 拣选 | ✅ 已覆盖 |
| `TASK_*` | 任务 | 3.3M + 1.7M | M-TE 任务引擎 | ✅ 已覆盖 |
| `STATS_*` | 统计快照 | 3.6M + 1.8M | M6-003 业务报表 | 🟡 缺快照表故事 |
| `RULE_*` | 规则引擎数据 | 2.1M + 1.1M + 671K | M-VR 规则引擎 | ✅ 已覆盖 |
| `WAVE_QUEUE_*` | 波次队列 | 1.1M | M4-002 波次规划 | ✅ 已覆盖 |
| `Z_WZ_*` | 中国本地化定制（销毁/车辆/打印等）| 各表 0-3.5M | 分散到多模块 | 🟡 看具体表 |
| `C_QI_*` | 质量检验 | 449K | M-QL 质量联系单 + M-DI 药检单 | ✅ 已覆盖 |
| `C_OSR_*` | OSR 穿梭车 / 自动设备 | 1.4M + 587K | ⚪ 本项目暂不集成自动化 | ⚪ 不在范围 |
| `BATCH_MASTER` | 批次主表 | 410K | M3-002 批次效期 | ✅ 已覆盖 |
| `CD_MASTER_*` | 仓主数据（多仓配置）| 608K | M1-009 多仓 | ✅ 已覆盖 |
| `BIN$%` | Oracle 回收站 | — | — | ⚪ 不迁移 |

---

## 3. 关键缺失项（本项目应补的故事）

| # | 现有表族 | 缺失原因 | 建议补充 | 优先级 |
|---|--------|--------|--------|------|
| 1 | `MSG_LOG` / `WMSFE_MSG_LOG` / `LABOR_MSG_DTL` 共 64M 行 | 已由 US-H8-003 补独立故事；消息/尝试存储、月分区、受控保留和故障重放仍待实现 | 按 US-H8-003 实现；未配置保留策略时禁止自动删除 | 🔴 P0 |
| 2 | `P_ANALYZE_PIX_LOG` 24M 行 | 已由 US-H8-003 补故事，接口处理日志实现仍缺 | 同上 | 🔴 P0 |
| 3 | `STATS_SNAPSHOT` / `STATS_SNAPSHOT_EVENT` 共 5.4M 行 | 本项目报表故事（M6-003）按需计算，无快照机制；规模扩大后实时聚合性能差 | **M6-003 加 §统计快照机制**（按日/月预聚合，加速报表查询）| 🟡 P1 |
| 4 | `Z_WZ_DESTRUCTION` 19K 行 | 中国本地化销毁台账，本项目 v25 已加 M-SA 销毁原因码 | M6-004 专用台账已覆盖 | ✅ 已覆盖 |
| 5 | `Z_WZ_CAR_MILEAGE` 等车辆里程 | 自有车队车辆数据 | M10-001 车辆档案已覆盖 | ✅ 已覆盖 |

---

## 4. 关键索引模式参考（5.csv 提取）

> 业务方已用大量索引保障超大表查询性能，本项目 schema 设计应参考。

### 4.1 通用模式：PURGE_IDX_* 系列

8 个超大表均有 `PURGE_IDX_*` 索引：

| 表 | 索引列 | 暗示分区策略 |
|----|------|-----------|
| `PROD_TRKG_TRAN` | `(CD_MASTER_ID, WHSE, MOD_DATE_TIME)` | 多仓按时间分区清理 |
| `PIX_TRAN` | `(TC_COMPANY_ID, WHSE, PROC_STAT_CODE, MOD_DATE_TIME)` | 多公司多仓按时间分区 |
| `MSG_LOG` | `(CD_MASTER_ID, WHSE, MOD_DATE_TIME)` | 同上 |

**业务方已按 `WHSE + 时间` 分区**。本项目对应：`货主 + 月份` 分区（货主隔离对应 WHSE 多仓隔离）。

### 4.2 GSP 追溯码（C_GSP_NBR_TRKG）17 个索引

热点查询维度：
- `(CNTR_NBR, STAT_CODE, I_O_FLAG)` —— 按容器查询
- `(GSP_NBR, STAT_CODE, I_O_FLAG)` —— 按追溯码查询
- `(I_O_FLAG, SKU_ID, BATCH_NBR)` —— 入出库 + 商品 + 批号
- `(PKT_CTRL_NBR, PROC_STAT_CODE, STAT_CODE, I_O_FLAG)` —— 拣货号反查

**本项目 M-TC 追溯码模块 schema 必备索引**：(`追溯码`, `状态`, `入出方向`) / (`容器 LPN`, `状态`) / (`商品编码`, `批号`)

### 4.3 时间字段索引（按时间扫描频繁）

几乎所有超大表都有 `CREATE_DATE_TIME` 或 `MOD_DATE_TIME` 单独索引，说明业务方频繁按时间范围查询（每日/每月报表、归档清理）。

**本项目应统一**：所有业务表 `created_at` / `updated_at` 加索引；分区表以 `created_at` 作分区键。

---

## 5. 字段命名规范化对齐（部分采样）

| 现有 Oracle 字段 | 本项目对应 | 差异 |
|---------------|---------|------|
| `WHSE` | warehouse_id（M1-009 多仓）| 一致 |
| `TC_COMPANY_ID` | owner_id（M1-007 货主）| Oracle 用 company，本项目用 owner |
| `CD_MASTER_ID` | warehouse_config_id（M1 仓主数据）| 概念对齐 |
| `SKU_ID` | product_code（M1-001 商品编码）| 一致 |
| `CNTR_NBR` | lpn_code（M1-004a 容器）| 一致 |
| `GSP_NBR` | traceability_code（M-TC）| 一致 |
| `PKT_CTRL_NBR` | wave_id / pick_task_id（M4-002 波次）| 概念对齐 |
| `BATCH_NBR` | batch_no / 批号（M3-002）| 一致 |
| `STAT_CODE` | status（多模块）| 一致 |
| `I_O_FLAG` | direction（入/出）| 一致 |
| `PROC_STAT_CODE` | process_status / 处理状态 | 一致 |
| `MOD_DATE_TIME` | updated_at | 一致 |
| `CREATE_DATE_TIME` | created_at | 一致 |
| `USER_ID` | user_id（H1）| 一致 |

> 详细字段对照见 docs/compliance/gsp-field-traceability.md。

---

## 6. 数据迁移建议

| 类型 | 迁移策略 |
|------|--------|
| 主数据（商品/客户/供应商/仓库/库位）| 完整迁移，按 ERP 主数据为准重新对齐 |
| 库存当前状态 | 全量迁移（M3 库存表）|
| 库存流水（PIX_TRAN 等）| 仅迁移近 1 年；归档数据保留只读 |
| 追溯码（C_GSP_NBR_TRKG）| 仅迁移最近 1-2 年；旧数据归档 |
| 订单（OUTPT_ORDERS / ORDER_*）| 近 1 年；旧数据按 GSP 5 年保留要求归档 |
| 消息日志（MSG_LOG / WMSFE_MSG_LOG）| 先按受控保留策略和合规要求确认迁移边界；未确认前不得以固定 7 天为由直接清理，新系统运行日志从零开始 |
| 任务表（TASK_*）| 不迁移（已完成任务无需迁移）|
| 统计快照（STATS_*）| 不迁移（新系统重新生成）|
| 配置表（RULE_* / CD_MASTER_*）| 完整迁移 |
| 归档表（WMARCH_*）| 不迁移（独立归档介质保留）|

预估迁移数据量：≈ 1 亿行（主数据 + 1 年业务流水 + 当前库存）

---

## 7. 关键洞见与本项目影响

1. **本项目 clarifications #42 数据量预估严重偏低**（追溯码 9M vs 实际 86M，~10x 低估）；建议按"长期 5-10 年累计"而非"3 年"重估。
2. **业务方已实施归档表**（WMARCH_*），本项目应在 H10 备份策略中加"业务表归档"故事（与备份策略联动）。
3. **多仓多货主分区设计已在业务方落地**（PURGE_IDX 系列），本项目 schema 设计应直接采用 `(owner_id, warehouse_id, created_at)` 复合分区。
4. **消息日志类表是隐藏的数据爆点**（合计 64M 行 + 24M PIX 日志 = 88M 行）；US-H8-003 已补独立故事，完成前必须验证月分区、索引、受控保留和约定数据量 P95。
5. **追溯码索引密度极高**（17 个索引），M-TC 设计阶段需充分考虑查询模式。

---

## 8. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-17 | v1 | 初版：基于 6 csv 综合分析现有 vs 本项目表对照 + 关键缺失项 + 索引参考 + 迁移建议 |

# GSP 字段追溯矩阵（Field-Level RTM）

> 时间：2026-05-17
> 版本：v1
> 文档层级：L4 评审/合规追溯
> 关联：[docs/compliance/README.md](README.md) 条款级 RTM；[scripts/governance/check_gsp_field_traceability.py](../../scripts/governance/check_gsp_field_traceability.py) 自动核对

---

## 1. 目的

**条款级追溯矩阵的细化**：把 GSP 法规明示的字段提取出来，建立"GSP 字段 → WMS 实现位置"的字段级追溯，用于：

1. **合规审计现场反查**：药监检查员问"批号在哪"，直接定位故事 + 字段表行
2. **命名一致性核查**：同一概念字段在不同故事中是否使用同名（如"到货温度" vs "收货温度"）
3. **字段空白识别**：GSP 明示字段无 WMS 字段实现的情况
4. **变更影响分析**：故事字段变更时反查影响哪些 GSP 条款

> 与 [README.md](README.md) 条款级 RTM 的区别：本矩阵粒度=字段，README 粒度=条款。

---

## 2. 状态约定

| 状态 | 含义 |
|------|------|
| ✅ 已实现 | GSP 字段在故事字段表中有对应实现，命名规范 |
| 🟡 命名不一致 | 已实现但同义词在不同故事中混用，建议规范化 |
| ❌ 未实现 | GSP 明示字段无 WMS 字段实现 |
| ⚪ 不适用 | 字段在 WMS 范围外（外部系统主管 / ERP 主管） |

---

## 3. 字段类别索引

| 类别 | 字段数 | 主要 GSP 引用 |
|------|------|-------------|
| 3.1 基础属性 | 11 | GSP 6.83 / 6.87 / 7.93 / 8.111 / 追-2 |
| 3.2 时间字段 | 9 | GSP 6.85 / 6.87 / 7.97 / 8.116 |
| 3.3 数量字段 | 6 | GSP 6.83 / 6.87 / 7.102 / 8.111 / 8.113 |
| 3.4 人员字段 | 7 | GSP 5.66 / 5.72 / 6.84 / 8.112 / 特-3 / 特-5 |
| 3.5 状态字段 | 6 | GSP 6.88 / 7.95 / 8.114 / 8.119 / 不-1 |
| 3.6 资质字段 | 8 | GSP 6.79 / 6.80 / 8.109 / 8.111（USCC）|
| 3.7 冷链字段 | 7 | GSP 6.85 / 7.91 / 8.116 / 冷-3 / 冷-7 / 冷-8 |
| 3.8 追溯字段 | 7 | GSP 5.67 / 8.111 / 追-1 ~ 追-6 |
| 3.9 特殊管理字段 (v24+v25) | 5 | 特-1 ~ 特-9 |
| 3.10 审计字段 | 5 | GSP 5.67 / 5.72 / 5.75 |
| 3.11 养护字段 | 4 | GSP 7.97 / 7.98 / 7.99 / 7.100 |
| 3.12 其他业务字段 | 8 | GSP 6.85 / 7.91 / 8.111 / 8.115 |
| **合计** | **83** | — |

---

## 3.1 基础属性字段（10）

| 字段名 | GSP 条款引用 | WMS 实现位置（故事 + 字段表行）| 命名一致性 | 状态 | 备注 |
|------|-----------|------------------------|---------|------|------|
| 商品编码 | 5.65 / 6.87 / 8.111 | M1-001 商品档案 §1（必填）/ M2-002 收货 §商品编码 / M-DI 药检单 / M6-001 流水账 | ✅ "商品编码"统一 | ✅ | ERP 给定，WMS 不生成 |
| 品名 / 商品名称 | 6.87 / 8.111 / 8.113 | M1-001 §1 / M6-001 流水账 §商品名称 / M6 报表 | 🟡 GSP 用"品名"，WMS 用"商品名称"；建议保留 WMS 名称，glossary 加映射 | 🟡 | 已加 glossary 映射 |
| 规格 | 6.87 / 8.111 / 8.113 | M1-001 §1 / M6-001 流水账 §商品名称 / 规格 / 验收记录 | ✅ "规格"统一 | ✅ | — |
| 剂型 | 7.92 | M1-001 §2 可选字段 / M-PM dosage_form 字典 | ✅ "剂型"统一 | ✅ | M-PM 处理 ERP 不规则文本 |
| 批号 | 5.65 / 6.87 / 7.93 / 7.94 / 8.111 / 8.113 / 追-2 | M1-001（关联）/ M2-002 收货 §批号+数量 / M2-003 验收 §批号 / M3 库存模型核心 / M4-001 §批号 / M4-003 拣选 §批号 / M6-001 流水账 §批号 | ✅ "批号"统一 | ✅ | M3 库存模型按批次粒度 |
| 生产日期 | 6.87 / 7.94 | M1-001 §2 可选 / M2-001 ASN §批号明细 / M2-003 验收核对 / M6-001 流水账 §生产日期 | ✅ "生产日期"统一 | ✅ | — |
| 有效期 | 6.87 / 7.94 / 7.100 / 冷-10 | M1-001 §2 / M2-001 ASN / M2-003 §有效期 / M3-002 效期管理 / M4-003 拣选 §批号 / 效期 / M6-001 §有效期 | ✅ "有效期"统一 | ✅ | FIFO 核心字段 |
| 生产厂家 | 6.87（隐含）| M1-001 §2 可选字段 | ✅ | ✅ | — |
| 批准文号 | 6.87 / 6.81 | M1-001 §2 / M2-003 验收核对 §批准文号 | ✅ "批准文号"统一 | ✅ | — |
| UDI / 电子监管码 | 追-1 ~ 追-3 | M1-001 §2 / M-TC 全模块 / M2-003 验收 §追溯码 / M4-003 §追溯码 | 🟡 "UDI / 电子监管码 / 追溯码"三种命名混用；M-TC 统一为"追溯码" | 🟡 | glossary 加映射 |

---

## 3.2 时间字段（8）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 到货时间 | 6.83 / 6.87 | M2-002 收货 §到货时间（必填）| ✅ "到货时间"统一 | ✅ | — |
| 启运时间 | 6.85 / 冷-4 | M2-002 收货 §启运时间（必填）| ✅ "启运时间"统一 | ✅ | — |
| 收货入库时间 | 6.83 / 6.87 | M2-002 收货 §收货入库时间（必填）| ✅ "收货入库时间"统一 | ✅ | 与"到货时间"区分 |
| 验收时间 | 6.87 | M2-004 双人签字 §签字时间 / M-DI 药检单 | 🟡 GSP 用"验收时间"，WMS 用"签字时间"语义等价；M6-004 用"操作时间" | 🟡 | glossary 加 |
| 上架时间 | 7.92（隐含）| M2-005 上架 §上架时间（系统带出）| ✅ | ✅ | — |
| 出库时间 / 发货时间 | 8.111 / 8.117 | M4-006 §交接时间 / M6-001 流水账 §操作时间 | 🟡 多处用"操作时间"代替 | 🟡 | 语义等价，已通过流水账时间字段反查 |
| 操作时间 | 5.67 / 5.75 | M6-001 流水账 §操作时间（必填）/ 全部审计 | ✅ "操作时间"统一 | ✅ | 跨模块通用 |
| 触发时间 | 5.71 | H4 通知 §触发时间 / M-QL §触发时间 | ✅ | ✅ | — |

---

## 3.3 数量字段（6）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 预报数量 | 6.83 | M2-001 ASN §批号明细 §数量 / M2-002 收货 §预报数量（系统带出）| ✅ | ✅ | — |
| 送货数量 | 6.83 | M2-002 收货 §送货数量（必填）| ✅ | ✅ | — |
| 实际到货数量 / 实到数量 | 6.83 / 6.87 | M2-002 §实际到货数量（必填）/ M-SA-001 §数量 | ✅ "实际到货数量"统一 | ✅ | 收货闭环：实到+缺货+拒收=预报 |
| 缺货数量 | 6.83 / 6.88 | M2-002 §缺货数量（必填，系统计算）| ✅ | ✅ | — |
| 拒收数量 | 6.88 | M2-002 §拒收数量（拒收时必填）| ✅ | ✅ | — |
| 库存数量 / 变动数量 | 7.102 / 8.111 / 8.113 | M3 库存模型核心 / M6-001 §变动数量 (带方向) / M-SA §数量 | ✅ "变动数量"在流水账中统一 | ✅ | M3 + M6-001 双重保障 |

---

## 3.4 人员字段（7）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 收货员 user_id | 6.83 / 特-3 | M2-002 收货 §收货员 user_id（系统带出）| ✅ "收货员 user_id"统一 | ✅ | — |
| 验收员 user_id（双人）| 6.84 / 特-3 / 特-5 | M2-004 §签字人（双人）/ M-DI §第一/第二验收人 | 🟡 GSP 用"验收员"，WMS 故事用"签字人"或"收货员（验收岗）"；建议 glossary 明确 | 🟡 | M2-004 §3 强校验角色 |
| 复核员 user_id（双人）| 8.112 / 8.113 / 特-3 | M4-004 §复核人 user_id / M-PK-002 / 跨约束 §6 双人复核 | ✅ "复核人 user_id"统一 | ✅ | M4 跨约束 §6 强校验拣选≠复核 |
| 拣选员 user_id | 8.113（隐含）| M4-003 §拣选人 user_id（系统带出）| ✅ "拣选人 user_id"统一 | ✅ | — |
| 上架员 user_id | 7.92（隐含）| M2-005 §上架人 user_id（系统带出）| ✅ | ✅ | — |
| 审批人 user_id | 5.66 / 5.72 / 6.79 / 不-2 | M-QL §审批人 user_id / M6-004 §审批人 / M3-003 审批源 | ✅ "审批人 user_id"统一 | ✅ | — |
| 操作人 user_id | 5.67 / 5.72 / 5.75 | M6-001 流水账 §操作人 user_id（必填）/ M6-004 §第一/第二操作人 user_id | ✅ "操作人 user_id"统一 | ✅ | 跨模块通用兜底 |

---

## 3.5 状态字段（6）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 库存状态（合格/隔离/不合格）| 7.95 / 8.114 / 8.119 / 不-1 | M3-003 §状态枚举 / M6-001 流水账 §变动前/后状态 | ✅ "库存状态"枚举统一 | ✅ | 三态枚举 |
| 库存子状态：待销毁 | 不-3 | M3-003 §状态扩展 / M-QL-004 / M-SA-001 销毁原因码 (v25) | ✅ "待销毁"统一 | ✅ | v25 销毁流程 |
| 验收结论（合格/不合格）| 6.87 / 6.88 | M2-003 §质量状态判定 / M2-004 §验收结论 | ✅ "验收结论"统一 | ✅ | — |
| 订单状态 | 6.87（隐含）/ 8.111 | M2 ASN 状态机 / M4 出库状态机 / M-SA 状态机 | 🟡 各模块状态名独立（M2 "已完成" vs M4 "已签收"），但语义清晰 | ✅ | 各模块状态机文档化 |
| 审批状态 / 联系单状态 | 5.72 / 不-2 | M-QL §联系单状态：待审批/审批中/已通过/已拒绝/待ERP同步/已落地 | ✅ "联系单状态"统一 | ✅ | — |
| 召回标记 / 状态 | 不-4 / 不-5 | M3-002 §7 库存批次 `recall_flag` 字段（v25 修复）/ M-QL 召回类型联系单 / M-TC §反查 | ✅ "召回标记 / recall_flag"统一 | ✅ | clarifications #21 决策落地（v25）|

---

## 3.6 资质字段（5）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 供应商资质：营业执照 | 6.79 / 6.80 | M1-002 §2 资质证照（含有效期）| ✅ | ✅ | — |
| 供应商资质：经营许可证 | 6.79 / 6.80 | M1-002 §2 / M-VR-001 §3 校验 | ✅ | ✅ | — |
| 供应商资质：GSP 证 | 6.79 / 6.80 | M1-002 §2 / M-VR-004 §1 模板 | ✅ | ✅ | — |
| 经营范围 | 6.80 / 8.108 | M1-002 §3 / M1-003 客户档案 §3 | ✅ "经营范围"统一 | ✅ / ⚪ | WMS 仅存档；校验由 ERP 做（v7 边界）|
| 客户资质 | 8.109 | M1-003 客户档案 §3 资质证照 | ✅ | ✅ | — |

---

## 3.7 冷链字段（7）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 到货温度 | 6.85 / 冷-4 | M2-002 §到货温度（冷链必填）| ✅ "到货温度"统一 | ✅ | 收货员手填 |
| 运输温度 / 装车温度 | 6.85 / 8.116 / 冷-7 | M4-006 §装车温度（冷链必填）/ M5-002 接收外部温度 | 🟡 "运输温度" vs "装车温度"语义不同：装车=出库前；运输=在途；冷-7 在途由外部冷链系统持有 | 🟡 | 已与外部冷链系统协作 |
| 温控方式 | 6.85 / 冷-1 | M2-002 §温控方式（冷链必填）| ✅ | ✅ | 冷藏车/保温箱/冰袋等 |
| 冷藏车 | 冷-1 / 冷-7 | M2-002 §运输方式 / M10-001 车辆档案 / M-PK-006 | ✅ | ⚪ | 物理设施由企业提供 |
| 保温箱 | 8.116 / 冷-6 | M-PK-004 §保温箱配置（蓄冷剂 + 温区）| ✅ "保温箱"统一 | ✅ | — |
| 蓄冷剂 | 冷-6 | M-PK-004 §蓄冷剂数量 | ✅ | ✅ | — |
| 温度超标事件 | 冷-8 / M-VR §冷链边界 | M5-003 §温度超标事件接收 / M3-003 审批源 = 温度超标事件 | ✅ "温度超标事件"统一 | ✅ | v7 边界：外部冷链系统判定 |

---

## 3.8 追溯字段（5）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 追溯码 | 追-1 ~ 追-6 | M-TC 全模块 / M2-003 §追溯码 / M4-003 §追溯码 / M-PK-005 核销 | ✅ "追溯码"统一 | ✅ | — |
| 电子监管码 | 追-1（GSP 用语）| M-TC §追溯码（同义合并）/ M-PM dosage_form 字典 | 🟡 GSP 法规用"电子监管码"；WMS 统一用"追溯码" | 🟡 | glossary 加映射 |
| 上报记录 / 上报状态 | 追-4 | M-TC-007 §码上放心上报 / 上报记录字段 | ✅ "上报记录"统一 | ✅ | v7 边界：WMS 直接对接 |
| 反查链路 | 追-6 | M-TC-006 §反查接口 / H2-002 审计追踪反查 / M6-001 §反向追溯能力 | ✅ "反查"统一 | ✅ | — |
| 召回记录 | 不-4 / 不-5 | M-QL US-QL-002 §5 召回类型联系单字段（含 recall_id / recall_level / affected_batches / affected_customers / recall_status，v25 修复）/ M3 库存批次 recall_flag / M-TC §反查 | ✅ "召回记录"统一 | ✅ | v25 修复：M-QL 召回类型联系单完整字段定义 |

---

## 3.9 特殊管理字段（5，v24+v25 新增）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| special_drug_category | 特-1 / 特-2 | M1-001 §1 / M1-010 字典 / M-PM 字典 / M6-004 §2 | ✅ "special_drug_category"统一 | ✅ | v24 新增 |
| 双人策略命中规则 ID | 特-3 / 特-4 / 特-5 | M-VR US-VR-006 §6 接口返回 / 各调用方审计追踪 | ✅ "双人策略命中规则 ID"统一 | ✅ | v25 新增 |
| 第二操作人 user_id + 姓名 | 特-3 / 特-5 / 特-9 | M2-004 §2 / M4-003 §10 / M4-004 §6 / M-SA §6 / M-PK-002 §4 / M4-006 §9 / M4-008 §4-§5 / M6-004 §2 必填 | ✅ "第二操作人 user_id" 统一 | ✅ | v25 全节点对齐 |
| 销毁原因 / 销毁原因码 | 不-3 / 特-9 | M-SA-001 §2 / M-SA-001 §6 销毁分支 / M-QL-004 §2 | ✅ "销毁原因码"统一 | ✅ | v25 新增 |
| 专用台账标识 | 特-7 / 特-8 | M1-010 §3 `requires_dedicated_ledger` / M6-004 §1 触发条件 | ✅ "专用台账标识"统一 | ✅ | — |

---

## 3.10 审计字段（5）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 旧值 | 5.67 / 5.72 | H2-002 §审计追踪 §旧值 / M6-002 §显示旧值 | ✅ "旧值"统一 | ✅ | — |
| 新值 | 5.67 / 5.72 | H2-002 §新值 / M6-002 §显示新值 | ✅ | ✅ | — |
| IP 地址 | 5.67（隐含）| H2-002 §IP 地址 / M6-002 | ✅ "IP 地址"统一 | ✅ | — |
| 关联单据 / 关联 ASN | 5.65 / 5.67 | M6-001 §关联单据类型 + 单据号 / 各 PDA 字段表 §关联 ASN | ✅ "关联单据"统一 | ✅ | — |
| 审批源 / approval_source | 5.72 / 不-2 / 特-9 | M3-003 §审批源枚举 7 类 / M6-001 §审批源类型 + ID / M-SA §7 / M-QL §6 | ✅ "审批源"统一 | ✅ | 跨模块通用 |

---

## 3.11 养护字段（4）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 养护频率 | 7.97 | M3-004 §养护频率（默认每月 1 次，可配置）| ✅ | ✅ | M1-008 配置 |
| 养护异常 | 7.99 | M3-004 §异常 → 触发 M-QL 联系单 / M3-003 审批源 = 养护异常 | ✅ "养护异常"统一 | ✅ | — |
| 温湿度记录 | 7.98 / 冷-3 | M3-004 §温湿度记录 / M5-002 接收外部冷链系统数据 | 🟡 "温湿度记录" vs "温度记录"在不同条款混用 | 🟡 | v7 边界：外部冷链系统主管 |
| 近效期预警 | 7.100 | M3-002 §近效期预警 / M3-009 §近效期 / M6-003 §近效期预警报表 | ✅ "近效期预警"统一 | ✅ | M1-008 配置预警天数 |

---

## 3.12 其他业务字段（5）

| 字段名 | GSP 条款引用 | WMS 实现位置 | 命名一致性 | 状态 | 备注 |
|------|-----------|----------|---------|------|------|
| 库位 | 7.92 / 7.95 / 7.96 | M1-004 §库位（编码规则）/ M2-005 §实际库位（必填）/ M3 库存模型 §库位 / M-SA §库位 | ✅ "库位"统一 | ✅ | — |
| 货主 | M1-007 多货主 / 8.115 抬头 | M1-007 §1 / 全部业务故事跨约束 §多货主 | ✅ "货主"统一 | ✅ | — |
| 运输方式 | 6.85 / 8.116 | M2-002 §运输方式（冷藏车/普通车/快递）| ✅ "运输方式"统一 | ✅ | — |
| 承运商 | 6.83 / 6.85 | M2-002 §承运商（必填）| ✅ "承运商"统一 | ✅ | — |
| 随货同行单 | 8.115 | M4-005 §随货同行单 / M-PK §装箱 / 打印模板 H9 | ✅ "随货同行单"统一 | ✅ | 多货主抬头 |

---

## 3.13 字段性质类别索引（v3 加入，2026-05-17）

按 `field_class`（字段性质）维度索引全部 153 字段。与 §3.1-§3.12 业务领域分类（按字段表达的业务概念）正交。

### 3.13.1 🔴 GSP 字段（65 项 + 5 audit = 70 GSP 强制）

GSP 法规明示要求的字段。详见 §3.1-§3.12 业务领域分类。

### 3.13.2 🟢 业务字段（40 项）— 业务必需，GSP 未明示

| 类别 | 数量 | 字段 |
|----|----|----|
| 单据号 | 9 | ASN 号 / 出库单号 / 拣货单号 / 复核单号 / 装车单号 / 任务号 / 调拨单号 / 销毁单号 / 盘点单号 |
| 业务实体 | 8 | 商品 ID / 仓库 ID / 库区编码 / 库位类型 / 客户 ID / 客户编码 / 供应商 ID / 供应商编码 |
| 任务/状态 | 3 | 任务类型 / 任务状态 / 任务优先级 |
| 业务数量 | 5 | 已分配数量 / 已拣货数量 / 已复核数量 / 可用数量 / 预占数量 |
| 物流 | 4 | 通道号 / 月台号 / 配送地址 / 收件人电话 |
| 计费 | 3 | 应收金额 / 计费数量 / 月结账单号 |
| 标识 | 3 | 整箱出库标识 / 越库标识 / 零拣标识 |
| 容器 | 3 | LPN 号 / 周转箱编码 / 整托数量 |
| 其他 | 2 | 状态变更原因 / 业务备注 |

### 3.13.3 ⚪ 系统元数据（8 项）

`id` / `created_at` / `updated_at` / `created_by` / `updated_by` / `version` / `tenant_id` / `deleted_at`

### 3.13.4 🟡 配置字段（15 项，全部归 M1-008 配置中心）

近效期 / 冷链断链 / 温度采样 / FIFO / 双人 / 审计保留 / 召回模板 / 冷链温区 / 库存预占 / 拣货策略 / 补货 / 审计导出 / 默认仓 (15 项详见 §6.3 yaml `field_class: config`)。

### 3.13.5 🟣 计算字段（10 项）

`is_near_expiry` / `days_to_expire` / `age_days` / `is_recalled` / `is_pickable` / `consecutive_exceed_count` / `picking_priority` / `requires_special_handling` / `inventory_age_seconds` / `can_self_destroy`

### 3.13.6 ⚫ 接口字段（10 项）

`external_ref` / `source_system` / `sync_status` / `sync_at` / `sync_retry_count` / `erp_doc_no` / `tms_shipment_id` / `wechat_msg_id` / `regulatory_report_id` / `third_party_carrier_no`

---

## 4. 状态总览

### 4.1 v3 性质类别分布（163 字段）

| 性质类别 | 数量 | 故事覆盖状态 | 治理动作 |
|----|----|----|----|
| 🔴 GSP（gsp + audit）| 80 | ✅ 70/80 闭环；10 项 v25 backlog（unimplemented）| T1 强制（error），unimplemented 降级 info |
| 🟢 业务字段（business）| 40 | 🟡 ~13/40 已在故事，27 待补 | T1 警告（warning，Wave 1 backlog）|
| ⚪ 系统元数据（system）| 8 | ⚪ 不强制故事提及 | 自动管理 |
| 🟡 配置字段（config）| 15 | 🟡 0/15 待 M1-008 故事补 | T1 警告（warning，Wave 1 backlog）|
| 🟣 计算字段（derived）| 10 | ⚪ 不强制故事提及 | 公式可重现 |
| ⚫ 接口字段（interface）| 10 | 🟡 0/10 待防腐层故事补 | T1 警告（warning，Wave 1 backlog）|
| **合计** | **163** | | |

### 4.2 v2 GSP 字段命名一致性（70 字段，不变）

| 状态 | 数量 | 占比 |
|------|------|------|
| ✅ 已实现且命名规范 | 47 | 67% |
| 🟢 已实现 + acceptable_alias（多 alias 合理）| 22 | 31% |
| ❌ 未实现 | 0 | 0% |
| ⚪ 不适用（外部系统/ERP 主管）| 1 | 2% |
| **合计** | **70** | **100%** |

**v3 结论**：
- GSP 强制字段 70/70 闭环（治理 T1 强制，error 级）
- business + config + interface 共 65 项中 12 项已在故事中实现，53 项待 Wave 1 补全（治理 warning 级，列入 Wave 1 backlog）
- system + derived 共 18 项不强制故事提及（系统自动管理）
- 治理脚本运行：0 error / 53 warning / 5 info — T1 通过

---

## 5. 命名一致性问题处置（22 项 acceptable_alias）

> v25 修复后所有命名混用已通过 `acceptable_alias` 状态明确为合理（不再是问题）。本节归档分析结论，便于 v26 评估是否启动代码层规范化。

### 5.1 细化语义类（14 项）— 多 alias 表达不同业务语义，必须保留

| canonical | aliases | 语义区分 |
|----------|---------|---------|
| 库位 | 库位 / 实际库位 / 推荐库位 / 目标库位 / 库位编码 | PDA 字段细化（系统推荐 vs 实际选择） |
| 库存状态 | 库存状态 / 变动前状态 / 变动后状态 | 流水账时点字段 |
| 关联单据 | 关联单据 / 关联 ASN / 关联 ASN 号 | 通用 vs 入库专用 |
| 操作时间 | 操作时间 / 出库时间 / 发货时间 | 流水账"操作时间 + 操作类型"组合表达细分时点 |
| 装车温度 | 装车温度 / 运输温度 / 在途温度 | 出库装车 vs 在途运输两段管理 |
| 反查链路 | 反查 / 反向追溯 | 接口/PDA 语境 vs 合规/文档语境 |
| 近效期预警 | 近效期 / 近效期预警 | 状态标签 vs 触发动作 |
| 变动数量 | 变动数量 / 库存数量 | 流水账字段 vs 业务术语 |

### 5.2 中英文双语类（4 项）— code/字段名 vs 中文显示，并行合理

| canonical | aliases |
|----------|---------|
| special_drug_category | special_drug_category / 特殊药品分类 |
| 审批源 | 审批源 / approval_source |
| 召回标记 | 召回标记 / recall_flag |
| 召回记录 | 召回记录 / recall_id |

### 5.3 GSP 法规简写类（4 项）— GSP 法规原文用语 vs WMS 内部命名

| canonical | aliases | 处置 |
|----------|---------|------|
| 商品名称 | 商品名称 / 品名 | GSP 法规打印模板使用"品名"，glossary §GSP 映射已对齐 |
| 有效期 | 有效期 / 效期 | "效期"是简写，多用于复合字段（"批号/效期"），glossary 已映射 |
| 生产厂家 | 生产厂家 / 厂家 | "厂家"是简写，正文使用合理 |
| 追溯码 | 追溯码 / UDI / 电子监管码 | GSP 法规多种用语，"追溯码"是父概念，glossary §5/§6 已定义 |

### 5.4 角色 / 状态等价命名（5 项）— 不同模块独立命名但语义等价

| canonical | aliases | 等价说明 |
|----------|---------|---------|
| 实际到货数量 | 实际到货数量 / 实到数量 | "实到数量"是简写，正文中可能简写 |
| 验收员 user_id | 验收员 user_id / 签字人 / 收货员（验收岗）| GSP 用通用称谓，WMS 用细化角色名 |
| 验收结论 | 验收结论 / 质量状态 | M2-003 字段名 vs GSP/业务用语 |
| 订单状态 | 订单状态 / ASN 状态 | M2 入库 vs M4 出库，状态机文档化合理 |
| 上报记录 | 上报记录 / 上报状态 | "记录"是数据，"状态"是字段值（成功/失败/重试中）|
| 销毁原因码 | 销毁原因码 / 销毁原因 | M-SA-001 字段名（v25）vs 简写 |
| 温湿度记录 | 温湿度记录 / 温度记录 | GSP 不同条款混用，温度是温湿度的子集 |

---

## 6. 字段词典（治理脚本输入）

> 本节为机器可读的字段词典，用于 `scripts/governance/check_gsp_field_traceability.py` 自动核对。
> 修改本节须同步更新脚本字段词典常量。

**v3 字段性质类别扩展（2026-05-17）**：从 v2 的 70 GSP 字段扩展到 153 字段，覆盖**医药 WMS 全部字段性质类别**。每个字段含 `field_class`（性质，7 类）+ `category`（业务领域，12 类）双维度分类。

### 6.1 字段性质类别（field_class）

| 类别 | 含义 | 数量 | 治理重点 |
|----|----|----|----|
| 🔴 `gsp` | GSP 法规明示要求的字段 | 75 | 5 年保留 + 不可篡改 + 必审计 + 故事字段表必须出现 |
| 🔵 `audit` | 系统级追溯字段（GSP 5.67/5.72 明示，但属性偏系统）| 5 | append-only / 自动填充 / 必审计 |
| 🟢 `business` | 业务流程必需但 GSP 未明示 | 40 | 业务规则校验 + 故事中至少出现一次 |
| ⚪ `system` | 技术性必需（id/created_at/version 等）| 8 | 自动管理，开发者勿改 |
| 🟡 `config` | M1-008 配置中心可调整 | 15 | 双向一致性校验（故事使用 ⇄ 配置中心 ⇄ 默认值）|
| 🟣 `derived` | 系统派生（generated column 或运行时计算）| 10 | 公式可重现 + 不持久化或 GENERATED |
| ⚫ `interface` | 跨系统集成（external_ref / source_system）| 10 | 防腐层归口 |

**总计**：163 字段（80 GSP 强制 = 75 gsp + 5 audit）。

### 6.2 治理脚本对 field_class 的处理

| field_class | 校验规则 |
|----|----|
| `gsp` / `audit` | 必须在故事字段表中实现（70/70 闭环）|
| `business` | 至少在某故事字段表或正文中出现一次（≥1 引用即可）|
| `system` / `derived` | 不强制故事提及（系统层自动管理）|
| `config` | 必须在 M1-008 配置中心故事中出现 + 故事使用方引用 |
| `interface` | 必须在对应防腐层故事（H8 ERP / M11 监管 / H4 企微 / M10 TMS+ / H5 快递）中出现 |

### 6.3 字段词典 YAML

```yaml
field_dictionary:
  # 🔴 GSP 字段（法规明示要求，70 项 — audit 类合并 5 项） — 65 项
  - canonical: 商品编码
    aliases: [商品编码, 品名编码]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [5.65, 6.87, 8.111]
    data_type: VARCHAR(32)
    validation: '^[A-Z0-9]{6,32}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'P-2026-001'

  - canonical: 商品名称
    aliases: [商品名称, 品名, 商品描述, DESCRIPTION, SKU_DESC]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [6.87, 8.111, 8.113]
    wms_status: acceptable_alias   # 法规打印模板使用"品名"，内部统一"商品名称"；glossary 已映射
    data_type: VARCHAR(128)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '盐酸左氧氟沙星片'

  - canonical: 规格
    aliases: [规格]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [6.87, 8.111, 8.113]
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '0.5g*12片*盒'

  - canonical: 剂型
    aliases: [剂型]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [7.92]
    data_type: VARCHAR(32)
    validation: 'from_dict(剂型字典)'
    nullable: false
    encryption: none
    audit_required: true
    example: '片剂'

  - canonical: 批号
    aliases: [批号, 批次号, lot, BATCH_NBR]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [5.65, 6.87, 7.93, 7.94, 8.111, 8.113, 追-2]
    data_type: VARCHAR(20)
    validation: '^[A-Za-z0-9]{1,20}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'BN20260517001'

  - canonical: 生产日期
    aliases: [生产日期]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [6.87, 7.94]
    data_type: DATE
    validation: '<=今天 AND >=有效期-保质期'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17'

  - canonical: 有效期
    aliases: [有效期, 效期]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [6.87, 7.94, 7.100, 冷-10]
    wms_status: acceptable_alias   # "效期"是"有效期"的简写，复合字段名（如"批号/效期"）保留；glossary 已映射
    data_type: DATE
    validation: '>今天'
    nullable: false
    encryption: none
    audit_required: true
    example: '2028-05-16'

  - canonical: 生产厂家
    aliases: [生产厂家, 厂家]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [6.87]
    wms_status: acceptable_alias   # "厂家"是简写，一般在内部正文出现；glossary 已映射
    data_type: VARCHAR(128)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '扬子江药业集团'

  - canonical: 批准文号
    aliases: [批准文号]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [6.87, 6.81]
    data_type: VARCHAR(32)
    validation: '^(国药准字|药品广告批准|HC|H|S|Z|J)[A-Z0-9]+$'
    nullable: false
    encryption: none
    audit_required: true
    example: '国药准字H20040030'

  - canonical: 追溯码
    aliases: [追溯码, UDI, 电子监管码, GSP_NBR, GTIN, TRAC_CODG_SN]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [追-1, 追-2, 追-3, 追-4, 追-5, 追-6]
    wms_status: acceptable_alias   # GSP 法规多种用语，WMS 统一"追溯码"为父概念；glossary §5/§6 已定义
    data_type: VARCHAR(32)
    validation: 'len in {8,12,17,20}'
    nullable: false
    encryption: none
    audit_required: true
    example: '81234567890123456'

  - canonical: 到货时间
    aliases: [到货时间, TO_DATE]
    field_class: gsp
    category: 时间
    gsp_clauses: [6.83, 6.87]
    data_type: TIMESTAMPTZ
    validation: '<=now() AND >=启运时间'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T08:30:00+08:00'

  - canonical: 启运时间
    aliases: [启运时间]
    field_class: gsp
    category: 时间
    gsp_clauses: [6.85, 冷-4]
    data_type: TIMESTAMPTZ
    validation: '<=到货时间'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T06:00:00+08:00'

  - canonical: 收货入库时间
    aliases: [收货入库时间, 收货时间]
    field_class: gsp
    category: 时间
    gsp_clauses: [6.83, 6.87]
    data_type: TIMESTAMPTZ
    validation: '>=到货时间'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T09:00:00+08:00'

  - canonical: 验收时间
    aliases: [验收时间, 签字时间]
    field_class: gsp
    category: 时间
    gsp_clauses: [6.87]
    data_type: TIMESTAMPTZ
    validation: '>=收货入库时间'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T10:30:00+08:00'

  - canonical: 上架时间
    aliases: [上架时间]
    field_class: gsp
    category: 时间
    gsp_clauses: [7.92]
    data_type: TIMESTAMPTZ
    validation: '>=验收时间'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T11:00:00+08:00'

  - canonical: 操作时间
    aliases: [操作时间, 出库时间, 发货时间]
    field_class: gsp
    category: 时间
    gsp_clauses: [5.67, 5.75, 8.111, 8.117]
    wms_status: acceptable_alias   # 流水账"操作时间" + 操作类型 = 出库时间/发货时间；语义合理
    data_type: TIMESTAMPTZ
    validation: '<=now()'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T14:00:00+08:00'

  - canonical: 触发时间
    aliases: [触发时间]
    field_class: gsp
    category: 时间
    gsp_clauses: [5.71]
    data_type: TIMESTAMPTZ
    validation: '<=now()'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T14:05:00+08:00'

  - canonical: 预报数量
    aliases: [预报数量]
    field_class: gsp
    category: 数量
    gsp_clauses: [6.83]
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '1000.000'

  - canonical: 送货数量
    aliases: [送货数量]
    field_class: gsp
    category: 数量
    gsp_clauses: [6.83]
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '998.000'

  - canonical: 实际到货数量
    aliases: [实际到货数量, 实到数量]
    field_class: gsp
    category: 数量
    gsp_clauses: [6.83, 6.87]
    wms_status: acceptable_alias   # "实到数量"是"实际到货数量"的简写；M2-002 字段表已统一为前者，正文中可能简写
    data_type: NUMERIC(15,3)
    validation: '>=0 AND <=送货数量'
    nullable: false
    encryption: none
    audit_required: true
    example: '998.000'

  - canonical: 缺货数量
    aliases: [缺货数量]
    field_class: gsp
    category: 数量
    gsp_clauses: [6.83, 6.88]
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '2.000'

  - canonical: 拒收数量
    aliases: [拒收数量]
    field_class: gsp
    category: 数量
    gsp_clauses: [6.88]
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '5.000'

  - canonical: 变动数量
    aliases: [变动数量, 库存数量]
    field_class: gsp
    category: 数量
    gsp_clauses: [7.102, 8.111, 8.113]
    wms_status: acceptable_alias   # "变动数量"是流水账字段，"库存数量"是业务术语；语义不同
    data_type: NUMERIC(15,3)
    validation: '!=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '-50.000'

  - canonical: 收货员 user_id
    aliases: [收货员 user_id]
    field_class: gsp
    category: 人员
    gsp_clauses: [6.83]
    data_type: BIGINT
    validation: 'FK -> user(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10001'

  - canonical: 验收员 user_id
    aliases: [验收员 user_id, 签字人, 收货员（验收岗）]
    field_class: gsp
    category: 人员
    gsp_clauses: [6.84, 特-3, 特-5]
    wms_status: acceptable_alias   # GSP 用"验收员"通用称谓，WMS M2-004 用"签字人"+"收货员（验收岗）"细化角色；glossary 已映射
    data_type: BIGINT
    validation: 'FK -> user(id) AND has_role(验收岗)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10002'

  - canonical: 复核人 user_id
    aliases: [复核人 user_id, 复核员 user_id]
    field_class: gsp
    category: 人员
    gsp_clauses: [8.112, 8.113, 特-3]
    data_type: BIGINT
    validation: 'FK -> user(id) AND has_role(复核岗)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10003'

  - canonical: 拣选人 user_id
    aliases: [拣选人 user_id, 拣选员 user_id]
    field_class: gsp
    category: 人员
    gsp_clauses: [8.113]
    data_type: BIGINT
    validation: 'FK -> user(id) AND has_role(拣选岗)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10004'

  - canonical: 上架人 user_id
    aliases: [上架人 user_id, 上架员 user_id]
    field_class: gsp
    category: 人员
    gsp_clauses: [7.92]
    data_type: BIGINT
    validation: 'FK -> user(id) AND has_role(上架岗)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10005'

  - canonical: 审批人 user_id
    aliases: [审批人 user_id, 审批人, CHEKUSER, OWNER_USER]
    field_class: gsp
    category: 人员
    gsp_clauses: [5.66, 5.72, 6.79, 不-2]
    wms_status: acceptable_alias   # "审批人" 是 "审批人 user_id" 的省略；故事正文使用，glossary 已映射
    data_type: BIGINT
    validation: 'FK -> user(id) AND has_role(审批岗)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10006'

  - canonical: 操作人 user_id
    aliases: [操作人 user_id]
    field_class: gsp
    category: 人员
    gsp_clauses: [5.67, 5.72, 5.75]
    data_type: BIGINT
    validation: 'FK -> user(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10007'

  - canonical: 库存状态
    aliases: [库存状态, 变动前状态, 变动后状态]
    field_class: gsp
    category: 状态
    gsp_clauses: [7.95, 8.114, 8.119, 不-1]
    wms_status: acceptable_alias   # 流水账时点字段细化（变动前/后状态合理）
    data_type: VARCHAR(20)
    validation: 'ENUM(可用|加锁|不合格|待销毁|预占|质冻|已分配)'
    nullable: false
    encryption: none
    audit_required: true
    example: '可用'

  - canonical: 待销毁
    aliases: [待销毁]
    field_class: gsp
    category: 状态
    gsp_clauses: [不-3]
    data_type: BOOLEAN
    validation: 'true|false'
    nullable: false
    encryption: none
    audit_required: true
    example: 'false'

  - canonical: 验收结论
    aliases: [验收结论, 质量状态]
    field_class: gsp
    category: 状态
    gsp_clauses: [6.87, 6.88]
    wms_status: acceptable_alias   # "验收结论"是 M2-003 字段名，"质量状态"是 GSP 法规用语兼业务术语；语义等价
    data_type: VARCHAR(20)
    validation: 'ENUM(合格|不合格|待复核)'
    nullable: false
    encryption: none
    audit_required: true
    example: '合格'

  - canonical: 联系单状态
    aliases: [联系单状态, 审批状态]
    field_class: gsp
    category: 状态
    gsp_clauses: [5.72, 不-2]
    data_type: VARCHAR(20)
    validation: 'ENUM(草稿|审批中|已通过|已拒绝|已撤销)'
    nullable: false
    encryption: none
    audit_required: true
    example: '已通过'

  - canonical: 召回标记
    aliases: [召回标记, recall_flag]
    field_class: gsp
    category: 状态
    gsp_clauses: [不-4, 不-5]
    wms_status: acceptable_alias   # 中英文双语（recall_flag 是字段 code，召回标记是中文术语）
    data_type: BOOLEAN
    validation: 'true|false'
    nullable: false
    encryption: none
    audit_required: true
    example: 'true'

  - canonical: 订单状态
    aliases: [订单状态, ASN 状态]
    field_class: gsp
    category: 状态
    gsp_clauses: [6.87, 8.111]
    wms_status: acceptable_alias   # M2 入库用"ASN 状态"，M4 出库用"订单状态"；状态机文档化合理
    data_type: VARCHAR(20)
    validation: 'ENUM(待校验|待收货|...)'
    nullable: false
    encryption: none
    audit_required: true
    example: '待收货'

  - canonical: 营业执照
    aliases: [营业执照]
    field_class: gsp
    category: 资质
    gsp_clauses: [6.79, 6.80]
    data_type: JSONB
    validation: '{ no, expire_date, image_url }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"no":"...", "expire_date":"2030-12-31"}'

  - canonical: 经营许可证
    aliases: [经营许可证]
    field_class: gsp
    category: 资质
    gsp_clauses: [6.79, 6.80]
    data_type: JSONB
    validation: '{ no, expire_date, scope, image_url }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"no":"鲁12345"}'

  - canonical: GSP 证
    aliases: [GSP 证]
    field_class: gsp
    category: 资质
    gsp_clauses: [6.79, 6.80]
    data_type: JSONB
    validation: '{ no, expire_date, image_url }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"no":"...", "expire_date":"2027-12-31"}'

  - canonical: 经营范围
    aliases: [经营范围]
    field_class: gsp
    category: 资质
    gsp_clauses: [6.80, 8.108]
    data_type: TEXT[]
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"中药饮片","中成药"}'

  - canonical: 客户资质
    aliases: [客户资质]
    field_class: gsp
    category: 资质
    gsp_clauses: [8.109]
    data_type: JSONB
    validation: '{ 营业执照, 经营许可证, GSP证, 法人代表, ... }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{...}'

  - canonical: 到货温度
    aliases: [到货温度, 收货温度, TEMPERATURE]
    field_class: gsp
    category: 冷链
    gsp_clauses: [6.85, 冷-4]
    data_type: NUMERIC(5,2)
    validation: '范围 -30.0 ~ 30.0'
    nullable: false
    encryption: none
    audit_required: true
    example: '5.20'

  - canonical: 装车温度
    aliases: [装车温度, 运输温度, 在途温度]
    field_class: gsp
    category: 冷链
    gsp_clauses: [6.85, 8.116, 冷-7]
    wms_status: acceptable_alias   # 出库装车 vs 在途运输两段管理（语义边界细化）
    data_type: NUMERIC(5,2)
    validation: '范围 -30.0 ~ 30.0'
    nullable: false
    encryption: none
    audit_required: true
    example: '4.80'

  - canonical: 温控方式
    aliases: [温控方式]
    field_class: gsp
    category: 冷链
    gsp_clauses: [6.85, 冷-1]
    data_type: VARCHAR(20)
    validation: 'ENUM(冷藏车|保温箱|常温车|普通)'
    nullable: false
    encryption: none
    audit_required: true
    example: '冷藏车'

  - canonical: 冷藏车
    aliases: [冷藏车]
    field_class: gsp
    category: 冷链
    gsp_clauses: [冷-1, 冷-7]
    data_type: JSONB
    validation: '{ vehicle_no, temp_range, GPS_id }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"vehicle_no":"鲁A12345"}'

  - canonical: 保温箱
    aliases: [保温箱, 保温箱号, WARMBOXID]
    field_class: gsp
    category: 冷链
    gsp_clauses: [8.116, 冷-6]
    data_type: JSONB
    validation: '{ box_id, capacity, ice_pack_count }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"box_id":"BX001"}'

  - canonical: 蓄冷剂
    aliases: [蓄冷剂]
    field_class: gsp
    category: 冷链
    gsp_clauses: [冷-6]
    data_type: JSONB
    validation: '{ count, conditioning_status }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"count":4,"conditioning":"完成"}'

  - canonical: 温度超标事件
    aliases: [温度超标事件]
    field_class: gsp
    category: 冷链
    gsp_clauses: [冷-8]
    data_type: JSONB
    validation: '{ event_id, exceed_seconds, max_temp, action }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{...}'

  - canonical: 上报记录
    aliases: [上报记录, 上报状态]
    field_class: gsp
    category: 追溯
    gsp_clauses: [追-4]
    wms_status: acceptable_alias   # "上报记录"是数据，"上报状态"是字段值（成功/失败/待重试）；语义不同但相关
    data_type: VARCHAR(20)
    validation: 'ENUM(待上报|上报中|成功|失败|待重试)'
    nullable: false
    encryption: none
    audit_required: true
    example: '成功'

  - canonical: 反查链路
    aliases: [反查, 反向追溯]
    field_class: gsp
    category: 追溯
    gsp_clauses: [追-6]
    wms_status: acceptable_alias   # "反查"用于接口/PDA语境，"反向追溯"用于合规/文档语境
    data_type: JSONB
    validation: '{ trace_path[], from, to }'
    nullable: true
    encryption: none
    audit_required: true
    example: '{...}'

  - canonical: 召回记录
    aliases: [召回记录, recall_id]
    field_class: gsp
    category: 追溯
    gsp_clauses: [不-4, 不-5]
    wms_status: acceptable_alias   # 中英文双语（recall_id 是字段 code，召回记录是中文术语）
    data_type: BIGINT
    validation: 'FK -> recall(id)'
    nullable: true
    encryption: none
    audit_required: true
    example: '20001'

  - canonical: special_drug_category
    aliases: [special_drug_category, 特殊药品分类]
    field_class: gsp
    category: 特殊管理
    gsp_clauses: [特-1, 特-2]
    wms_status: acceptable_alias   # 中英文双语合理（code/字段名 + 中文显示）
    data_type: VARCHAR(20)
    validation: 'ENUM(麻醉|精神一类|精神二类|医疗用毒性|放射性|疫苗|生物制品|普通)'
    nullable: false
    encryption: none
    audit_required: true
    example: '精神二类'

  - canonical: 双人策略命中规则 ID
    aliases: [双人策略命中规则 ID]
    field_class: gsp
    category: 特殊管理
    gsp_clauses: [特-3, 特-4, 特-5]
    data_type: BIGINT
    validation: 'FK -> dual_person_rule(id)'
    nullable: true
    encryption: none
    audit_required: true
    example: '100'

  - canonical: 第二操作人 user_id
    aliases: [第二操作人 user_id]
    field_class: gsp
    category: 特殊管理
    gsp_clauses: [特-3, 特-5, 特-9]
    data_type: BIGINT
    validation: 'FK -> user(id) AND user_id != 第一操作人'
    nullable: true
    encryption: none
    audit_required: true
    example: '10010'

  - canonical: 销毁原因码
    aliases: [销毁原因码, 销毁原因]
    field_class: gsp
    category: 特殊管理
    gsp_clauses: [不-3, 特-9]
    wms_status: acceptable_alias   # M-SA-001 字段名是"销毁原因码"（v25），"销毁原因"是简写
    data_type: VARCHAR(20)
    validation: 'ENUM(过期|破损|召回|质量问题|监管要求)'
    nullable: false
    encryption: none
    audit_required: true
    example: '过期'

  - canonical: 专用台账标识
    aliases: [专用台账标识, requires_dedicated_ledger]
    field_class: gsp
    category: 特殊管理
    gsp_clauses: [特-7, 特-8]
    data_type: BOOLEAN
    validation: 'true|false'
    nullable: false
    encryption: none
    audit_required: true
    example: 'true'

  - canonical: 养护频率
    aliases: [养护频率]
    field_class: gsp
    category: 养护
    gsp_clauses: [7.97]
    data_type: VARCHAR(20)
    validation: 'ENUM(每日|每周|每月|每季)'
    nullable: false
    encryption: none
    audit_required: true
    example: '每月'

  - canonical: 养护异常
    aliases: [养护异常]
    field_class: gsp
    category: 养护
    gsp_clauses: [7.99]
    data_type: JSONB
    validation: '{ type, severity, action_taken }'
    nullable: true
    encryption: none
    audit_required: true
    example: '{...}'

  - canonical: 温湿度记录
    aliases: [温湿度记录, 温度记录]
    field_class: gsp
    category: 养护
    gsp_clauses: [7.98, 冷-3]
    wms_status: acceptable_alias   # GSP 7.98 / 冷-3 不同条款混用；温度是温湿度的子集；glossary 已映射
    data_type: JSONB
    validation: '{ ts, temp, humidity, sensor_id }[]'
    nullable: false
    encryption: none
    audit_required: true
    example: '[{...}]'

  - canonical: 近效期预警
    aliases: [近效期预警, 近效期]
    field_class: gsp
    category: 养护
    gsp_clauses: [7.100]
    wms_status: acceptable_alias   # "近效期"是状态标签，"近效期预警"是触发动作；语义不同
    data_type: BOOLEAN
    validation: 'true|false（计算字段）'
    nullable: false
    encryption: none
    audit_required: true
    example: 'true'

  - canonical: 库位
    aliases: [库位, 库位编码, 实际库位, 推荐库位, 目标库位, 货位ID, LOCN_ID, LOCN_BRCD]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [7.92, 7.95, 7.96]
    wms_status: acceptable_alias   # PDA 字段细化（系统推荐/实际选择/目标库位语义不同）
    data_type: VARCHAR(32)
    validation: '^[A-Z]\d{2}-\d{2}-\d{2}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'A01-02-03'

  - canonical: 货主
    aliases: [货主, owner_id, TC_COMPANY_ID, INVOWNERID, OWNERID]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [M1-007]
    wms_status: acceptable_alias   # 中英双语混用合理（owner_id 用于 schema/接口，货主用于业务文档）
    data_type: BIGINT
    validation: 'FK -> owner(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '1'

  - canonical: 运输方式
    aliases: [运输方式]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [6.85, 8.116]
    data_type: VARCHAR(20)
    validation: 'ENUM(冷藏车|保温箱|常温|空运|铁路)'
    nullable: false
    encryption: none
    audit_required: true
    example: '冷藏车'

  - canonical: 承运商
    aliases: [承运商, 送货承运商, CARRIER_NAME, CARRIER, SHIP_VIA]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [6.83, 6.85]
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '顺丰冷运'

  - canonical: 随货同行单
    aliases: [随货同行单]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [8.115]
    data_type: JSONB
    validation: '{ no, items, qr_url }'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"no":"SD20260517001"}'

  # 🔵 审计字段（系统级追溯，5 项 — GSP 5.67/5.72 明示） — 5 项
  - canonical: 旧值
    aliases: [旧值]
    field_class: audit
    category: 审计
    gsp_clauses: [5.67, 5.72]
    data_type: JSONB
    validation: '可序列化'
    nullable: true
    encryption: none
    audit_required: true
    example: '{"status":"可用"}'

  - canonical: 新值
    aliases: [新值]
    field_class: audit
    category: 审计
    gsp_clauses: [5.67, 5.72]
    data_type: JSONB
    validation: '可序列化'
    nullable: true
    encryption: none
    audit_required: true
    example: '{"status":"加锁"}'

  - canonical: IP 地址
    aliases: [IP 地址]
    field_class: audit
    category: 审计
    gsp_clauses: [5.67]
    data_type: INET
    validation: 'v4 或 v6'
    nullable: true
    encryption: masked
    audit_required: true
    example: '192.168.1.100'

  - canonical: 关联单据
    aliases: [关联单据, 关联 ASN, 关联 ASN 号]
    field_class: audit
    category: 审计
    gsp_clauses: [5.65, 5.67]
    wms_status: acceptable_alias   # 通用 vs 入库专用合理细化
    data_type: VARCHAR(32)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: true
    example: 'ASN20260517001'

  - canonical: 审批源
    aliases: [审批源, approval_source]
    field_class: audit
    category: 审计
    gsp_clauses: [5.72, 不-2, 特-9]
    wms_status: acceptable_alias   # 中英文双语合理（code/字段名 + 中文显示）
    data_type: VARCHAR(20)
    validation: 'ENUM(质量联系单|库存调整审批|验收|对账|养护|系统盘点|退货审批)'
    nullable: false
    encryption: none
    audit_required: true
    example: '质量联系单'

  # ── P0 字段补充（2026-05-17 v3.1，从 legacy Oracle WMS 提取）──
  - canonical: country_of_origin
    aliases: [country_of_origin, 原产国, 产地, CNTRY_OF_ORGN]
    field_class: gsp
    category: 基础属性
    gsp_clauses: [6.87, 8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(64)
    validation: 'from_dict(国家字典) OR ISO 3166-1 alpha-2'
    nullable: false
    encryption: none
    audit_required: true
    example: '中国'

  - canonical: delivery_org_name
    aliases: [delivery_org_name, 配送单位名称, DELV_EMP]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(128)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '上海冷链物流有限公司'

  - canonical: delivery_org_uscc
    aliases: [delivery_org_uscc, 配送单位统一社会信用代码, DELV_EMP_USCC]
    field_class: gsp
    category: 资质
    gsp_clauses: [8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(18)
    validation: '^[0-9A-HJ-NPQRTUWXY]{2}\d{6}[0-9A-HJ-NPQRTUWXY]{10}$'
    nullable: false
    encryption: none
    audit_required: true
    example: '91110108MA01ABCDXY'

  - canonical: shipper_org_name
    aliases: [shipper_org_name, 发货机构名称, SHP_ORG_NAME]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(128)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '本公司中心仓'

  - canonical: shipper_org_uscc
    aliases: [shipper_org_uscc, 发货机构统一社会信用代码, SHP_ORG_USCC]
    field_class: gsp
    category: 资质
    gsp_clauses: [8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(18)
    validation: '^[0-9A-HJ-NPQRTUWXY]{2}\d{6}[0-9A-HJ-NPQRTUWXY]{10}$'
    nullable: false
    encryption: none
    audit_required: true
    example: '91110108MA01XYZ123'

  - canonical: receiver_org_name
    aliases: [receiver_org_name, 收货机构名称, SHPP_ORG_NAME]
    field_class: gsp
    category: 其他业务
    gsp_clauses: [8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(128)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '某医院药剂科'

  - canonical: receiver_org_uscc
    aliases: [receiver_org_uscc, 收货机构统一社会信用代码, SHPP_ORG_USCC]
    field_class: gsp
    category: 资质
    gsp_clauses: [8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(18)
    validation: '^[0-9A-HJ-NPQRTUWXY]{2}\d{6}[0-9A-HJ-NPQRTUWXY]{10}$'
    nullable: false
    encryption: none
    audit_required: true
    example: '91110108MA01HOSP456'

  - canonical: shipment_doc_no
    aliases: [shipment_doc_no, 货单编号, SHP_SIN_NO]
    field_class: gsp
    category: 追溯
    gsp_clauses: [追-1, 追-2, 8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: VARCHAR(32)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: 'SHP20260517001'

  - canonical: operation_type
    aliases: [operation_type, 操作类型, INV_TYPE]
    field_class: gsp
    category: 追溯
    gsp_clauses: [追-3, 8.111]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    # 101=入库 201=出库（码上放心规范）
    data_type: VARCHAR(8)
    validation: 'IN (101, 201)'
    nullable: false
    encryption: none
    audit_required: true
    example: '101'

  - canonical: delivery_time
    aliases: [delivery_time, 配送时间, DELV_TIME]
    field_class: gsp
    category: 时间
    gsp_clauses: [8.111, 8.116]
    wms_status: unimplemented   # v25 backlog：故事字段表待补
    data_type: TIMESTAMPTZ
    validation: '<=now() AND >=启运时间'
    nullable: false
    encryption: none
    audit_required: true
    example: '2026-05-17T15:30:00+08:00'

  # 🟢 业务字段（业务流程必需，GSP 未明示） — 40 项
  - canonical: ASN 号
    aliases: [ASN 号, asn_no, TC_ASN_ID, SHPMT_NBR, SHIPMENT_NBR]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^ASN\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'ASN20260517001'

  - canonical: 出库单号
    aliases: [出库单号, shipment_no, ORDER_ID, TC_ORDER_ID]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^SO\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'SO20260517001'

  - canonical: 拣货单号
    aliases: [拣货单号, pick_no, PKT_CTRL_NBR, WAVE_NBR]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^PK\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'PK20260517001'

  - canonical: 复核单号
    aliases: [复核单号, verify_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^VR\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'VR20260517001'

  - canonical: 装车单号
    aliases: [装车单号, load_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^LD\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'LD20260517001'

  - canonical: 任务号
    aliases: [任务号, task_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: 'T20260517001'

  - canonical: 调拨单号
    aliases: [调拨单号, transfer_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^TF\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'TF20260517001'

  - canonical: 销毁单号
    aliases: [销毁单号, destruction_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^DS\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'DS20260517001'

  - canonical: 盘点单号
    aliases: [盘点单号, stocktake_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^ST\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'ST20260517001'

  - canonical: 商品 ID
    aliases: [商品 ID, product_id, SKU_ID, ITEM_ID]
    field_class: business
    category: 基础属性
    data_type: BIGINT
    validation: 'FK -> product(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10001'

  - canonical: 仓库 ID
    aliases: [仓库 ID, warehouse_id, WHSE, FACILITY_ID, CD_MASTER_ID]
    field_class: business
    category: 其他业务
    data_type: BIGINT
    validation: 'FK -> warehouse(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '1'

  - canonical: 库区编码
    aliases: [库区编码, zone_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(20)
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: 'A区'

  - canonical: 库位类型
    aliases: [库位类型, location_type]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(20)
    validation: 'ENUM(货架位|拣选位|暂存区|缓存区|月台)'
    nullable: false
    encryption: none
    audit_required: true
    example: '货架位'

  - canonical: 客户 ID
    aliases: [客户 ID, customer_id]
    field_class: business
    category: 其他业务
    data_type: BIGINT
    validation: 'FK -> customer(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '20001'

  - canonical: 客户编码
    aliases: [客户编码, customer_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^CUS\d{6}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'CUS000001'

  - canonical: 供应商 ID
    aliases: [供应商 ID, supplier_id]
    field_class: business
    category: 其他业务
    data_type: BIGINT
    validation: 'FK -> supplier(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '30001'

  - canonical: 供应商编码
    aliases: [供应商编码, supplier_no, 供应商代码, CSTCODE]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^SUP\d{6}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'SUP000001'

  - canonical: 任务类型
    aliases: [任务类型, task_type]
    field_class: business
    category: 状态
    data_type: VARCHAR(20)
    validation: 'ENUM(上架|拣选|复核|盘点|养护|调拨|销毁)'
    nullable: false
    encryption: none
    audit_required: true
    example: '拣选'

  - canonical: 任务状态
    aliases: [任务状态, task_status]
    field_class: business
    category: 状态
    data_type: VARCHAR(20)
    validation: 'ENUM(待领取|进行中|已完成|已取消)'
    nullable: false
    encryption: none
    audit_required: true
    example: '进行中'

  - canonical: 任务优先级
    aliases: [任务优先级, priority]
    field_class: business
    category: 其他业务
    data_type: SMALLINT
    validation: '1-9（1 最高）'
    nullable: false
    encryption: none
    audit_required: true
    example: '5'

  - canonical: 已分配数量
    aliases: [已分配数量, allocated_qty]
    field_class: business
    category: 数量
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '100.000'

  - canonical: 已拣货数量
    aliases: [已拣货数量, picked_qty]
    field_class: business
    category: 数量
    data_type: NUMERIC(15,3)
    validation: '>=0 AND <=allocated_qty'
    nullable: false
    encryption: none
    audit_required: true
    example: '98.000'

  - canonical: 已复核数量
    aliases: [已复核数量, verified_qty]
    field_class: business
    category: 数量
    data_type: NUMERIC(15,3)
    validation: '>=0 AND <=picked_qty'
    nullable: false
    encryption: none
    audit_required: true
    example: '98.000'

  - canonical: 可用数量
    aliases: [可用数量, qty_available]
    field_class: business
    category: 数量
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '500.000'

  - canonical: 预占数量
    aliases: [预占数量, qty_reserved]
    field_class: business
    category: 数量
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '100.000'

  - canonical: 通道号
    aliases: [通道号, dock_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(8)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: false
    example: 'D01'

  - canonical: 月台号
    aliases: [月台号, platform_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(8)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: false
    example: 'P03'

  - canonical: 配送地址
    aliases: [配送地址, delivery_address]
    field_class: business
    category: 其他业务
    data_type: TEXT
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '山东省济南市...'

  - canonical: 收件人电话
    aliases: [收件人电话, recipient_phone]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(20)
    validation: '^1\d{10}$ OR ^0\d{2,3}-?\d{7,8}$'
    nullable: false
    encryption: masked
    audit_required: true
    example: '186****1234'

  - canonical: 应收金额
    aliases: [应收金额, amount_due]
    field_class: business
    category: 数量
    data_type: NUMERIC(15,2)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '1234.56'

  - canonical: 计费数量
    aliases: [计费数量, billable_qty]
    field_class: business
    category: 数量
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '100.000'

  - canonical: 月结账单号
    aliases: [月结账单号, monthly_bill_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^BL\d{6}\d{6}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'BL202605000001'

  - canonical: 整箱出库标识
    aliases: [整箱出库标识, is_full_carton]
    field_class: business
    category: 状态
    data_type: BOOLEAN
    validation: 'true|false'
    nullable: false
    encryption: none
    audit_required: true
    example: 'true'

  - canonical: 越库标识
    aliases: [越库标识, is_crossdock]
    field_class: business
    category: 状态
    data_type: BOOLEAN
    validation: 'true|false'
    nullable: false
    encryption: none
    audit_required: true
    example: 'false'

  - canonical: 零拣标识
    aliases: [零拣标识, is_pick_to_light]
    field_class: business
    category: 状态
    data_type: BOOLEAN
    validation: 'true|false'
    nullable: false
    encryption: none
    audit_required: true
    example: 'true'

  - canonical: LPN 号
    aliases: [LPN 号, lpn_no, CNTR_NBR, TC_LPN_ID, 货箱号]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(32)
    validation: '^L\d{14}\d{3}$'
    nullable: false
    encryption: none
    audit_required: true
    example: 'L20260517001'

  - canonical: 周转箱编码
    aliases: [周转箱编码, tote_no]
    field_class: business
    category: 其他业务
    data_type: VARCHAR(20)
    validation: '^TT\d{6}$'
    nullable: true
    encryption: none
    audit_required: true
    example: 'TT000001'

  - canonical: 整托数量
    aliases: [整托数量, qty_per_pallet]
    field_class: business
    category: 数量
    data_type: NUMERIC(10,3)
    validation: '>0'
    nullable: true
    encryption: none
    audit_required: false
    example: '1000.000'

  - canonical: 状态变更原因
    aliases: [状态变更原因, status_change_reason]
    field_class: business
    category: 状态
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: true
    example: '质量联系单 QL20260517001'

  - canonical: 业务备注
    aliases: [业务备注, remark]
    field_class: business
    category: 其他业务
    data_type: TEXT
    validation: '<=500 字符'
    nullable: true
    encryption: none
    audit_required: false
    example: '急件'

  # ⚪ 系统元数据（技术性必需） — 8 项
  - canonical: id
    aliases: [id, ID 主键]
    field_class: system
    category: 其他业务
    data_type: BIGSERIAL
    validation: '>0'
    nullable: false
    encryption: none
    audit_required: false
    example: '1001'

  - canonical: created_at
    aliases: [created_at, 创建时间, CREATE_DATE_TIME, SYS_CRT_DTM, CREATEDATE, ADD_DATE]
    field_class: system
    category: 时间
    data_type: TIMESTAMPTZ
    validation: 'DEFAULT now()'
    nullable: false
    encryption: none
    audit_required: false
    example: '2026-05-17T08:30:00+08:00'

  - canonical: updated_at
    aliases: [updated_at, 更新时间, 修改时间, MOD_DATE_TIME, SYS_MDF_DTM, MODIFYDATE, LAST_UPDATED_DTTM]
    field_class: system
    category: 时间
    data_type: TIMESTAMPTZ
    validation: 'DEFAULT now()'
    nullable: false
    encryption: none
    audit_required: false
    example: '2026-05-17T08:35:00+08:00'

  - canonical: created_by
    aliases: [created_by, 创建人, USER_ID, SYS_CRT_BY, CREATEUSER, ADD_USER]
    field_class: system
    category: 人员
    data_type: BIGINT
    validation: 'FK -> user(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '10001'

  - canonical: updated_by
    aliases: [updated_by, 更新人, 修改人, MOD_USER, SYS_MDF_BY, MODIFYUSER]
    field_class: system
    category: 人员
    data_type: BIGINT
    validation: 'FK -> user(id)'
    nullable: true
    encryption: none
    audit_required: true
    example: '10002'

  - canonical: version
    aliases: [version, 乐观锁版本]
    field_class: system
    category: 其他业务
    data_type: BIGINT
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: false
    example: '1'

  - canonical: tenant_id
    aliases: [tenant_id, 租户标识]
    field_class: system
    category: 其他业务
    data_type: BIGINT
    validation: 'FK -> tenant(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '1'

  - canonical: deleted_at
    aliases: [deleted_at, 软删时间]
    field_class: system
    category: 时间
    data_type: TIMESTAMPTZ
    validation: 'DEFAULT NULL'
    nullable: true
    encryption: none
    audit_required: true
    example: 'NULL'

  # 🟡 配置字段（M1-008 配置中心可调） — 15 项
  - canonical: expire_warning_days
    aliases: [expire_warning_days, 近效期阈值]
    field_class: config
    category: 养护
    data_type: INT
    validation: '1-365 天'
    nullable: false
    encryption: none
    audit_required: true
    example: '30'

  - canonical: chain_break_threshold_count
    aliases: [chain_break_threshold_count, 断链连续超标次数]
    field_class: config
    category: 冷链
    data_type: INT
    validation: '1-100'
    nullable: false
    encryption: none
    audit_required: true
    example: '3'

  - canonical: chain_break_severe_max_delta
    aliases: [chain_break_severe_max_delta, 严重断链温差阈值]
    field_class: config
    category: 冷链
    data_type: NUMERIC(5,2)
    validation: '0.0-50.0'
    nullable: false
    encryption: none
    audit_required: true
    example: '5.00'

  - canonical: temp_sample_interval_seconds
    aliases: [temp_sample_interval_seconds, 温度采样间隔]
    field_class: config
    category: 冷链
    data_type: INT
    validation: '5-3600 秒'
    nullable: false
    encryption: none
    audit_required: true
    example: '30'

  - canonical: fifo_mode
    aliases: [fifo_mode, FIFO 严格模式]
    field_class: config
    category: 其他业务
    data_type: VARCHAR(10)
    validation: 'ENUM(strict|loose)'
    nullable: false
    encryption: none
    audit_required: true
    example: 'strict'

  - canonical: dual_person_categories
    aliases: [dual_person_categories, 双人复核触发类别]
    field_class: config
    category: 特殊管理
    data_type: TEXT[]
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: true
    example: '{"麻醉","精神一类","医疗用毒性","放射性"}'

  - canonical: audit_retention_years
    aliases: [audit_retention_years, 审计保留年限]
    field_class: config
    category: 审计
    data_type: INT
    validation: '5 OR 30'
    nullable: false
    encryption: none
    audit_required: true
    example: '5'

  - canonical: recall_notification_template
    aliases: [recall_notification_template, 召回通知模板]
    field_class: config
    category: 其他业务
    data_type: TEXT
    validation: 'non_empty'
    nullable: false
    encryption: none
    audit_required: false
    example: '尊敬的客户...'

  - canonical: cold_zone_temp_min
    aliases: [cold_zone_temp_min, 冷链区温度下限]
    field_class: config
    category: 冷链
    data_type: NUMERIC(5,2)
    validation: '范围 -50.0 ~ 30.0'
    nullable: false
    encryption: none
    audit_required: true
    example: '2.00'

  - canonical: cold_zone_temp_max
    aliases: [cold_zone_temp_max, 冷链区温度上限]
    field_class: config
    category: 冷链
    data_type: NUMERIC(5,2)
    validation: '范围 cold_zone_temp_min ~ 50.0'
    nullable: false
    encryption: none
    audit_required: true
    example: '8.00'

  - canonical: inventory_lock_timeout_minutes
    aliases: [inventory_lock_timeout_minutes, 库存预占超时]
    field_class: config
    category: 其他业务
    data_type: INT
    validation: '1-1440 分钟'
    nullable: false
    encryption: none
    audit_required: true
    example: '30'

  - canonical: picking_strategy
    aliases: [picking_strategy, 拣货策略]
    field_class: config
    category: 其他业务
    data_type: VARCHAR(20)
    validation: 'ENUM(单人|波次|分区|总拣后分播)'
    nullable: false
    encryption: none
    audit_required: true
    example: '波次'

  - canonical: replenishment_threshold
    aliases: [replenishment_threshold, 补货阈值]
    field_class: config
    category: 数量
    data_type: NUMERIC(15,3)
    validation: '>=0'
    nullable: false
    encryption: none
    audit_required: true
    example: '20.000'

  - canonical: audit_log_export_format
    aliases: [audit_log_export_format, 审计导出格式]
    field_class: config
    category: 审计
    data_type: VARCHAR(10)
    validation: 'ENUM(PDF|CSV|JSON)'
    nullable: false
    encryption: none
    audit_required: false
    example: 'PDF'

  - canonical: default_warehouse_id
    aliases: [default_warehouse_id, 默认仓库]
    field_class: config
    category: 其他业务
    data_type: BIGINT
    validation: 'FK -> warehouse(id)'
    nullable: false
    encryption: none
    audit_required: true
    example: '1'

  # 🟣 计算字段（系统派生） — 10 项
  - canonical: is_near_expiry
    aliases: [is_near_expiry, 近效期标记]
    field_class: derived
    category: 养护
    data_type: BOOLEAN
    validation: 'GENERATED: expire_date - CURRENT_DATE <= expire_warning_days'
    nullable: false
    encryption: none
    audit_required: false
    example: 'true'

  - canonical: days_to_expire
    aliases: [days_to_expire, 剩余效期天数]
    field_class: derived
    category: 时间
    data_type: INT
    validation: 'GENERATED: expire_date - CURRENT_DATE'
    nullable: false
    encryption: none
    audit_required: false
    example: '29'

  - canonical: age_days
    aliases: [age_days, 库存天数]
    field_class: derived
    category: 时间
    data_type: INT
    validation: 'GENERATED: CURRENT_DATE - 入库日期'
    nullable: false
    encryption: none
    audit_required: false
    example: '15'

  - canonical: is_recalled
    aliases: [is_recalled, 已召回标记]
    field_class: derived
    category: 状态
    data_type: BOOLEAN
    validation: 'GENERATED: 召回标记 OR EXISTS recall_record'
    nullable: false
    encryption: none
    audit_required: false
    example: 'false'

  - canonical: is_pickable
    aliases: [is_pickable, 可拣货标记]
    field_class: derived
    category: 状态
    data_type: BOOLEAN
    validation: 'GENERATED: status=''可用'' AND qty_available>0 AND NOT is_recalled'
    nullable: false
    encryption: none
    audit_required: false
    example: 'true'

  - canonical: consecutive_exceed_count
    aliases: [consecutive_exceed_count, 连续超标次数]
    field_class: derived
    category: 冷链
    data_type: INT
    validation: 'GENERATED: 实时计算最近连续超标采样数'
    nullable: false
    encryption: none
    audit_required: false
    example: '0'

  - canonical: picking_priority
    aliases: [picking_priority, 拣货优先级]
    field_class: derived
    category: 其他业务
    data_type: INT
    validation: 'GENERATED: based_on(expire_date, created_at, batch_no)'
    nullable: false
    encryption: none
    audit_required: false
    example: '1'

  - canonical: requires_special_handling
    aliases: [requires_special_handling, 需要特殊处理]
    field_class: derived
    category: 特殊管理
    data_type: BOOLEAN
    validation: 'GENERATED: special_drug_category != ''普通'''
    nullable: false
    encryption: none
    audit_required: false
    example: 'true'

  - canonical: inventory_age_seconds
    aliases: [inventory_age_seconds, 库存秒级年龄]
    field_class: derived
    category: 时间
    data_type: BIGINT
    validation: 'GENERATED: extract(epoch from now() - created_at)'
    nullable: false
    encryption: none
    audit_required: false
    example: '1296000'

  - canonical: can_self_destroy
    aliases: [can_self_destroy, 可自主销毁]
    field_class: derived
    category: 特殊管理
    data_type: BOOLEAN
    validation: 'GENERATED: special_drug_category NOT IN (麻醉, 放射性) AND has_destruction_approval'
    nullable: false
    encryption: none
    audit_required: false
    example: 'true'

  # ⚫ 接口字段（跨系统集成） — 10 项
  - canonical: external_ref
    aliases: [external_ref, 外部系统主键]
    field_class: interface
    category: 其他业务
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: true
    example: 'ERP-PO-2026-001'

  - canonical: source_system
    aliases: [source_system, 来源系统]
    field_class: interface
    category: 其他业务
    data_type: VARCHAR(32)
    validation: 'ENUM(ERP|TMS|监管平台|码上放心|微信|手工)'
    nullable: false
    encryption: none
    audit_required: true
    example: 'ERP'

  - canonical: sync_status
    aliases: [sync_status, 同步状态]
    field_class: interface
    category: 状态
    data_type: VARCHAR(20)
    validation: 'ENUM(待同步|同步中|成功|失败|放弃)'
    nullable: false
    encryption: none
    audit_required: true
    example: '成功'

  - canonical: sync_at
    aliases: [sync_at, 最后同步时间]
    field_class: interface
    category: 时间
    data_type: TIMESTAMPTZ
    validation: '<=now()'
    nullable: true
    encryption: none
    audit_required: true
    example: '2026-05-17T09:00:00+08:00'

  - canonical: sync_retry_count
    aliases: [sync_retry_count, 重试次数]
    field_class: interface
    category: 其他业务
    data_type: INT
    validation: '0-10'
    nullable: false
    encryption: none
    audit_required: true
    example: '0'

  - canonical: erp_doc_no
    aliases: [erp_doc_no, ERP 单据号]
    field_class: interface
    category: 其他业务
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: true
    example: 'PO-2026-05-17-001'

  - canonical: tms_shipment_id
    aliases: [tms_shipment_id, TMS 调度号]
    field_class: interface
    category: 其他业务
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: true
    example: 'TMS20260517001'

  - canonical: wechat_msg_id
    aliases: [wechat_msg_id, 企微消息 ID]
    field_class: interface
    category: 其他业务
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: false
    example: 'WX-20260517-1234'

  - canonical: regulatory_report_id
    aliases: [regulatory_report_id, 监管平台流水号]
    field_class: interface
    category: 追溯
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: true
    example: 'REG20260517001'

  - canonical: third_party_carrier_no
    aliases: [third_party_carrier_no, 第三方承运单号]
    field_class: interface
    category: 其他业务
    data_type: VARCHAR(64)
    validation: 'non_empty'
    nullable: true
    encryption: none
    audit_required: true
    example: 'SF20260517001'

```

---

## 7. 维护规则

1. **GSP 矩阵新增条款触发**：条款级矩阵（README）新增条款时，识别其中字段，回头补到本矩阵对应类别
2. **故事字段表新增字段触发**：故事字段表新增字段时，反查 GSP 是否有对应条款，无则在状态列填"⚪ 业务字段"，有则在 §6 字段词典加 alias
3. **命名规范化触发**：当 §5 命名一致性问题在多个故事重复出现，应在 docs/glossary.md 加术语映射，本矩阵对应字段状态从 🟡 升 ✅
4. **治理脚本依赖**：`scripts/governance/check_gsp_field_traceability.py` 输入本矩阵 §6 字段词典，输出与故事字段表的一致性报告

---

## 8. 与其他文档的关系

| 文档 | 关系 |
|------|------|
| `docs/compliance/README.md` | 条款级 RTM；本矩阵字段反查到的条款引用此 |
| `docs/compliance/gsp-ch5-warehouse-management.md` ~ `gsp-special-drugs.md` | 各章节条款级矩阵；本矩阵字段从原文中提取 |
| `docs/glossary.md` | 字段术语规范；本矩阵 §5 命名问题修复需更新 glossary |
| `scripts/governance/check_gsp_field_traceability.py` | 自动化核对脚本；以本矩阵 §6 字段词典为输入 |

---

## 9. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-17 | v1 | 初版（v25 后期阶段建立）：12 类 73 字段全字段追溯 + 9 项命名一致性问题 + 字段词典 §6 |
| 2026-05-17 | v2 | v25 修复（"修复"指令）：(a) 召回标记 / 召回记录字段补到 M3-002 §7 + M-QL-002 §5（2 项 backlog 清零）(b) 22 项多 alias 字段标 acceptable_alias 并按语义分类（细化语义 14 / 中英双语 4 / GSP 简写 4）(c) §5 重写为命名一致性处置归档；治理脚本运行干净（0 错误 / 0 警告 / 0 信息）|
| 2026-05-17 | v2.1 | v2 字段技术属性扩展：每字段增加 5 列技术属性（data_type / validation / nullable / encryption / audit_required / example），70 GSP 字段升级为"业务+合规+技术"三位一体；治理脚本支持技术属性完整性校验 |
| 2026-05-17 | v3 | 字段性质类别扩展（B 完整版）：词典从 70 GSP 字段扩展到 153 字段，覆盖 7 性质类别（gsp 65 + audit 5 + business 40 + system 8 + config 15 + derived 10 + interface 10）；新增 §3.13 性质类别索引与 §6.1-6.2 治理规则；治理脚本支持 field_class + category 校验，gsp/audit 强制（error），business/config/interface 软警告（warning，Wave 1 backlog），system/derived 不强制 |
| 2026-05-17 | v3.1 | 从 legacy Oracle WMS 提取字段补强：(a) PR-A 22 个现有 canonical 追加 31+ legacy 英文 alias（WHSE/FACILITY_ID/CD_MASTER_ID/LOCN_ID/BATCH_NBR/GSP_NBR/CNTR_NBR/TC_LPN_ID/PKT_CTRL_NBR/WAVE_NBR/TC_COMPANY_ID/INVOWNERID/USER_ID/MOD_USER/CREATE_DATE_TIME/MOD_DATE_TIME/LAST_UPDATED_DTTM/SYS_CRT_*/SYS_MDF_*/ORDER_ID/TC_ORDER_ID/SKU_ID/ITEM_ID/DESCRIPTION/SKU_DESC/TEMPERATURE/TO_DATE/CARRIER_NAME/SHIP_VIA/WARMBOXID/CHEKUSER/TC_ASN_ID/SHPMT_NBR/CSTCODE 等），消除 legacy 列名↔canonical 的对照盲区。(b) PR-B 新增 10 个 P0 GSP canonical（country_of_origin / delivery_org_name + delivery_org_uscc / shipper_org_name + shipper_org_uscc / receiver_org_name + receiver_org_uscc / shipment_doc_no / operation_type / delivery_time），覆盖 GSP 8.111 配送/发货/收货机构 USCC 标识 + 码上放心上报 4 字段。10 项暂标 wms_status: unimplemented（v25 backlog），故事字段表后续 PR 补全。词典总数 153 → 163，GSP 强制 70 → 80。治理脚本：0 error / 53 warning / 15 info（含 10 项 backlog 通知），T1 通过 |

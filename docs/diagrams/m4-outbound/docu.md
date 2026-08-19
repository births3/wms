# M4 出库流程图文说明

## 目的

本文配套说明 M4 出库流程的 PlantUML 图，帮助后续设计、开发、review、子代理拆分和治理脚本对齐同一套理解。这里不替代用户故事、ADR、RTM、OpenAPI 或数据库文档；正式实现和验收仍以这些事实源为准。

## 适用范围

- M4 出库订单管理、波次规划、PDA 拣选、复核、打印、发货交接和采购退货出库。
- 与出库强相关的 M1 系统字典、M-CG 单据号、M3 库存、M-TE 任务、M-VR 双人策略、M-TC 追溯码、M10 / H5 配送、H1 权限和 H2 审计。
- PC Web 页面只按管理型后台承载订单、波次、复核发货和采购退货入口；PDA 离线和扫码细节仍以 PDA 故事为准。

## 关联文档

- [用户故事：M4 出库（订单与波次）](../../domain/user-stories-m4-outbound-order.md)
- [用户故事：M4 出库（拣选、复核与发货）](../../domain/user-stories-m4-outbound-pick.md)
- [用户故事：M4 出库（退货处理）](../../domain/user-stories-m4-outbound-return.md)
- [M4 PC Web 出库页面设计规划](../../m4-outbound-web-design-plan.md)
- [需求追溯矩阵](../../requirements-traceability-matrix.md)
- [业务澄清记录 C8](../../domain/clarifications.md)
- [M-CG 单据号生成图文说明](../mcg-document-numbering/docu.md)

## 图清单

| 图 | 说明 | 适合回答的问题 |
|---|---|---|
| [order-state.puml](order-state.puml) | 出库订单从待校验到签收 / 作废的状态流转 | 出库订单有哪些状态，哪些节点允许作废或回退 |
| [fulfillment-activity.puml](fulfillment-activity.puml) | 出库主作业链路：校验、波次、拣选、复核、打印、交接、扣减和 ERP 反馈 | 一张出库单如何完成真实发货，跨模块如何衔接 |
| [purchase-return-outbound.puml](purchase-return-outbound.puml) | 采购退货出库申请、审批、拣货、复核和交接 | 采购退货为什么归 M4，和销售退货有什么边界 |

## 如何读图

- 先看 `order-state.puml`：它表达订单主状态，不展开每个作业明细。
- 再看 `fulfillment-activity.puml`：它表达作业顺序和跨模块边界，尤其是 M-CG、M-VR、M-TC、M10 / H5、H2 的接入点。
- 最后看 `purchase-return-outbound.puml`：它只整理退供应商的采购退货出库；销售退货 / 客户退货按 C8 归 M2 入库。

## 关键约束

- `document_type` 来源于 M1 系统字典；M4 只消费 `direction = outbound` 的启用字典项。
- WMS 出库单号和采购退货单号由 M-CG 统一生成，业务模块不得自行拼号。
- 出库订单的商品、批号和数量由 ERP 指定；WMS 校验合格库存并在波次阶段决定具体库位。
- 波次分配时锁定库存，发货交接确认时正式扣减库存。
- 短拣可以继续拣选 / 复核，但发货前必须补齐；一个订单不允许部分发货。
- 复核无“不通过”状态；发现问题按故事记录差异，并走后续退回流程。
- 拣货、复核、装箱、发货交接等节点按 M-VR 查询双人策略；特殊药品默认值以 M1-010 / M-VR 规则为准。
- M-VR 查询必须发生在对应作业提交前；图中节点顺序必须与用户故事的“前 / 后 / 时”约束一致。
- 随货同行单必须使用货主抬头；冷链发货交接需记录装车温度和保温箱 / 冰袋配置。
- 所有写操作需要 H1 权限、Idempotency-Key 和 H2 审计；审计表只追加，不更新或删除。
- 采购退货出库不把批号作为申请、拣货或复核字段；审批源为 `purchase_return_approval`。

## 当前实现提醒

- RTM 已记录 M4 后端闭环覆盖，前端 M4 PC 页面仍有列表 / 详情 / 写操作接真实 API 的后续项。
- 本图只沉淀流程理解；如果后续新增字段、状态、接口、审批源或单据类型，先改用户故事、RTM、OpenAPI / 数据库文档，再同步本目录。

## 维护规则

- 修改 `.puml` 时必须同步检查本文的图清单、关键约束和关联文档。
- 不把图中的节点当作新事实源；图中任何新增语义都必须能在用户故事、澄清记录、RTM 或代码中追溯。
- 前端页面实现时，按 [M4 PC Web 出库页面设计规划](../../m4-outbound-web-design-plan.md) 的字段 / 动作 / 状态 / 证据 RTM 验证。

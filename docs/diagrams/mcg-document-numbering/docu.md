# M-CG 单据号生成图文说明

## 目的

本文配套说明 M-CG 单据号生成模块的 PlantUML 图，帮助后续设计、开发、review 和治理脚本对齐同一套理解。这里不替代用户故事、ADR、RTM、OpenAPI 或数据库文档；正式实现前仍需要同步这些事实源。

## 适用范围

- M-CG 编码生成模块。
- M2 入库、M4 出库、M3 库内业务以及后续需要 WMS 单据号的业务模块。
- 与单据号生成相关的 M1 `document_type` 系统字典、H1 权限、H2 审计、PostgreSQL 事务。

## 关联文档

- [用户故事：M-CG 编码生成模块](../../domain/user-stories-mcg-code-generator.md)
- [US-M1-011：系统字典中心](../../domain/user-stories-m1-master-data-warehouse.md)
- [数据库设计与命名规范](../../database/database-design-standards.md)
- [数据库设计审查矩阵](../../database/database-design-review.md)
- [M2 入库 Web 设计方案](../../m2-inbound-web-design-plan.md)
- [M4 出库 Web 设计方案](../../m4-outbound-web-design-plan.md)

## 图清单

| 图 | 说明 | 适合回答的问题 |
|---|---|---|
| [component.puml](component.puml) | M-CG 与 M1、M2、M3、M4、H1、H2、PostgreSQL 的模块关系 | 单据号能力放在哪个模块，谁能调用它 |
| [create-document-sequence.puml](create-document-sequence.puml) | 创建业务单据时，业务模块如何向 M-CG 申请单据号 | 请求如何流转，幂等和审计在哪里发生 |
| [no-gap-activity.puml](no-gap-activity.puml) | no-gap 生成流程和异常分支 | 如何保证不跳号，失败时如何处理 |
| [data-model.puml](data-model.puml) | 建议的数据模型关系 | 规则、计数器、生成记录和系统字典如何关联 |

## 如何读图

- `component.puml` 先看模块边界：M-CG 是横向平台能力，业务模块只申请号码，不拼接号码。
- `create-document-sequence.puml` 再看调用顺序：幂等检查优先，字典校验在生成前，counter 行锁和生成记录在事务中完成。
- `no-gap-activity.puml` 看异常路径：字典无效、规则不存在、流水号溢出都必须拒绝创建，不能降级成临时号码。
- `data-model.puml` 看表职责：规则表定义格式，counter 表负责递增，allocation 表负责生成记录、幂等与请求冲突识别。

## 关键约束

- `document_type` 的事实源是 M1 系统字典，M-CG 只消费启用、生效、作用域匹配且参数有效的字典项。
- 单据号统一由 M-CG 生成，M2、M3、M4 等业务模块不得自行拼接。
- 本次讨论确认当前设计采用全部 no-gap：所有 WMS 单据号都要连续不跳号。
- 取消或作废单据时，不回收、不复用、不修改已生成号码；业务单据状态应标记为取消或作废，并保留审计链。
- no-gap 生成必须和业务单据写入形成事务闭环；如果业务单据写入失败，counter 不能前进。
- 第一切片通过内部 `generate_in_tx` 支持业务模块复用当前事务；未新增公开 HTTP API。
- counter 锁竞争要靠 `counter_key` 拆分控制，建议按货主、单据类型、重置周期拆分。
- 编码规则变更只影响新单据，不能改变历史单据号解释。

## 建议模板

```text
{OWNER}{DOCUMENT_TYPE}{YYYY}{MM}{DD}{SEQ:5}
```

示例：

```text
HZ001-ASN-P-20260701-00001
```

## 后续同步项

- 继续补 M2/M3/M4 创建单据接入，禁止业务模块自行拼接单据号。
- 补治理脚本：检查 M2/M4/M3 创建单据时不得自行拼接单据号。
- 规则管理 API、生成记录查询 API 已落到 code-generator OpenAPI；后续扩展时继续补对应后端测试并同步 api-client。
- 补前端设计：M1 单据号规则管理页、预览规则、生成记录查询。

## 维护规则

- 修改 `.puml` 时必须同步检查本文件的图清单和关键约束。
- 如果图中出现新的字段、状态、模块、审批源或默认值，先同步用户故事、ADR 或 RTM。
- 只把图用于解释已确认设计；不要把图当成绕过治理文档的新事实源。

# 全链路质量矩阵

> 本文件由 `governance/quality-matrix.toml` 生成。不要手工改表格；修改事实源后运行 `python3 scripts/governance/check_quality_matrix.py --write-doc`。

## 范围

- 强门禁范围：M1、M2、M3、M4 和已进入执行的 H 层横向能力。
- 状态只允许 `verified` 或 `not_applicable`；不适用必须在事实源写原因。
- S2 测试层由故事类型自动推导。

## 矩阵

| 故事 | 模块 | 类型 | 测试层 | 前端 | API | 状态 |
|---|---|---|---|---|---|---|
| US-M1-011 系统字典中心 | M1 | write、config_rule、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m1-system-dictionary | GET /api/v1/system-dictionaries/{dict_code}/items<br>PUT /api/v1/system-dictionaries/{dict_code}/items/{item_code}<br>PATCH /api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-007 PC 管理端三层菜单管理 | H1 | write、config_rule、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h1-menu-management | GET /api/v1/admin/menus/draft<br>POST /api/v1/admin/menus/draft/nodes<br>PATCH /api/v1/admin/menus/draft/nodes/{id}<br>POST /api/v1/admin/menus/draft/batch-enable<br>POST /api/v1/admin/menus/publish<br>GET /api/v1/admin/menus/published<br>POST /api/v1/admin/menus/rollback | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-001 OpenAPI 契约生成与前端类型同步 | H3 | api_change | L2、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-001 打印模板类型字典 | H9 | read_only、config_rule、frontend_interaction | L1、L2、L3、L4、L7、L8、L9 | m1-system-dictionary、h9-print-templates | GET /api/v1/system-dictionaries/{dict_code}/items<br>PUT /api/v1/system-dictionaries/{dict_code}/items/{item_code}<br>PATCH /api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-002 字段库生成与字段元数据维护第一切片 | H9 | read_only、api_change、frontend_interaction | L1、L2、L3、L7、L8、L9 | h9-print-templates | GET /api/v1/print-templates/field-libraries | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M2-002 PDA/PC Web 收货 | M2 | write、inventory_change、frontend_interaction、critical_path | L1、L2、L3、L4、L5、L6、L7、L8、L9、L10、L11 | m2-receiving | POST /api/v1/inbound/receiving-orders/{id}/receive | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M3-002 批次与效期管理 | M3 | read_only、frontend_interaction | L1、L2、L3、L7、L8 | m3-batches | GET /api/v1/inventory/batches | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M4-001 出库订单管理 | M4 | write、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m4-orders | GET /api/v1/outbound/orders<br>POST /api/v1/outbound/orders<br>GET /api/v1/outbound/orders/{id} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |

## 明确延期范围

| 故事 | 模块 | 原因 |
|---|---|---|
| US-H9-003 模板设计与版本管理 | H9 | H9 当前闭环只覆盖模板类型字典和字段库第一切片；模板主数据、hiprint JSON 版本、发布流程和删除限制按 ADR-0036 后续切片实现。 |
| US-H9-004 预览与浏览器打印 | H9 | 浏览器预览、打印记录、补打和设备失败策略尚未进入当前实现；按 H9 用户故事和 ADR-0036 后续切片接入。 |
| US-H9-005 业务模块接入规则 | H9 | M2/M4/标签等业务打印接入规则需要在模板版本和预览打印能力完成后落地；当前只登记延期范围。 |

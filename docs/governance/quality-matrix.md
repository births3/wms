# 全链路质量矩阵

> 本文件由 `governance/quality-matrix.toml` 生成。不要手工改表格；修改事实源后运行 `python3 scripts/governance/check_quality_matrix.py --write-doc`。

## 范围

- 首版强门禁范围：M1、M2、M3、M4。
- 状态只允许 `verified` 或 `not_applicable`；不适用必须在事实源写原因。
- S2 测试层由故事类型自动推导。

## 矩阵

| 故事 | 模块 | 类型 | 测试层 | 前端 | API | 状态 |
|---|---|---|---|---|---|---|
| US-M1-011 系统字典中心 | M1 | write、config_rule、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m1-system-dictionary | GET /api/v1/system-dictionaries/{dict_code}/items<br>PUT /api/v1/system-dictionaries/{dict_code}/items/{item_code}<br>PATCH /api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M2-002 PDA/PC Web 收货 | M2 | write、inventory_change、frontend_interaction、critical_path | L1、L2、L3、L4、L5、L6、L7、L8、L9、L10、L11 | m2-receiving | POST /api/v1/inbound/receiving-orders/{id}/receive | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M3-002 批次与效期管理 | M3 | read_only、frontend_interaction | L1、L2、L3、L7、L8 | m3-batches | GET /api/v1/inventory/batches | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M4-001 出库订单管理 | M4 | write、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m4-outbound-orders | GET /api/v1/outbound/orders<br>POST /api/v1/outbound/orders<br>GET /api/v1/outbound/orders/{id} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |

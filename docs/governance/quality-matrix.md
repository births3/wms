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
| US-H1-001 用户登录与 token 签发（PC 密码登录切片） | H1 | write、api_change、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | - | POST /api/v1/auth/login<br>GET /api/v1/auth/me | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-003 API 鉴权中间件 | H1 | read_only、api_change | L1、L2、L3、L8、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-004 多货主数据隔离（AuthContext 切片） | H1 | read_only、api_change | L1、L2、L3、L8、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-007 PC 管理端三层菜单管理 | H1 | write、config_rule、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h1-menu-management | GET /api/v1/admin/menus/draft<br>POST /api/v1/admin/menus/draft/nodes<br>PATCH /api/v1/admin/menus/draft/nodes/{id}<br>POST /api/v1/admin/menus/draft/batch-enable<br>POST /api/v1/admin/menus/publish<br>GET /api/v1/admin/menus/published<br>POST /api/v1/admin/menus/rollback | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-001 审计事件统一记录（同步 append-only 切片） | H2 | write、audit_compliance | L1、L2、L3、L4、L5、L8、L10、L11 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:not_applicable<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-002 审计追踪查询（后端分页查询切片） | H2 | read_only、api_change、frontend_interaction | L1、L2、L3、L7、L8、L9 | h2-audit-trail | GET /api/v1/audit/events | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-003 append-only 不变量保护 | H2 | audit_compliance | L5、L8、L10、L11 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:not_applicable<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-004 审计数据归档与保留（后端归档运行切片） | H2 | write、audit_compliance | L1、L2、L3、L4、L5、L8、L10、L11 | - | GET /api/v1/audit/archive/partitions<br>POST /api/v1/audit/archive/runs | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-005 事件总线（DB outbox 与订阅投递切片） | H2 | write、integration、audit_compliance | L1、L2、L3、L4、L5、L8、L9、L10、L11 | - | GET /api/v1/event-bus/deliveries/pending<br>POST /api/v1/event-bus/deliveries/{delivery_id}/ack<br>POST /api/v1/event-bus/deliveries/{delivery_id}/nack | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-006 业务数据生命周期归档（后端策略与计划切片） | H2 | write、config_rule、audit_compliance | L1、L2、L3、L4、L5、L8、L9、L10、L11 | - | GET /api/v1/business-retention/policies<br>POST /api/v1/business-retention/jobs | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-001 OpenAPI 契约生成与前端类型同步 | H3 | api_change | L2、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-002 前端 TS 类型生成 | H3 | api_change | L2、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-003 API 限流与熔断 | H3 | api_change、runtime_guard、config_rule、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | - | GET /api/v1/resilience/status<br>GET /metrics | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-004 API 文档可访问性 | H3 | read_only、api_change、config_rule、frontend_interaction | L1、L2、L3、L4、L7、L8、L9 | h3-api-contract | GET /openapi.json<br>GET /api-docs<br>GET /redoc | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H4-001 通知配置 | H4 | write、config_rule、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h4-notify-configs、h4-wechat-settings | GET /api/v1/wechat-notify/configs<br>POST /api/v1/wechat-notify/configs | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H4-004 通知发送记录查询 | H4 | read_only、write、api_change、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h4-notify-records | GET /api/v1/wechat-notify/records<br>POST /api/v1/wechat-notify/records/{record_id}/resend | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-001 快递商配置 | H5 | write、config_rule、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express | GET /api/v1/express/carriers<br>POST /api/v1/express/carriers | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-002 快递选择规则配置 | H5 | write、config_rule、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express | GET /api/v1/express/routing-rules<br>POST /api/v1/express/routing-rules | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-003 快递单打印 | H5 | write、frontend_interaction、api_change、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express、h9-print-templates | POST /api/v1/packing/jobs/{id}/waybill<br>POST /api/v1/express/waybills | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-004 快递推送下单 | H5 | write、integration、runtime_guard、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express | POST /api/v1/express/waybills<br>POST /api/v1/express/waybills/{waybill_no}/cancel | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-005 快递轨迹查询 | H5 | read_only、integration、frontend_interaction、api_change | L1、L2、L3、L4、L7、L8、L9、L10 | h5-express | GET /api/v1/express/waybills/{waybill_no}/tracking | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H6-001 状态机定义注册与转换校验 | H6 | read_only、api_change、config_rule | L1、L2、L3、L4、L8、L9 | - | GET /api/v1/state-machines<br>GET /api/v1/state-machines/{machine_code}<br>GET /api/v1/state-machines/{machine_code}/transition-validation | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-001 打印模板类型字典 | H9 | read_only、config_rule、frontend_interaction | L1、L2、L3、L4、L7、L8、L9 | m1-system-dictionary、h9-print-templates | GET /api/v1/system-dictionaries/{dict_code}/items<br>PUT /api/v1/system-dictionaries/{dict_code}/items/{item_code}<br>PATCH /api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-002 字段库生成与字段元数据维护第一切片 | H9 | read_only、api_change、frontend_interaction | L1、L2、L3、L7、L8、L9 | h9-print-templates | GET /api/v1/print-templates/field-libraries | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-003 模板设计与版本管理 | H9 | write、config_rule、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h9-print-templates | GET /api/v1/print-templates/templates<br>POST /api/v1/print-templates/templates<br>GET /api/v1/print-templates/field-libraries/{version_id}/fields | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-004 预览与浏览器打印 | H9 | write、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h9-print-templates | POST /api/v1/print-templates/preview<br>POST /api/v1/print-templates/print | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-005 业务模块接入规则 | H9 | read_only、frontend_interaction、api_change | L1、L2、L3、L7、L8、L9 | h9-print-templates | POST /api/v1/print-templates/resolve | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M2-002 PDA/PC Web 收货 | M2 | write、inventory_change、frontend_interaction、critical_path | L1、L2、L3、L4、L5、L6、L7、L8、L9、L10、L11 | m2-receiving | POST /api/v1/inbound/receiving-orders/{id}/receive | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M3-002 批次与效期管理 | M3 | read_only、frontend_interaction | L1、L2、L3、L7、L8 | m3-batches | GET /api/v1/inventory/batches | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M4-001 出库订单管理 | M4 | write、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m4-orders | GET /api/v1/outbound/orders<br>POST /api/v1/outbound/orders<br>GET /api/v1/outbound/orders/{id} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |

## 明确延期范围

| 故事 | 模块 | 原因 |
|---|---|---|
| US-H1-002 角色与权限管理 | H1 | 当前只落地角色、权限、用户绑定底层表和登录态权限读取；角色权限维护 API / UI、批量授权和权限即时变更操作未进入本轮执行。 |
| US-H1-005 Token 失效与登出 | H1 | 当前只落地 Redis token 黑名单和 permissions_changed_at 运行策略；登出 API、多设备会话列表和管理员强制踢人未进入本轮执行。 |
| US-H1-006 API Key 生命周期管理 | H1 | 当前没有 API Key 创建、轮换、吊销、作用域和审计接口；该能力应与 H-INT 外部系统接入切片一起设计和实现。 |
| US-H4-002 企业微信消息发送 | H4 | 当前仅完成参数维护、本地完整性测试、模板渲染和发送记录持久化；未接外部 provider 时记录明确为失败，禁止伪报发送成功。企业微信真实 API 调用、失败重试、token 刷新和外部联调证据尚未完成。 |
| US-H4-003 企业微信审批流对接 | H4 | 当前仅完成受 JWT 权限保护的内部审批记录和指定审批人回写模型；企业微信审批推送、外部回调签名校验、轮询兜底、业务单据原子回写和外部联调证据尚未完成。 |
| US-AL-001 告警定义注册 | AL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-AL-002 告警触发与生命周期 | AL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-AL-003 告警升级机制 | AL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-AL-004 告警看板与统计 | AL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-AL-005 告警通道与静默配置 | AL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DOCK-001 月台档案管理 | DOCK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DOCK-002 预约创建 | DOCK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DOCK-003 预约时间窗冲突检测 | DOCK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DOCK-004 预约变更与取消 | DOCK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DOCK-005 实到对账 | DOCK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DOCK-006 月台占用看板 | DOCK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DOCK-007 预约统计与报表 | DOCK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DR-001 司机端登录与今日任务列表 | DR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DR-002 司机签收（自有车队交接确认） | DR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DR-003 在途异常上报 | DR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DR-004 司机端到店签收（客户签字） | DR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DR-005 司机端工作日志与绩效查询 | DR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-ST-001 门店登录与首页 | ST | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-ST-002 门店订货与补货请求 | ST | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-ST-003 门店签收（电子签收） | ST | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-ST-004 门店退货发起 | ST | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-ST-005 门店订单查询与历史 | ST | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-ST-006 门店冷链订单温控查询（GSP 合规专项） | ST | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-001 商品档案管理 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-002 供应商资质档案 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-003 客户/门店档案 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-004 仓库与库位管理 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-005 客商开票信息 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-006 用户与权限管理 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-007 多货主架构 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-008 系统配置中心 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-009 多仓管理 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-010 特殊药品分类字典管理 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M10-001 接收 TMS 路径规划结果 | M10 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M10-002 在途温控数据接入 | M10 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M10-003 容器回收追踪 | M10 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M10-004 出库订单与 TMS 协作 | M10 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M11-000 占位（v7 移除） | M11 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-001 创建 ASN（采购入库通知） | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-003 PDA/PC Web 验收 | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-004 双人验收签字 | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-005 PDA/PC Web 智能上架 | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-006 收货异常处理 | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-007 收货单据打印 | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-008 收货进度看板 | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-009 打印模板设计模块（迁移引用） | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-010 上架策略配置 | M2 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-004 在库养护 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-005 库存盘点 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-006 库内移库 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-009 库存预警 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-010 ABC 分类管理 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-011 库位历史追踪 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-001 实时库存查询 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M3-003 库存状态管理 | M3 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-002 波次规划 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-003 PDA 拣选作业 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-004 出库复核 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-005 随货同行单打印 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-006 发货与交接 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-007 出库进度看板 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-008 销售退货入库处理（历史编号，归口 M2） | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-009 波次出库箱规则 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-010 拣选路径策略 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-011 合并发货与拆单 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-012 采购退货出库 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M5-001 冷链设备台账 | M5 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M5-002 温控数据接入与展示 | M5 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M5-003 温度超标事件接入与联动 | M5 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M6-001 GSP 法定台账自动生成 | M6 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M6-002 审计追踪查询 | M6 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M6-003 业务报表 | M6 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M6-004 特殊管理药品专用台账 | M6 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M8-001 自动补货算法（门店水位） | M8 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M8-002 越库作业（Cross-Docking） | M8 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M9-001 计费规则配置 | M9 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M9-002 自动计费 | M9 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M9-003 账单管理 | M9 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-001 创建批号调整单 | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-002 批号调整双人审批 | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-003 PDA 执行批号调整（实物核对 + 库存与追溯码联动） | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-004 ERP 反馈批号调整结果 | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-CG-001 编码规则配置 | CG | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-CG-002 编码生成服务 | CG | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DI-001 药检单平台对接配置 | DI | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DI-002 扫码批量查询药检单 | DI | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DI-003 药检单存储与查看 | DI | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-DI-004 药检单有效性校验 | DI | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-PK-001 包装站工位管理 | PK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-PK-002 装箱作业 | PK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-PK-003 称重校验 | PK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-PK-004 保温箱配置（冷链） | PK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-PK-005 追溯码出库核验 | PK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-PK-006 快递面单打印 | PK | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-MPM-001 参数对照字典定义 | MPM | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-MPM-002 映射规则配置 | MPM | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-MPM-003 待映射队列与质量联系单触发 | MPM | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-MPM-004 映射执行 API | MPM | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-MPM-005 反向追溯与映射溯源 | MPM | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-MPM-006 交易类型字典管理（PIX 三码） | MPM | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-QL-001 质量联系单类型与审批模板配置 | QL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-QL-002 创建质量联系单 | QL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-QL-003 质量联系单审批 | QL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-QL-004 质量联系单与业务流程联动 | QL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-QL-005 质量联系单查询与统计 | QL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RC-001 定时对账任务 | RC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RC-002 差异处理 | RC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RC-003 手动触发对账 | RC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RC-004 对账报表 | RC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RP-001 补货阈值管理 | RP | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RP-002 日常补货（阈值触发） | RP | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RP-003 波次补货（紧急触发） | RP | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RP-004 拣选补拣（兜底触发） | RP | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-RP-005 补货记录与报表 | RP | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-SA-001 报损单管理 | SA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-SA-002 报溢单管理 | SA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-SA-003 报损报溢查询与统计 | SA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-001 追溯码分类管理 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-002 码库管理（大中小码） | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-003 追溯码与商品绑定 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-004 PDA 追溯码对应关系维护 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-005 追溯码录入环节配置 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-006 出库追溯码核验 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-007 "码上放心"平台上报 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-001 任务类型配置 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-002 任务组与人员资格 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-003 任务创建（各模块触发） | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-004 任务优先级规则 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-005 任务分配 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-006 任务释放控制 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-007 任务合并 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-008 任务执行（PDA） | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-009 任务跟踪看板 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-010 任务绩效统计 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-011 设备接口抽象（RFID/AGV/输送线预留） | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-001 校验规则配置 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-002 校验规则执行 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-003 校验异常人工处理 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-004 校验规则模板 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-005 校验规则测试 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-006 双人策略矩阵规则 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |

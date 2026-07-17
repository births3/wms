# 全链路质量矩阵

> 本文件由 `governance/quality-matrix.toml` 生成。不要手工改表格；修改事实源后运行 `python3 scripts/governance/check_quality_matrix.py --write-doc`。

## 范围

- 强门禁范围：M1、M2、M3、M4 和已进入执行的 H 层横向能力。
- 状态只允许 `verified` 或 `not_applicable`；不适用必须在事实源写原因。
- S2 测试层由故事类型自动推导。
- 验收深度由故事类型自动推导：S1 查询/展示，S2 普通写操作，S3 库存/并发/关键路径/GSP，S4 PDA/硬件/外部系统/发布。

## 状态摘要

| 指标 | 数量 |
|---|---:|
| 故事总数 | 174 |
| 已完成（已验证） | 45 |
| 未完成 / 延期 | 129 |
| 完成率 | 25.9% |

> “已完成”表示故事已进入 `stories` 并通过矩阵维度门禁；延期故事中的局部代码、页面或测试切片不计入完成。

## 模块进度

| 模块 | 已完成 | 未完成 / 延期 | 总数 |
|---|---:|---:|---:|
| AL | 3 | 2 | 5 |
| BA | 0 | 4 | 4 |
| CG | 0 | 2 | 2 |
| DI | 0 | 4 | 4 |
| DOCK | 1 | 6 | 7 |
| DR | 0 | 5 | 5 |
| H1 | 7 | 0 | 7 |
| H2 | 6 | 0 | 6 |
| H3 | 4 | 0 | 4 |
| H4 | 2 | 2 | 4 |
| H5 | 5 | 0 | 5 |
| H6 | 1 | 0 | 1 |
| H9 | 5 | 0 | 5 |
| M1 | 2 | 9 | 11 |
| M10 | 0 | 4 | 4 |
| M2 | 2 | 7 | 9 |
| M3 | 2 | 7 | 9 |
| M4 | 1 | 10 | 11 |
| M5 | 0 | 3 | 3 |
| M6 | 0 | 4 | 4 |
| M8 | 0 | 2 | 2 |
| M9 | 0 | 3 | 3 |
| MPM | 0 | 6 | 6 |
| PK | 0 | 6 | 6 |
| QL | 0 | 5 | 5 |
| RC | 0 | 4 | 4 |
| RP | 0 | 5 | 5 |
| SA | 0 | 3 | 3 |
| ST | 0 | 6 | 6 |
| TC | 0 | 7 | 7 |
| TE | 4 | 7 | 11 |
| VR | 0 | 6 | 6 |

## 已完成故事

| 故事 | 模块 | 验收层级 |
|---|---|---|
| US-M1-011 系统字典中心 | M1 | S2 |
| US-H1-005 Token 失效与登出 | H1 | S2 |
| US-H1-006 API Key 生命周期管理 | H1 | S2 |
| US-M1-004 仓库与库位管理 | M1 | S2 |
| US-M3-001 实时库存查询 | M3 | S2 |
| US-M3-011 库位历史追踪 | M3 | S2 |
| US-H1-001 用户登录与 token 签发（PC 密码登录切片） | H1 | S2 |
| US-H1-002 角色与权限管理 | H1 | S2 |
| US-H1-003 API 鉴权中间件 | H1 | S1 |
| US-H1-004 多货主数据隔离（AuthContext 切片） | H1 | S1 |
| US-H1-007 PC 管理端三层菜单管理 | H1 | S2 |
| US-H2-001 审计事件统一记录（同步 append-only 切片） | H2 | S3 |
| US-H2-002 审计追踪查询（后端分页查询切片） | H2 | S1 |
| US-H2-003 append-only 不变量保护 | H2 | S3 |
| US-H2-004 审计数据归档与保留（后端归档运行切片） | H2 | S3 |
| US-H2-005 事件总线（DB outbox 与订阅投递切片） | H2 | S3 |
| US-H2-006 业务数据生命周期归档（后端策略与计划切片） | H2 | S3 |
| US-H3-001 OpenAPI 契约生成与前端类型同步 | H3 | S1 |
| US-H3-002 前端 TS 类型生成 | H3 | S1 |
| US-H3-003 API 限流与熔断 | H3 | S3 |
| US-H3-004 API 文档可访问性 | H3 | S1 |
| US-H4-001 通知配置 | H4 | S3 |
| US-H4-004 通知发送记录查询 | H4 | S3 |
| US-H5-001 快递商配置 | H5 | S3 |
| US-H5-002 快递选择规则配置 | H5 | S3 |
| US-H5-003 快递单打印 | H5 | S3 |
| US-H5-004 快递推送下单 | H5 | S3 |
| US-H5-005 快递轨迹查询 | H5 | S2 |
| US-H6-001 状态机定义注册与转换校验 | H6 | S1 |
| US-H9-001 打印模板类型字典 | H9 | S1 |
| US-H9-002 字段库生成与字段元数据维护第一切片 | H9 | S1 |
| US-H9-003 模板设计与版本管理 | H9 | S2 |
| US-H9-004 预览与浏览器打印 | H9 | S2 |
| US-H9-005 业务模块接入规则 | H9 | S1 |
| US-M2-002 PDA/PC Web 收货 | M2 | S3 |
| US-M2-008 收货进度看板 | M2 | S1 |
| US-M4-001 出库订单管理 | M4 | S2 |
| US-AL-001 告警定义注册 | AL | S3 |
| US-AL-003 告警升级机制 | AL | S3 |
| US-AL-004 告警看板与统计 | AL | S3 |
| US-DOCK-001 月台档案管理 | DOCK | S2 |
| US-TE-001 任务类型配置 | TE | S2 |
| US-TE-002 任务组与人员资格 | TE | S3 |
| US-TE-004 任务优先级规则 | TE | S3 |
| US-TE-006 任务释放控制 | TE | S3 |

## 未完成 / 延期故事

| 故事 | 模块 | 当前原因 |
|---|---|---|
| US-H4-002 企业微信消息发送 | H4 | 已完成可注入企业微信 provider 边界、模板渲染、批量收件人记录、幂等最终结果更新、成功/可重试失败/永久失败状态、发送审计和参数完整性测试；未接外部 provider 时明确记录失败，禁止伪报发送成功。企业微信真实 HTTP API 调用、Secret alias 解析、access_token 获取与刷新、自动重试调度和外部联调证据尚未完成。 |
| US-H4-003 企业微信审批流对接 | H4 | 当前仅完成受 JWT 权限保护的内部审批记录和指定审批人回写模型；企业微信审批推送、外部回调签名校验、轮询兜底、业务单据原子回写和外部联调证据尚未完成。 |
| US-M3-003 库存状态管理 | M3 | 已完成状态变更 API、幂等、审计、PC 弹窗和隔离库存出库门禁；状态转换规则的全局/货主覆盖维护 API、状态字典校验、幂等、审计及 PC 配置页已闭环，真实 PostgreSQL 浏览器 E2E 已验证货主覆盖规则保存和本地门户截图；截图需由 PR 附件或 CI artifact 长期归档。移库门禁、调度接线、ERP 异步反馈重试及外部系统证据仍未闭环。 |
| US-M3-002 批次与效期管理 | M3 | 已完成批号查询、近效期查询/报表、PC 风险筛选、过期隔离、追溯、召回及双人取消召回，并将 180 天预警默认值和货主覆盖落到 inventory_policy 字典；真实 PostgreSQL 浏览器 E2E 已验证批次追溯、状态变更、召回/取消召回和本地门户截图，截图需由 PR 附件或 CI artifact 长期归档。调度与 H4 预警联动、已发货客户回收提示和完整外部通知证据仍未闭环。 |
| US-AL-002 告警触发与生命周期 | AL | 已完成 H2 事件订阅、JSON 条件匹配、静默去重、H4 重试与失败二级告警、生命周期状态机、PC 确认/处理/关闭/忽略、业务解除和 7 天自动关闭、权限、审计、OpenAPI、PostgreSQL 测试及真实 PC E2E。受 ADR-0027 Proposed 约束，生产 PDA 应用禁止启动，企微/PDA 点击确认、离线暂存恢复和真机证据尚不能补齐，因此不得整体关闭。 |
| US-AL-005 告警通道与静默配置 | AL | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
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
| US-M1-001 商品档案管理 | M1 | 已完成商品批量 API 同步后端切片及管理端批量导入解析预览：整批 PostgreSQL 事务、重复幂等回放、货主隔离、受控储存条件/特殊药品字典校验、API Key 中间件鉴权、请求审计和原子回滚均有真实 PostgreSQL 回归证据；管理端批量导入已直接调用原子批量接口，并由一次性 PostgreSQL 浏览器 E2E 验证两条请求、解析预览、页面回显和本地门户截图；截图按门禁不入库，长期证据仍须由 PR 附件或 CI artifact 归档；故事仍不能整体关闭，原生 xlsx 文件解析、M-PM 未匹配项展示、ERP 回写、PDA 和完整字段映射证据尚未闭环。 |
| US-M1-002 供应商资质档案 | M1 | 已完成供应商批量 API 同步后端切片：整批 PostgreSQL 事务、重复幂等回放/冲突、货主隔离、API Key 中间件鉴权、请求审计和原子回滚均有真实 PostgreSQL 回归证据；供应商创建、批量同步和更新入口现统一执行 GB 32100 字符集、长度、校验码与大写归一化校验，前后端共用同一算法语义并覆盖 422 契约；管理端批量导入已直接调用原子批量接口，并由一次性 PostgreSQL 浏览器 E2E 验证两条请求、页面回显和本地门户截图；截图按门禁不入库，长期证据仍须由 PR 附件或 CI artifact 归档；故事仍不能整体关闭，资质证照有效期/上传、经营范围标签、到期预警、过期供应商收货拦截、印章及样张备案、完整供应商更新/查询、PDA、Excel 原生导入和完整外部系统联调尚未闭环。 |
| US-M1-003 客户/门店档案 | M1 | 已补客户多地址及客户/门店联系方式、经营范围、资质证照、连锁归属的 PostgreSQL 读写、默认地址互斥、货主隔离、幂等和审计，并新增客户批量同步 API：整批事务、重复幂等回放/冲突、货主隔离、API Key 中间件鉴权、请求审计和原子回滚有真实 PostgreSQL 回归证据；管理端批量导入已直接调用原子批量接口，并由一次性 PostgreSQL 浏览器 E2E 验证两条请求、页面回显和本地门户截图；截图按门禁不入库，长期证据仍须由 PR 附件或 CI artifact 归档；仍缺 Excel 原生导入与完整外部系统联调，不能标记完成。 |
| US-M1-005 客商开票信息 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-006 用户与权限管理 | M1 | 已补用户创建后端切片、生产菜单和管理端弹窗/API 接线：用户名/姓名/手机号/密码/角色校验、货主隔离、多角色原子绑定、幂等、密码哈希、审计、真实浏览器创建用户与角色绑定和本地门户截图均有证据，并已登记 OpenAPI；截图需由 PR 附件或 CI artifact 长期归档。仍缺完整角色/用户/权限矩阵验收和发布证据，不能标记完成。 |
| US-M1-007 多货主架构 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-008 系统配置中心 | M1 | 已补 Feature Flag 配置中心 PostgreSQL 导入/导出、货主隔离、事务内审计和真实数据库重载验证，并将配置中心源文件拆分至 539 行；真实 PostgreSQL 浏览器 E2E 已验证迁移操作、页面回显和本地门户截图，截图需由 PR 附件或 CI artifact 长期归档。仍缺完整配置项分组/校验/变更历史、前端所有操作真实 API 接线、审批/回滚和发布证据，不能标记完成。 |
| US-M1-009 多仓管理 | M1 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M1-010 特殊药品分类字典管理 | M1 | 已补齐特殊药品字典 8 类预置、合规属性矩阵、通用字典维护、自定义分类消费，并由数据库触发器与 M-VR 双人策略规则双向同步，矩阵 UI 已通过真实浏览器 E2E；仍缺分类字段/合规属性修改的仓库主管+质量负责人审批、M-PM 枚举同步、专用台账与专柜专库运行联动及完整 GSP 回查证据，禁止标记完成。 |
| US-M10-001 接收 TMS 路径规划结果 | M10 | 已补 TMS 路径规划结果接收后端切片、正式数据库菜单发布迁移、E2E 运行入口的 Wave 5 路由、PC 接收页面和页面级 self-check；路线/站点/ETA/订单完整性校验、货主与司机隔离、幂等冲突、审计、PostgreSQL 证据和真实浏览器 E2E/截图已有。仍缺 H8 外部 TMS 真实联调、仓库主管查询界面、版本递增覆盖、TMS 不可用时人工录入与审批和正式发布证据，不能标记完成。 |
| US-M10-002 在途温控数据接入 | M10 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M10-003 容器回收追踪 | M10 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M10-004 出库订单与 TMS 协作 | M10 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M2-001 创建 ASN（采购入库通知） | M2 | 已补创建契约切片：领域层和内存/PostgreSQL 创建入口统一要求供应商、非历史预计到货日期、商品编码非空且采购 ASN 只能对应一个商品；真实 PostgreSQL 浏览器 E2E 已验证 PC 创建、放行、收货、打印数据绑定、页面回显和本地门户截图，截图需由 PR 附件或 CI artifact 长期归档。仍缺供应商本地资质与商品存在性/货主校验、M-VR 初始状态接线、完整单号并发唯一性、开放 API/API Key、作废企业微信审批、完整审计和物理打印证据。 |
| US-M2-003 PDA/PC Web 验收 | M2 | 已补验收累计量、零数量、签字前总量闭合和逐明细验收门禁，并完成 PC Web 按商品/货主/仓库实时查询 M-VR 入库/验收策略、服务端防降级、规则 ID 与主管审批 ID 落库；PostgreSQL、审计回放、真实浏览器 E2E 和本地门户截图均已验证，截图需由 PR 附件或 CI artifact 长期归档。故事仍不能整体关闭：逐商品/逐批号 UI、多商品 ASN、质量联系单/不合格分流、档案补录、PDA 离线和完整权限/审计断言尚未闭环。 |
| US-M2-004 双人验收签字 | M2 | 已补同货主启用 receiving_clerk 双签资格门禁，并完成 M-VR single/dual_scan/dual_scan_with_approval 三档策略解析、客户端不可降级、规则 ID/H4 审批 ID 落库和审计 diff；PC Web 实时查询策略，真实 PostgreSQL 浏览器 E2E 和本地门户截图覆盖相同签字人拦截与双签成功，截图需由 PR 附件或 CI artifact 长期归档。故事仍不能整体关闭：待第二人签字状态、签字结论/备注、30 分钟超时、电子药检单附件、PDA 离线和完整审计回查尚未闭环。 |
| US-M2-005 PDA/PC Web 智能上架 | M2 | 已补推荐库位 API 与 PC Web 上架确认的真实接口接线，覆盖同货主、温区/质量颜色、容量、SKU 数量、验收数量、幂等和审计约束；真实 PostgreSQL 浏览器 E2E 和本地门户截图已验证推荐读取、失败校验、上架提交与库存追溯，截图需由 PR 附件或 CI artifact 长期归档。验收签字后已原子创建 M-TE 上架任务，但任务完成尚未驱动上架确认；M-VR 动态上架策略、LPN、全自动模式和 PDA 离线冲突仍未闭环。 |
| US-M2-006 收货异常处理 | M2 | 已补收货数量闭合、短少/拒收数量与异常备注持久化，以及 released/receiving 状态整单拒收、关闭状态、货主隔离、幂等和审计；整单拒收审计现明确记录原因、拒收数量和状态变化，PC Web 已接真实拒收 API。故事仍不能整体关闭：销售退货逐批部分拒收、短少强制关闭、温度超标稳定性报告附件、质量联系单/H4 通知、异常统计及拒收专属真实 E2E 截图尚未闭环。 |
| US-M2-007 收货单据打印 | M2 | 已补真实收货/验收/双签打印数据聚合接口、类型化收货现场字段持久化、前端 H9 数据绑定、PostgreSQL 回归和 M2 真实 E2E；仍缺冷链/普通单据的完整生产模板字段库、PC/蓝牙打印机联调、打印失败重试与正式打印审计证据，继续延期。 |
| US-M2-010 上架策略配置 | M2 | 已具备真实 PostgreSQL 库位推荐切片：按货主、仓库、温区/质量色标、容量、SKU 数量和同品聚集过滤排序，PC Web 可读取 Top N 推荐并有真实 E2E 截图。故事仍不能整体关闭：当前是固定推荐算法，尚无多策略方案、优先级拖拽、规则启停、商品类别/货主/仓库绑定、ABC/品类/效期规则配置和无库位 H4 通知。 |
| US-M3-004 在库养护 | M3 | 已补在库养护任务/记录最小真实闭环：任务和批号快照按货主隔离读取，提交校验效期、库存状态、温湿度和结果枚举，记录 append-only，结果写入审计并支持同任务同日幂等回放；真实 PostgreSQL 已覆盖成功、重复、跨货主、过期和非法状态。故事仍不能整体关闭：计划自动生成/周期调度、M-TE PDA 下发、异常照片和 M-QL 联系单、温控设备联动、完成率/逾期预警、PDA 离线冲突及真实前端 E2E 尚未闭环。 |
| US-M3-005 库存盘点 | M3 | 已补 cycle/full/blind 盘点单创建、库存快照、盲盘数量提交、差异计算、主管权限审批、盘盈盘亏原子调整、库存流水、货主隔离、盘点锁、幂等与审计，真实 PostgreSQL 覆盖成功、跨货主、新分配拦截和重放。故事仍不能整体关闭：范围选择前端、PDA 扫码与离线冲突、盘点期间已下发任务账面、差异金额/比例升级审批、盘点报告打印和真实设备/E2E 尚未闭环。 |
| US-M3-006 库内移库 | M3 | M-TE 已有 relocation 任务类型和源/目标库位字段，但尚未形成 M3 移库业务闭环；缺移库单/API、源减目标增同事务、状态/温区/容量门禁、整托 LPN 与逐件追溯码分支、PC/PDA、幂等审计和真实 PostgreSQL/PDA 证据，不能标记完成。 |
| US-M3-009 库存预警 | M3 | 已具备货主级近效期阈值、近效期报表和到期批次自动隔离调度切片。故事仍不能整体关闭：安全库存/超储/养护逾期/温湿度统一预警事件、待处理/已处理/已忽略生命周期、系统消息与可选渠道、审计和统计看板尚未形成 M3 业务闭环。 |
| US-M3-010 ABC 分类管理 | M3 | 当前尚无 ABC 分类持久化、出库统计任务、阈值配置、人工覆盖、报表和审计实现；M2 上架推荐也未消费 ABC 分类，不能以商品 attrs 或固定排序替代完整故事。 |
| US-M4-002 波次规划 | M4 | 已补 M4 PC 波次列表、详情、创建、刷新和未拣选取消真实 API：后端在同一事务内校验已确认订单、按合格库存进行货主隔离锁定、生成按库位排序的拣选明细和 M-TE 拣选任务、更新订单入波次，取消时释放库存锁定并恢复订单状态，且写入幂等和审计；独立 PostgreSQL 回归与临时数据库浏览器 E2E 已验证列表读取、创建响应、库存分配、拣选任务、刷新回显、详情读取、取消响应和真实页面截图。故事仍不能整体关闭：容量/完整路径规则、任务执行回写、异常回滚和完整波次规则证据尚未闭环。 |
| US-M4-003 PDA 拣选作业 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-004 出库复核 | M4 | 已补出库复核查询/提交闭环：状态、逐行商品/数量、拣选与复核分离、短拣、货主隔离、幂等和审计；PC 页面提交前查询 M-VR 出库/复核策略，服务端按整单最严格策略强制第二复核员资质和已通过的 H4 主管审批，并在 outbound_review_records 落规则/双人/审批证据；全量复核完成后原子创建 M-TE 装车任务，短拣不提前创建。一次性 PostgreSQL 真实浏览器 E2E 已验证策略展示、第二人提交与成功回显。故事仍不能整体关闭：装车任务执行回写、PDA 整件/零件扫码、离线冲突、追溯码和真实 PDA/外部运行证据尚未闭环。 |
| US-M4-005 随货同行单打印 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-006 发货与交接 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-007 出库进度看板 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-009 波次出库箱规则 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-010 拣选路径策略 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-011 合并发货与拆单 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M4-012 采购退货出库 | M4 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M5-001 冷链设备台账 | M5 | 已补冷链设备台账的 PostgreSQL 创建、列表、更新和停用切片：设备类型受控、同货主编码唯一、货主隔离、监控中停用门禁、幂等重放和审计均有真实数据库测试及 OpenAPI 契约；故事仍不能整体关闭，校准到期预警/企业微信通知、验证报告附件、外部冷链同步、前端台账和真实浏览器证据尚未闭环。 |
| US-M5-002 温控数据接入与展示 | M5 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M5-003 温度超标事件接入与联动 | M5 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M6-001 GSP 法定台账自动生成 | M6 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M6-002 审计追踪查询 | M6 | 已补审计查询核心的操作类型/关联资源/商品编码/批号过滤、货主隔离、游标分页、IP 返回和 diff 旧值/新值展示，并接入 PC 查询页、开发 mock、导出弹窗、服务端按货主筛选的有界全量 CSV 导出和 H2 归档策略证据；真实 PostgreSQL、TypeScript 和 Shell E2E 已验证。仍缺 H10 恢复审计证据及 Excel/PDF 法定格式证据，故事继续延期。 |
| US-M6-003 业务报表 | M6 | 已补 POST /api/v1/reports/query 的出入库汇总真实 PostgreSQL 查询，按货主统计 receiving_orders/outbound_orders，并覆盖未知报表错误契约和两个货主的数据隔离；故事仍不能整体关闭：前端报表菜单/页面、完整日报月报与周转率/岗位绩效/合规维度、图表、Excel/PDF、定时邮件/企业微信推送、导出审计和正式 E2E/截图证据尚未闭环。 |
| US-M6-004 特殊管理药品专用台账 | M6 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M8-001 自动补货算法（门店水位） | M8 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M8-002 越库作业（Cross-Docking） | M8 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M9-001 计费规则配置 | M9 | 已补计费规则的计费项、计量单位、周期、非负费率、生效窗口与合同窗口校验、同货主合同隔离、并发窗口冲突、Idempotency-Key 幂等回放/冲突、规则写入与审计同事务 PostgreSQL 证据，并新增正式菜单迁移、PC 配置页面、页面级 self-check 和真实浏览器 E2E/截图。仍缺阶梯价格、H-APV 配置审批、真实 GET 历史查询和正式发布证据，不能标记完成。 |
| US-M9-002 自动计费 | M9 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-M9-003 账单管理 | M9 | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-001 创建批号调整单 | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-002 批号调整双人审批 | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-003 PDA 执行批号调整（实物核对 + 库存与追溯码联动） | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-BA-004 ERP 反馈批号调整结果 | BA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-CG-001 编码规则配置 | CG | PC 规则管理页、M1 单据类型字典绑定、真实 PostgreSQL API、真实浏览器动作和本地门户截图已验证，截图需由 PR 附件或 CI artifact 长期归档；但配置审批、规则变更审计展示以及发布前跨业务模块接入验收仍未完成，暂不能关闭故事。 |
| US-CG-002 编码生成服务 | CG | 已完成 M-CG 同事务 no-gap 生成服务、M2 ASN 自动编号、M4 出库单按 M1 单据类型字典校验与同事务自动编号，并接入 M4 PC 出库订单列表、创建、详情和波次创建真实 API，临时数据库浏览器 E2E 已产生订单/详情/波次截图；仍缺配置审批、规则变更审计展示、M4 其他动作真实 API、M3/其他创建方接入及正式版证据，不能关闭故事。 |
| US-DI-001 药检单平台对接配置 | DI | 已补药检平台配置后端和 PC 管理端切片：HTTP/HTTPS 地址、API Key/账号密码认证方式、Vault 凭证引用、超时和 connected/testing/disabled 状态校验；支持多平台、货主隔离、幂等、敏感字段脱敏、审计、真实 PostgreSQL、真实 PC API E2E 和截图，并已挂载 OpenAPI 与菜单迁移。仍缺真实药检平台连通性测试、二维码查询、报告存储与查看、验收联动、PDA E2E 和发布证据，不能标记完成。 |
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
| US-QL-001 质量联系单类型与审批模板配置 | QL | 已实现货主级类型读取/写入、指定审批人、H4 模板、超时时长、启停、权限、审计与幂等配置后端；仍缺预置类型及默认时长、按角色/按货主审批人规则、PC 配置页和超时提醒执行器，故不得整体关闭。 |
| US-QL-002 创建质量联系单 | QL | 已实现手工创建、M-CG 发号、H4 审批记录同事务创建、详情查询、货主隔离、权限、审计与幂等；仍缺 M-VR/M2/M3 自动触发、档案补录与召回专用字段校验、PC/PDA 入口及真实企业微信推送。 |
| US-QL-003 质量联系单审批 | QL | 已实现指定审批人、同意/拒绝、必填意见、H4 与 M-QL 状态原子回写、审计和幂等；仍缺企业微信外部回调签名验真、真实通知送达、审批超时提醒、档案补录 ERP 子状态及外部联调证据。 |
| US-QL-004 质量联系单与业务流程联动 | QL | 已实现审批通过后同事务创建 M-SA 销毁报损单，并由 M-SA 执行链调用 M-VR 销毁节点双人/H4 策略；联动失败会回滚 H4 与 M-QL 状态。仍缺校验放行、验收不合格其他处置、养护异常、库存状态调整、档案补录 ERP 闭环和召回完整状态联动。 |
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
| US-SA-001 报损单管理 | SA | 已完成报损单手工/ERP 创建、M-CG 发号、ERP 外部单号去重、待审批/待执行/执行中/已完成主状态、质量联系单审批回写入口、报损/销毁节点 M-VR 三档双人策略、两名保管员资格与 H4 主管审批门禁、M3 原子扣减、召回销毁标记解除、审计、幂等、权限、查询及 ERP 反馈 outbox，并通过一次性 PostgreSQL 测试。仍缺 M-QL 联系单实体及企业微信真实审批回调、ERP outbox 投递/失败重试 worker 与真实 ERP 回执、PDA 扫码/离线同步和真机证据，故不得整体关闭。 |
| US-SA-002 报溢单管理 | SA | 已完成报溢单手工/ERP 创建、M-CG 发号、ERP 外部单号并发去重、待审批/待执行/执行中/已完成主状态、质量联系单审批回写入口、报溢节点 M-VR 三档双人策略、两名保管员资格与 H4 主管审批门禁、M2 温区/色标/容量校验、M3 库存与库位容量原子增加、审计、幂等、权限、状态查询及 ERP 反馈 outbox，并通过 PostgreSQL 测试。仍缺 M-QL 联系单实体及企业微信真实审批回调、ERP outbox 投递/失败重试 worker 与真实 ERP 回执、PDA 扫码/离线同步和真机证据，故不得整体关闭。 |
| US-SA-003 报损报溢查询与统计 | SA | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-001 追溯码分类管理 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-002 码库管理（大中小码） | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-003 追溯码与商品绑定 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-004 PDA 追溯码对应关系维护 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-005 追溯码录入环节配置 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-006 出库追溯码核验 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TC-007 "码上放心"平台上报 | TC | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-003 任务创建（各模块触发） | TE | 已实现统一任务模型、业务源幂等、待分配初态和审计，并接入 M2 验收通过→上架、M4 波次下发→拣选、最终复核→装车三条原子触发链。仍缺补货、移库、盘点、销退上架等其余业务触发源以及全链路发布证据。 |
| US-TE-005 任务分配 | TE | 已实现手工分派、按资格有效期、当前活跃负荷和最大并发容量自动分派，以及改派、下发、召回；状态与分配事件原子写入，PC 调度页已通过一次性 PostgreSQL 真实浏览器 E2E 和本地门户截图，截图需由 PR 附件或 CI artifact 长期归档。仍缺就近定位、冷库技能匹配和真实 PDA 推送证据。 |
| US-TE-007 任务合并 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-008 任务执行（PDA） | TE | 已实现 PDA 面向的本人统一待办 API，以及已下发→执行中→完成/异常状态机、本人执行权限、数量差异拦截、幂等和审计。受 ADR-0027 约束尚未启动生产 PDA 应用，扫码三/四步、SQLite 离线缓存、冲突待主管处理和真机证据仍缺。 |
| US-TE-009 任务跟踪看板 | TE | 已补按状态、类型、仓库、人员与优先级展示的 PC 任务调度列表和主管干预；支持关闭或 5/15/30 秒自动刷新，并按任务预计时长高亮执行超时和未接单超时，真实 PostgreSQL 浏览器 E2E/截图已验证。仍缺超时企业微信通知和 PAD 视图。 |
| US-TE-010 任务绩效统计 | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-TE-011 设备接口抽象（RFID/AGV/输送线预留） | TE | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-001 校验规则配置 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-002 校验规则执行 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-003 校验异常人工处理 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-004 校验规则模板 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-005 校验规则测试 | VR | 当前实现和证据不足以证明该故事全部验收标准，禁止以局部页面、接口或静态文件标记完成。 |
| US-VR-006 双人策略矩阵规则 | VR | 已完成 6 流程 12 节点、8 类默认模板、全局/货主/仓库优先级、双人确认、M1 字典双向同步、幂等、审计、权限、查询缓存与变更失效；PC 矩阵、M2 验收、M4 复核及 M-SA 报损/报溢/销毁执行均已接入三档策略，M-QL 审批通过可同事务创建销毁报损单并进入该执行链，规则/第二人/H4 审批证据可回查。仍缺正式环境 P95 指标，以及 ADR-0027 解禁后的 PDA 离线 24 小时缓存/恢复校验和真实 PDA 证据，故不得整体关闭。 |

## 验证故事详细矩阵

| 故事 | 模块 | 验收层级 | 类型 | 测试层 | 前端 | API | 状态 |
|---|---|---|---|---|---|---|---|
| US-M1-011 系统字典中心 | M1 | S2 | write、config_rule、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m1-system-dictionary | GET /api/v1/system-dictionaries/{dict_code}/items<br>PUT /api/v1/system-dictionaries/{dict_code}/items/{item_code}<br>PATCH /api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-005 Token 失效与登出 | H1 | S2 | write、api_change、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h1-session-management | POST /api/v1/auth/logout<br>PUT /api/v1/auth/me/password<br>GET /api/v1/auth/sessions<br>POST /api/v1/auth/sessions/{session_id}/revoke<br>POST /api/v1/auth/sessions/revoke-others<br>POST /api/v1/auth/users/{user_id}/kick<br>PUT /api/v1/auth/users/{user_id}/status | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-006 API Key 生命周期管理 | H1 | S2 | write、api_change、frontend_interaction、integration | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h1-api-keys | GET /api/v1/auth/api-keys<br>POST /api/v1/auth/api-keys<br>POST /api/v1/auth/api-keys/{api_key_id}/rotate<br>POST /api/v1/auth/api-keys/{api_key_id}/revoke | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M1-004 仓库与库位管理 | M1 | S2 | write、api_change、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m1-warehouses、m1-zones、m1-locations | GET /api/v1/master-data/warehouses<br>POST /api/v1/master-data/warehouses<br>PATCH /api/v1/master-data/warehouses/{id}<br>GET /api/v1/master-data/warehouse-zones<br>POST /api/v1/master-data/warehouse-zones<br>PATCH /api/v1/master-data/warehouse-zones/{id}<br>GET /api/v1/master-data/locations<br>POST /api/v1/master-data/locations<br>POST /api/v1/master-data/locations/batch-create<br>PATCH /api/v1/master-data/locations/{id} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M3-001 实时库存查询 | M3 | S2 | read_only、api_change、frontend_interaction、integration | L1、L2、L3、L4、L7、L8、L9、L10 | m3-batches | GET /api/v1/inventory/batches | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M3-011 库位历史追踪 | M3 | S2 | read_only、api_change、frontend_interaction、integration | L1、L2、L3、L4、L7、L8、L9、L10 | m3-location-history | GET /api/v1/inventory/locations/history<br>GET /api/v1/inventory/batches/{id}/trace | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-001 用户登录与 token 签发（PC 密码登录切片） | H1 | S2 | write、api_change、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | - | POST /api/v1/auth/login<br>GET /api/v1/auth/me | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-002 角色与权限管理 | H1 | S2 | write、api_change、frontend_interaction、config_rule | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h1-role-permission | GET /api/v1/auth/roles<br>POST /api/v1/auth/roles<br>PUT /api/v1/auth/roles/{role_id}<br>DELETE /api/v1/auth/roles/{role_id}<br>PUT /api/v1/auth/roles/{role_id}/permissions<br>GET /api/v1/auth/permissions<br>GET /api/v1/auth/users<br>PUT /api/v1/auth/user-roles/batch | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-003 API 鉴权中间件 | H1 | S1 | read_only、api_change | L1、L2、L3、L8、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-004 多货主数据隔离（AuthContext 切片） | H1 | S1 | read_only、api_change | L1、L2、L3、L8、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H1-007 PC 管理端三层菜单管理 | H1 | S2 | write、config_rule、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h1-menu-management | GET /api/v1/admin/menus/draft<br>POST /api/v1/admin/menus/draft/nodes<br>PATCH /api/v1/admin/menus/draft/nodes/{id}<br>POST /api/v1/admin/menus/draft/batch-enable<br>POST /api/v1/admin/menus/publish<br>GET /api/v1/admin/menus/published<br>POST /api/v1/admin/menus/rollback | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-001 审计事件统一记录（同步 append-only 切片） | H2 | S3 | write、audit_compliance | L1、L2、L3、L4、L5、L8、L10、L11 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:not_applicable<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-002 审计追踪查询（后端分页查询切片） | H2 | S1 | read_only、api_change、frontend_interaction | L1、L2、L3、L7、L8、L9 | h2-audit-trail | GET /api/v1/audit/events | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-003 append-only 不变量保护 | H2 | S3 | audit_compliance | L5、L8、L10、L11 | - | - | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:not_applicable<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-004 审计数据归档与保留（后端归档运行切片） | H2 | S3 | write、audit_compliance | L1、L2、L3、L4、L5、L8、L10、L11 | - | GET /api/v1/audit/archive/partitions<br>POST /api/v1/audit/archive/runs | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-005 事件总线（DB outbox 与订阅投递切片） | H2 | S3 | write、integration、audit_compliance | L1、L2、L3、L4、L5、L8、L9、L10、L11 | - | GET /api/v1/event-bus/deliveries/pending<br>POST /api/v1/event-bus/deliveries/{delivery_id}/ack<br>POST /api/v1/event-bus/deliveries/{delivery_id}/nack | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H2-006 业务数据生命周期归档（后端策略与计划切片） | H2 | S3 | write、config_rule、audit_compliance | L1、L2、L3、L4、L5、L8、L9、L10、L11 | - | GET /api/v1/business-retention/policies<br>POST /api/v1/business-retention/jobs | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-001 OpenAPI 契约生成与前端类型同步 | H3 | S1 | api_change | L2、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-002 前端 TS 类型生成 | H3 | S1 | api_change | L2、L9 | - | - | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-003 API 限流与熔断 | H3 | S3 | api_change、runtime_guard、config_rule、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | - | GET /api/v1/resilience/status<br>GET /metrics | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H3-004 API 文档可访问性 | H3 | S1 | read_only、api_change、config_rule、frontend_interaction | L1、L2、L3、L4、L7、L8、L9 | h3-api-contract | GET /openapi.json<br>GET /api-docs<br>GET /redoc | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:not_applicable<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H4-001 通知配置 | H4 | S3 | write、config_rule、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h4-notify-configs、h4-wechat-settings | GET /api/v1/wechat-notify/configs<br>POST /api/v1/wechat-notify/configs | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H4-004 通知发送记录查询 | H4 | S3 | read_only、write、api_change、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h4-notify-records | GET /api/v1/wechat-notify/records<br>POST /api/v1/wechat-notify/records/{record_id}/resend | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-001 快递商配置 | H5 | S3 | write、config_rule、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express | GET /api/v1/express/carriers<br>POST /api/v1/express/carriers | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-002 快递选择规则配置 | H5 | S3 | write、config_rule、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express | GET /api/v1/express/routing-rules<br>POST /api/v1/express/routing-rules | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-003 快递单打印 | H5 | S3 | write、frontend_interaction、api_change、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express、h9-print-templates | POST /api/v1/packing/jobs/{id}/waybill<br>POST /api/v1/express/waybills | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-004 快递推送下单 | H5 | S3 | write、integration、runtime_guard、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | h5-express | POST /api/v1/express/waybills<br>POST /api/v1/express/waybills/{waybill_no}/cancel | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H5-005 快递轨迹查询 | H5 | S2 | read_only、integration、frontend_interaction、api_change | L1、L2、L3、L4、L7、L8、L9、L10 | h5-express | GET /api/v1/express/waybills/{waybill_no}/tracking | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H6-001 状态机定义注册与转换校验 | H6 | S1 | read_only、api_change、config_rule | L1、L2、L3、L4、L8、L9 | - | GET /api/v1/state-machines<br>GET /api/v1/state-machines/{machine_code}<br>GET /api/v1/state-machines/{machine_code}/transition-validation | requirement:verified<br>fields:verified<br>frontend:not_applicable<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-001 打印模板类型字典 | H9 | S1 | read_only、config_rule、frontend_interaction | L1、L2、L3、L4、L7、L8、L9 | m1-system-dictionary、h9-print-templates | GET /api/v1/system-dictionaries/{dict_code}/items<br>PUT /api/v1/system-dictionaries/{dict_code}/items/{item_code}<br>PATCH /api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-002 字段库生成与字段元数据维护第一切片 | H9 | S1 | read_only、api_change、frontend_interaction | L1、L2、L3、L7、L8、L9 | h9-print-templates | GET /api/v1/print-templates/field-libraries | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-003 模板设计与版本管理 | H9 | S2 | write、config_rule、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h9-print-templates | GET /api/v1/print-templates/templates<br>POST /api/v1/print-templates/templates<br>GET /api/v1/print-templates/field-libraries/{version_id}/fields | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-004 预览与浏览器打印 | H9 | S2 | write、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | h9-print-templates | POST /api/v1/print-templates/preview<br>POST /api/v1/print-templates/print | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-H9-005 业务模块接入规则 | H9 | S1 | read_only、frontend_interaction、api_change | L1、L2、L3、L7、L8、L9 | h9-print-templates | POST /api/v1/print-templates/resolve | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M2-002 PDA/PC Web 收货 | M2 | S3 | write、inventory_change、frontend_interaction、critical_path | L1、L2、L3、L4、L5、L6、L7、L8、L9、L10、L11 | m2-receiving | POST /api/v1/inbound/receiving-orders/{id}/receive | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M2-008 收货进度看板 | M2 | S1 | read_only、frontend_interaction | L1、L2、L3、L7、L8 | m2-receiving | GET /api/v1/inbound/receiving-dashboard | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:not_applicable<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-M4-001 出库订单管理 | M4 | S2 | write、frontend_interaction、api_change | L1、L2、L3、L4、L5、L7、L8、L9、L11 | m4-orders | GET /api/v1/outbound/orders<br>POST /api/v1/outbound/orders<br>GET /api/v1/outbound/orders/{id} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-AL-001 告警定义注册 | AL | S3 | read_only、write、api_change、frontend_interaction、config_rule、integration、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | hal-alert-definitions | GET /api/v1/alert-definitions<br>GET /api/v1/alert-definitions/{id}<br>POST /api/v1/alert-definitions/change-requests<br>PUT /api/v1/quality-liaisons/types/{type_code}<br>POST /api/v1/quality-liaisons/{id}/approval-callback | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-AL-003 告警升级机制 | AL | S3 | read_only、write、api_change、frontend_interaction、config_rule、integration、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | hal-alert-escalations | GET /api/v1/alert-escalation-rules<br>PUT /api/v1/alert-escalation-rules/{rule_code} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-AL-004 告警看板与统计 | AL | S3 | read_only、write、api_change、frontend_interaction、integration、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | hal-alert-dashboard | GET /api/v1/alerts/active<br>GET /api/v1/alerts/statistics<br>GET /api/v1/alerts/gsp-report<br>GET /api/v1/alerts/changes<br>POST /api/v1/alerts/exports<br>GET /api/v1/alerts/exports/{id}<br>GET /api/v1/alerts/exports/{token}/download | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-DOCK-001 月台档案管理 | DOCK | S2 | write、api_change、frontend_interaction、integration | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | dock-management | GET /api/v1/docks<br>POST /api/v1/docks<br>POST /api/v1/docks/import<br>PATCH /api/v1/docks/{id}<br>DELETE /api/v1/docks/{id} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-TE-001 任务类型配置 | TE | S2 | write、config_rule、frontend_interaction | L1、L2、L3、L4、L5、L7、L8、L9、L11 | mte-task-types | GET /api/v1/task-engine/task-types<br>PUT /api/v1/task-engine/task-types/{task_type_code}<br>PATCH /api/v1/task-engine/task-types/{task_type_code}/enabled | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-TE-002 任务组与人员资格 | TE | S3 | write、config_rule、frontend_interaction、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | mte-task-groups | GET /api/v1/task-engine/task-groups<br>GET /api/v1/task-engine/workers<br>PUT /api/v1/task-engine/task-groups/{task_group_code} | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-TE-004 任务优先级规则 | TE | S3 | write、config_rule、frontend_interaction、api_change、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | mte-task-types、mte-task-dispatch | GET /api/v1/task-engine/priority-rule<br>PUT /api/v1/task-engine/priority-rule<br>GET /api/v1/task-engine/tasks<br>POST /api/v1/task-engine/tasks/{task_id}/transitions | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |
| US-TE-006 任务释放控制 | TE | S3 | write、config_rule、frontend_interaction、api_change、runtime_guard、audit_compliance | L1、L2、L3、L4、L5、L7、L8、L9、L10、L11 | mte-task-types、mte-task-dispatch | GET /api/v1/task-engine/task-types<br>PUT /api/v1/task-engine/task-types/{task_type_code}<br>POST /api/v1/task-engine/tasks<br>GET /api/v1/task-engine/tasks<br>POST /api/v1/task-engine/tasks/{task_id}/transitions | requirement:verified<br>fields:verified<br>frontend:verified<br>api:verified<br>backend:verified<br>database:verified<br>security:verified<br>audit:verified<br>tests:verified<br>evidence:verified<br>docs:verified<br>governance:verified |

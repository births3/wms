# 用户故事：H9 打印组套与 Print Agent

> 模块：H9 print-orchestration
> 性质：H9 第二阶段横向能力
> 状态：US-H9-006、US-H9-007、US-H9-008、US-H9-009、US-H9-011 已完成 V0-V3
> 开发软件验收（US-H9-011 的真实物理打印机/USB/Print Agent 证据待 S4 硬件验收；
> US-H9-009 的正式环境 KMS/S3 证据按环境分层延期）；US-H9-010、012～015 的业务规则、
> 安全残余风险、机器身份协议和多货主物理打印站点已确认，软件与适用真实硬件证据待实现
> 依赖：H1 权限、H2 审计、H3 OpenAPI、H6 状态机、H-FILE、M1 客户地址/系统字典、
> M-CG 单据号、M4 出库、M-DI 药检单
> 关联 ADR：[ADR-0039 打印组套与 Print Agent](../adr/0039-print-suite-and-agent.md)、
> [ADR-0040 Print Agent 机器身份与协议闭环](../adr/0040-print-agent-machine-protocol.md)、
> [ADR-0041 H9 打印编排细化](../adr/0041-print-orchestration-refinement.md)

---

## 范围和边界

本故事集承接 ADR-0036 已有模板能力，增加随货同行单归集、打印组套、分类 PDF、打印任务、
打印机/纸盒、Print Agent 和多 Agent 调度。现有浏览器打印仍可用于模板设计和人工预览，
但不能替代本故事集的本地静默打印和真实硬件验收。

US-H9-006 已完成线路冻结、结构化截单计划、原子归集、管理端和真实数据 E2E；后续故事的
文档完成仍不代表代码、页面、数据库、Windows 客户端或真实打印机已经验收。

## US-H9-006：随货同行单归集与截单

**作为** 仓库主管
**我要** 按受控截单计划把多个出库订单归成一个随货同行单号
**以便** 同一客户后续追加的订单可以在作业截止点前合并打印

### 验收标准

1. 订单进入 WMS 时冻结线路；同一送货地址在同一时点只能归属一条线路。
2. 截单计划支持结构化周计划、例外日期和授权人工截单。
3. 截单计划优先级为客户、线路、货主+仓库；同级同对象有效期重叠时禁止发布。
4. 截单在同一事务内冻结订单集合，并通过 M-CG 按受控编号主题
   `print_document_category:delivery_note` 生成唯一且无跳号的随货同行单号；后续订单不得
   回写已冻结集合。
5. `owner_id + warehouse_id + delivery_address_id` 是不可配置覆盖的归集硬边界；任一值不同
   都禁止归入同一随货同行单。地址使用 M1 稳定地址主数据 ID 匹配，三者相同的订单可跨 ERP
   订单组号归集。
6. 截单、人工截单和归集结果写入 H2 审计并支持幂等重放。

## US-H9-007：归集维度规则配置

**作为** 系统管理员
**我要** 从受控订单字段中配置归集维度及顺序
**以便** 不同发票、运输方式、部门或特殊业务可以复用同一套归集能力

### 验收标准

1. 页面只允许选择已登记的订单标准字段、等值归组方式和维度顺序。
2. 禁止自由 SQL、脚本、正则表达式和任意字段路径。
3. 规则使用草稿、测试、发布版本；已被归集实例引用的发布版本不可改写。
4. 发布前以样本订单展示命中规则、分组键和预计归集结果。
5. 地址隔离是不可覆盖的系统约束，配置不能允许跨地址归集。
6. 发布、停用和测试支持幂等重放并写入 H2 审计，运行实例保存规则版本快照。

## US-H9-008：打印组套配置与就绪策略

**作为** 系统管理员
**我要** 按送货地址和客户发布包含多个有序打印项的打印组套
**以便** 每个客户地址使用正确的单据、份数、模板、顺序和等待策略

### 验收标准

1. 组套匹配顺序为送货地址、客户、线路、货主+仓库默认；同级有效期重叠禁止发布。
2. 组套使用草稿、测试、发布版本；已发布版本不可改写。
3. 每个打印项维护单据分类、份数、顺序、逻辑输出槽、是否必需、就绪策略和失败策略；
   `rendered` 分类还必须绑定模板版本，`external_file` 分类绑定已摄取文件而不强制本地模板。
4. 单据分类使用 M1 系统字典 `print_document_category`，字典项至少包含编码、中文名称和
   `source_mode = rendered|external_file`。首批为随货同行单（`rendered`）、药检单和发票
   （`external_file`）；同一分类允许多项，渲染分类允许使用不同模板，后续通过字典受控新增。
5. 发票完整指全部源订单被有效发票覆盖；药检单完整指全部必需商品+批号被有效报告覆盖。
6. 发票和药检单引用 H-FILE 文件 ID 或已摄取的稳定文件来源；访问 URL 按需短期生成，
   禁止把临时外部 URL 当长期事实源。
7. 必需单据未就绪时，组套可配置为仅挂起当前实例并允许后续已就绪实例继续，或暂停对应
   Agent 队列等待人工；策略冻结到实例，不得未经授权自动跳过。
8. 组套实例保存组套版本、规则版本和源单据快照；每个 `rendered` 打印项保存模板版本，
   每个 `external_file` 打印项保存权威文件 ID 与版本。
9. 组套测试、发布和停用支持幂等重放，并记录版本、作用范围、操作者和 H2 审计追踪。

## US-H9-009：分类 PDF 渲染与留存

**作为** 仓库主管
**我要** 按单据分类生成可选择打印的 PDF
**以便** 可以全打、只打印部分分类或按保留要求管理文件

### 验收标准

1. `rendered` 分类由 Render Worker 在服务端生成 PDF 并写入 H-FILE；`external_file` 分类
   校验并引用已摄取的权威 PDF，不做无意义的二次渲染。
2. 每个分类 PDF 保存 `source_mode`、源数据版本或权威文件 ID/版本、内容哈希和处理结果；
   `rendered` 分类还必须保存模板版本，`external_file` 分类的模板版本为空。
3. 支持选择全部分类或部分分类打印；完整组套 PDF 仅按需临时合并。
4. 随货同行单按受控策略归档；已有权威文件的发票和药检单只保存引用及短期缓存。
5. PDF 渲染失败或外部 PDF 校验/摄取失败不得创建可执行打印任务；重试复用同一组套实例
   和幂等键。
6. 文件读取、下载和应急打印按 H1 权限控制并写入 H2 审计。

## US-H9-010：打印任务队列、顺序与重打

**作为** 仓库主管
**我要** 查看和控制组套实例、打印项及每次尝试
**以便** 按配置顺序打印并安全处理失败、暂停、改派和重打

### 验收标准

1. 队列先按业务优先级、再按原始入队时间 FIFO；授权加急保留审计。
2. 一个组套实例同一时点只由一个 Agent 执行；一个 Agent 同时只执行一个组套实例。
3. 打印项严格串行提交；前一项成功后才提交下一项。前一项失败时，只有“非必需 + 实例冻结
   的失败策略允许继续”才能原子标记为跳过并推进，否则暂停组套；任何策略都不得并行越序。
4. 支持整组套、分类、单项重打；页码范围重打需要独立权限。
5. 暂停接单在当前打印项结束后不再开始下一项，且不自动移动队列；任务改派只允许未开始
   实例，并保留原优先级和入队时间。
6. 运行中、结果不明或等待 Agent 对账的实例不得普通改派或自动重打。
7. 紧急停止只尽力取消当前本地打印后台任务，无法确认时进入人工确认。
8. 暂停/恢复、作废、取消、失败、跳过、重打、改派和人工确认支持幂等重放，并保留 PDF
   哈希、尝试记录与 H2 审计。
9. 支持按参数自动释放或授权人工释放组套；释放策略冻结到组套实例。
10. 本地打印后台无法确认结果时，打印项进入 `result_unknown`，组套进入
    `awaiting_manual_confirmation`；不得自动重试、跳过、改派或释放设备租约。
11. 授权人工可为运行中组套提交幂等暂停请求并填写原因；当前打印项结束且确认无在途、
    `result_unknown` 或待对账后，组套进入 `paused`，不得启动下一项或移动队列。紧急停止
    无法确认结果时仍进入人工确认，不能伪装成普通暂停。Agent“暂停接单”是独立运行状态，
    不改变组套分配，也不等于改派。

## US-H9-011：打印机、纸盒与设备租约

**作为** 系统管理员
**我要** 按物理打印站点维护打印机、纸盒和测试结果
**以便** 多货主共享仓库设备且每个打印项使用正确纸张

### 验收标准

1. 打印机归属一个物理打印站点，可被本站显式映射的多个货主+仓库使用，禁止跨站点引用。
2. 打印机支持多个纸盒；纸盒维护纸张能力、启用状态和设备标识。
3. 维护页可以对指定打印机和纸盒执行测试打印，并保存结果和 H2 审计。
4. 模板/打印项声明纸张要求，Print Agent 输出槽映射到兼容的主打印机/纸盒，并可明确配置
   本站兼容备用设备；未配置备用设备时默认暂停并告警。
5. 网络打印机可被多个 Agent 登记，但同一时点只有一个 Agent 持有设备租约；USB 设备只归本机。
6. 租约支持安全自动释放或仅人工释放，默认仅人工释放；全局默认在参数页维护，打印机可
   单独覆盖，运行中的租约使用配置快照。
7. 人工释放需要专用权限、原因和二次确认；存在 `printing`、`result_unknown` 或未决对账时
   任何人都不得释放，必须先完成打印结果确认或对账。人工权限只能覆盖 `manual_only`
   释放模式，不能覆盖该硬安全条件。

## US-H9-012：Print Agent 注册、状态与设备映射

**作为** 系统管理员
**我要** 注册和测试物理打印站点 Print Agent
**以便** 每个站点客户端以最小权限连接本地打印设备

### 验收标准

1. 一个 Agent 归属一个物理打印站点，可连接本站多台打印机，不归属打印组套。
2. 复用“设备·Print Agent 管理”维护物理打印站点及其显式授权的
   `owner_id + warehouse_id` 映射，不新增菜单，也不改变 M1 仓库的单货主边界。
3. 站点映射的新增、变更和停用需要专用权限、幂等键和 H2 审计。
4. 管理端使用密码学安全随机数生成器签发至少 128 bit 熵、绑定 Agent、站点、精确源 IP 和
   有效期的一次性注册码；激活失败按请求 Agent 标识/IP 限流并按受控阈值锁定告警。能够
   解析到 Agent/站点的失败按映射 owner 写 H2，无法归属的探测只写脱敏安全日志和指标，
   不伪造 owner，也不记录注册码或机器秘密。激活后每个 Agent 使用独立 H9 机器凭据。
5. 一次性注册码只能调用激活端点；激活是唯一预凭据普通 HTTP 例外，校验注册码与 socket
   peer IP。机器凭据绑定 `agent_id + print_site_id` 和最小机器权限，由 H9
   `MachineAuthContext` 鉴权，不构造普通货主 `AuthContext`；只复用 H1 的哈希、有效期、
   轮换、吊销、失败锁定和审计规则。
6. Agent 只能领取服务端按 `agent_id` 分配且其 `owner_id + warehouse_id` 属于本站映射的任务；
   Agent 请求不得传入或选择 `owner_id`，任务继续按自身货主隔离并写入 H2 审计。
7. 配置状态为 testing/active/disabled；运行状态为 online/suspected/offline/paused，分别展示。
8. 启用测试覆盖 H9 机器凭据、socket peer 精确源 IP、物理打印站点及货主仓映射、完整
   机器端点白名单、长轮询、心跳和逻辑输出槽完整性。
9. 站点、机器凭据、IP 白名单、专用 listener/协议或输出映射变化后回到 testing；名称和备注
   不触发。Agent 必须直连独立端口的 WMS 专用 listener；该入口只挂载 H9 机器白名单并拒绝
   其他路由，禁止经过会改写源地址的 L7 代理、网关或 NAT。
10. 输出映射版本化并审计；运行实例使用启动时快照，新映射只影响新实例。
11. 默认心跳 10 秒、疑似离线 30 秒、离线 60 秒、稳定恢复 60 秒，允许受控覆盖。
12. 注册、激活、测试、启停和凭据轮换支持幂等重放并写入 H2 审计；激活重放不得再次返回
    明文秘密，首次响应丢失时必须吊销凭据并重新注册。首版轮换时先暂停；存在运行中、
    结果不明或等待对账任务时必须阻塞轮换，完成或对账后才能吊销旧凭据并复用新注册码
    重新激活，不得通过取消、删除或“清空”未决任务绕过该条件，不建设双凭据切换协议。
13. Web 管理页展示当前任务、排队实例、版本、心跳、输出映射、凭据状态和审计入口；
    本地界面只管理本机。暂停接单完成当前打印项后不再开始下一项。明文机器密钥只返回一次，
    由 Rust 写入 Windows Credential Manager，React、日志和业务数据目录不得保存。
14. 站点至少存在一条有效货主+仓库映射后才能激活。Web 端站点/Agent 读写、激活、轮换和
    单 Agent pilot 动作必须通过 H1 port 校验操作者对映射前后 owner 并集逐一具备对应
    Agent 管理权限。pilot 提升 stable 和全局最低版本变更还必须具备专用平台权限
    `h9.agent_version.global.write`，并校验全部未删除站点映射的 owner 并集；任一失败整体
    返回 403，owner 并集为空时返回 409 且不得变更全局版本，只写脱敏安全日志/指标，
    不伪造 owner 写 H2。
15. 任务/尝试动作按任务快照 owner 写 H2；站点、Agent、凭据和安全动作按映射前后 owner
    并集分别写 H2，全局版本动作按全部未删除站点映射 owner 分别写 H2；同一动作产生的事件
    复用 H2 既有 `request_id` 聚合。心跳/磁盘/设备读数只进运行状态和指标，只有状态跃迁、
    阈值告警和安全事件写 H2。机器事件的审计操作者使用 Agent ID/冻结名称及
    `h9-machine:<credential_id>`；可解析的预凭据激活使用
    `h9-activation:<activation_code_id>`，不得冒充用户。
16. 所有机器写操作都携带 `Idempotency-Key` 且重试复用原键。非遥测命令使用 H9 自有
    幂等作用域，不复用 owner 必填的共享业务幂等表；高频遥测不落通用幂等行，以
    `agent_id + boot_id + sequence` 单调更新，同序号变载荷拒绝并告警；任务/尝试结果继续
    使用 `Idempotency-Key + request_hash` 防重。

## US-H9-013：多 Agent 分配与故障降级

**作为** 仓库主管
**我要** 按地址、客户、线路和仓库配置主备 Agent
**以便** 设备或 Agent 故障时能在不重复打印的前提下降级

### 验收标准

1. 分配优先级为送货地址、客户、线路、仓库默认；同级同对象有效期重叠时禁止发布。
2. 分配规则支持立即或未来生效；变更只影响尚未创建的组套实例，不追溯重算既有实例。
3. 每条规则明确主 Agent 和可选备用 Agent；主备必须属于同一物理打印站点且输出槽兼容。
4. Agent 在创建组套实例时解析；订单入库只冻结业务线路，不冻结 Agent。
5. 主 Agent 开始前故障可切备用；进入运行态后，如果尚无任何已提交尝试，或上一项已确认
   成功/受控跳过，且没有在途尝试、结果不明或待对账，则可通过状态机安全故障转移到同站点
   兼容备用 Agent，从首个待打印项重新预载后续打。运行中转移还要求旧 Agent 在线、持久化
   确认停止及交接；疑似离线/离线一律先对账，不得转移。
6. 主 Agent 恢复稳定后只接新实例；已在备用 Agent 的实例不自动回迁。
7. 降级顺序为本 Agent 备用设备、备用 Agent、授权人工改派本站兼容 Agent、具备专用权限的
   PDF 应急打印。
8. 分配、改派和应急打印支持幂等重放，记录原因、范围、操作者、确认结果和 H2 审计。
9. 每次分配携带单调递增的 `assignment_epoch`，每个设备租约携带不可复用的 `lease_token`。
   Agent 启动打印项和上报结果时必须回传二者；旧代次不得推进正常状态，任何迟到结果转入
   对账并告警。运行中故障转移只有在冻结租约策略允许 `safe_auto`，或授权用户提供原因并
   完成二次确认时才能释放旧租约。

## US-H9-014：Agent 断网对账与本地空间

**作为** 系统管理员
**我要** 在断网、重启和磁盘不足时安全恢复 Print Agent
**以便** 不丢任务、不重复打印且不泄露长期业务文件

### 验收标准

1. Agent 开始前完整下载清单、全部 PDF 和哈希；下载不完整时禁止执行。
2. 执行中断网只完成当前组套，不领取下一实例；服务端在 Agent 进入 suspected/offline 时
   把组套转为 `awaiting_reconciliation`，禁止故障转移和自动重打。重连后先对账；只有在线
   旧 Agent 持久化确认停止及交接后，才可另走受控故障转移。
3. Agent 持久化不含业务正文的本地日志；重启后先扫描缓存、恢复和对账。
4. 同一尝试且服务端仍等待、没有后续重打时可自动对账；已有人工动作或后续尝试时转人工。
5. PDF 只保存在应用安装目录之外的专用数据目录缓存；目录使用 Windows ACL，仅允许 Agent
   运行账户和本机管理员访问，禁止使用网络共享目录；服务端或人工确认后删除。
6. 自动清理不得删除运行中、结果不明、等待对账和人工处理文件；空间不足时停止接单并告警。
7. Web 和本地界面展示路径、总量、可用量、缓存、未决、可回收、最后清理和失败原因。
   Agent 离线时 Web 显示最后上报时间。
8. 默认剩余 20% 告警、10% 停止；预下载还必须满足组套大小加安全余量。
9. 未决缓存存在时禁止更改目录；变更目录通过权限/空间测试后使 Agent 回到 testing。
10. 自动对账、人工冲突处理、清理和目录变更支持幂等重放并写入 H2 审计，审计不得包含 PDF 正文。

## US-H9-015：Print Agent 客户端与启动更新

**作为** 系统管理员
**我要** 下载、启动和更新 Windows Print Agent
**以便** 客户端版本受控且更新不会打断打印任务

### 验收标准

1. 首期仅支持 Windows；客户端采用 Tauri 2 + React/Rust 和登录自启动托盘模式，不安装服务。
2. React 只负责本地 UI；Rust 负责长轮询、凭据、Windows 打印、缓存、日志和更新编排。
3. 管理端显示由已登录且具备 Agent 管理权限的 H1 用户通过常规 HTTPS Web 端点访问的当前
   stable 完整初始安装包下载地址，以及当前版本、推荐版本、最低版本、发布说明和更新状态；
   初装不得选择 pilot 或复用普通 HTTP `/agent-releases`。
4. Velopack 启动钩子必须最先运行但关闭默认自动应用；随后先扫描本地缓存并恢复/对账。
   只有不存在运行中、结果不明或等待对账任务时，才允许显式检查并应用更新；已下载的待应用
   包也不得绕过该检查。
5. 当前版本不低于最低版本时可拒绝推荐更新；低于最低版本时禁止接单。
6. 使用 Velopack 增量包；增量或 SHA-256 校验失败时只再尝试一次完整包，仍失败则保留当前
   版本。更新允许重启，但不得丢失任务日志和缓存。
7. 更新只替换客户端程序目录，不覆盖或清理独立的数据目录、凭据、缓存和持久日志。
8. 更新源复用 WMS 固定 `/agent-releases`；清单、当前 pilot/stable、相邻增量、完整包和
   SHA-256 通过 H9 机器凭据与 socket peer 精确源 IP 白名单下载，禁止匿名下载。
9. 包只能由发布脚本/CI 发布；临时文件经大小和 SHA-256 校验后原子改名，再发布元数据。
   包按版本和哈希不可变，WMS 进程只读；管理端不能上传、覆盖或复制可执行文件。
10. Agent 启动默认检查 stable；stable 可直接选择更新或暂不更新。选择 pilot 必须在线登录
    具备 Agent 管理权限的 H1 用户，由服务端签发绑定
    `agent_id + startup_id + target_version + SHA-256` 的短期授权；机器凭据本身不能选择
    pilot，用户 token 和密码只驻留内存，选择只对本次启动有效。
11. pilot 提升 stable 必须由具备 `h9.agent_version.global.write`，且对全部未删除站点映射
    owner 均具备 Agent 管理权限的用户手工执行；任一鉴权失败整体返回 403，owner 并集为空
    时返回 409。动作只更新通道指向并复用同一包和 SHA-256，不设置自动验证门禁，也不得
    重新构建；按该 owner 并集分别写 H2，并复用同一 `request_id`，记录操作者、版本和哈希。
12. 禁止降级；错误版本以更高版本重新发布。pilot 高于 stable 时不得因切回 stable 降级。
13. 最低版本与 stable 独立设置；全局最低版本变更必须具备
    `h9.agent_version.global.write`，并校验操作者对全部未删除站点映射 owner 均具备 Agent
    管理权限；任一鉴权失败整体返回 403，owner 并集为空时返回 409。设置前展示将停止接单
    的 Agent，要求二次确认，并按该 owner 并集分别写 H2、复用同一 `request_id`。低于最低
    版本的 Agent 完成恢复/对账后禁止领取新任务，直至更新成功。
14. 在线更新目录只保留当前 pilot、当前 stable 和相邻增量，历史构建由 CI artifact 归档；
    缺少增量的旧 Agent 使用完整包，不建设更新并发调度器。
15. 本地日志记录完整更新过程；H2 只记录通道选择、接受更新、应用成功/失败、stable 提升和
    最低版本变更，并包含通道、原版本、目标版本、SHA-256 和结果。stable 接受动作没有用户
    登录时使用 Agent 机器身份。
16. 正式环境允许普通 HTTP + SHA-256 启动更新，不做代码签名或发布者验真。必须显示并审计
    该残余风险，不得把 SHA-256 描述为来源证明。
17. 更新通道跨网段、公网或不可信网络，或者发生包替换安全事件时，必须重新评估 HTTPS 或
    代码签名。
18. 首个正式版本前直接更新并重装测试 Agent；正式发布后的破坏性协议变更按 ADR-0016
    两阶段执行，禁止永久维护双协议。
19. 无签名 Windows 客户端必须用真实机器验证 SmartScreen/杀毒软件对安装和更新的影响；
    被拦截时保留旧版本并告警，禁止客户端自动关闭或绕过系统防护。

## 状态机与合法迁移

H9 在 H6 注册不可变状态机定义；H6 负责定义与迁移校验，H9 仍保存业务实体状态，并在同一
事务内完成状态写入、业务事件和 H2 审计。

| H6 状态机编码 | 实体 | 初始态 | 终态 | 状态集合 |
|---|---|---|---|---|
| `h9_print_suite` | 组套实例 | `waiting_documents` | `completed`、`failed`、`cancelled` | `waiting_documents`、`queued`、`preparing`、`running`、`paused`、`awaiting_reconciliation`、`awaiting_manual_confirmation`、`completed`、`failed`、`cancelled` |
| `h9_print_item` | 打印项 | `pending` | `succeeded`、`skipped`、`cancelled` | `pending`、`printing`、`succeeded`、`failed`、`result_unknown`、`skipped`、`cancelled` |
| `h9_print_attempt` | 打印尝试 | `created` | `succeeded`、`failed` | `created`、`submitted`、`succeeded`、`failed`、`result_unknown` |
| `h9_device_lease` | 设备租约 | `active` | `released` | `active`、`released` |

#### 组套实例转换事件

| 事件编码 | 从 | 到 | 守卫 |
|---|---|---|---|
| `documents_ready` | `waiting_documents` | `queued` | 必需单据和分类 PDF 全部就绪 |
| `agent_claimed` | `queued` | `preparing` | Agent 与设备租约原子领取成功 |
| `preload_verified` | `preparing` | `running` | 冻结清单、全部 PDF 和哈希完整校验，且当前 Agent 持有匹配 `assignment_epoch + lease_token` 的兼容 active 设备租约 |
| `prepare_released` | `preparing` | `queued` | 没有已提交尝试，领取和租约已安全释放 |
| `prepare_paused` | `preparing` | `paused` | 准备重试耗尽或策略要求人工处理 |
| `all_items_finished` | `running` | `completed` | 全部打印项为 `succeeded|skipped` |
| `safe_agent_failover` | `running` | `preparing` | 至少一项仍为 `pending`；旧 Agent 在线并持久化确认停止/交接；尚无已提交尝试或上一项已确认 `succeeded|skipped`；无在途/结果不明/待对账；冻结租约策略为 `safe_auto`，或授权用户提供原因并二次确认；同一事务递增 `assignment_epoch`、释放旧占用/租约、领取同站点兼容备用 Agent 与新 `lease_token` 并冻结分配，任一步失败整体回滚 |
| `execution_paused` | `running` | `paused` | 必需项失败或冻结策略要求暂停 |
| `operator_pause_applied` | `running` | `paused` | 已有授权、原因和幂等人工暂停请求；当前项已结束，且无在途/结果不明/待对账；不改变分配、优先级或入队时间 |
| `result_missing` | `running` | `awaiting_reconciliation` | 已提交尝试尚未收到可信结果 |
| `agent_connection_lost` | `running` | `awaiting_reconciliation` | Agent 进入 `suspected|offline`；服务端冻结原分配，禁止故障转移、新 Agent 推进、自动重打/释放及领取下一实例；旧 Agent 可按已冻结清单完成当前组套并在重连后整体对账 |
| `result_unknown` | `running` | `awaiting_manual_confirmation` | 本地后台结果不明或紧急停止无法确认 |
| `reconciled_continue` | `awaiting_reconciliation` | `running` | 对账一致且仍有待执行项 |
| `reconciled_complete` | `awaiting_reconciliation` | `completed` | 对账一致且全部项完成 |
| `reconciliation_conflict` | `awaiting_reconciliation` | `awaiting_manual_confirmation` | 已有后续动作或结果冲突 |
| `manual_resume` | `awaiting_manual_confirmation` | `running` | 授权人工确认结果并允许继续 |
| `manual_complete` | `awaiting_manual_confirmation` | `completed` | 授权人工确认全部完成 |
| `manual_fail` | `awaiting_manual_confirmation` | `failed` | 授权人工确认终止失败 |
| `manual_cancel` | `awaiting_manual_confirmation` | `cancelled` | 授权人工确认可安全取消 |
| `resume_preparing` | `paused` | `preparing` | 准备阶段恢复 |
| `resume_execution` | `paused` | `running` | 冻结文件仍完整有效并授权恢复/重试 |
| `terminate_failed` | `paused` | `failed` | 授权终止 |
| `cancel_suite` | `waiting_documents|queued|paused|preparing` | `cancelled` | 无在途尝试；`preparing` 还须先释放领取和租约 |

#### 打印项、尝试和租约转换事件

| 状态机 | 事件编码 | 从 | 到 | 守卫 |
|---|---|---|---|---|
| `h9_print_item` | `start_item` | `pending` | `printing` | 前序项已完成或允许跳过，且当前 Agent 持有匹配 `assignment_epoch + lease_token` 的兼容 active 设备租约 |
| `h9_print_item` | `item_succeeded` | `printing` | `succeeded` | 当前尝试成功 |
| `h9_print_item` | `item_failed` | `printing` | `failed` | 当前尝试明确失败 |
| `h9_print_item` | `item_result_unknown` | `printing` | `result_unknown` | 本地结果无法确认 |
| `h9_print_item` | `retry_item` | `failed` | `printing` | 创建新尝试且已授权重试 |
| `h9_print_item` | `skip_optional_item` | `failed` | `skipped` | 仅非必需项且冻结失败策略允许继续 |
| `h9_print_item` | `confirm_item_succeeded` | `result_unknown` | `succeeded` | 对账或授权人工确认成功 |
| `h9_print_item` | `confirm_item_failed` | `result_unknown` | `failed` | 对账或授权人工确认失败 |
| `h9_print_item` | `cancel_item` | `pending|failed` | `cancelled` | 组套已授权取消且无在途尝试 |
| `h9_print_attempt` | `submit_attempt` | `created` | `submitted` | 本地后台已接受提交 |
| `h9_print_attempt` | `attempt_succeeded` | `submitted|result_unknown` | `succeeded` | 后台结果或对账/人工证据确认成功 |
| `h9_print_attempt` | `attempt_failed` | `submitted|result_unknown` | `failed` | 后台结果或对账/人工证据确认失败 |
| `h9_print_attempt` | `attempt_result_unknown` | `submitted` | `result_unknown` | 无法确认本地结果 |
| `h9_device_lease` | `release_lease` | `active` | `released` | 硬守卫始终要求无 `printing`、`result_unknown` 或未决对账；满足硬守卫后，冻结策略须为 `safe_auto`，否则必须由具备专用权限的用户填写原因并二次确认 |

必需打印项永远不能跳过；失败后只能修复来源并重试、终止失败或安全取消组套。普通改派只允许
`queued` 状态；`preparing` 必须先通过 `prepare_released` 返回 `queued`。运行中只有满足
`safe_agent_failover` 全部守卫时才能切同站点兼容备用 Agent；结果不明和等待对账始终禁止
改派。每次重打创建新尝试，既有尝试不得复用、删除或回退。
`prepare_released`、`safe_agent_failover`、取消和其他释放路径都必须调用同一个
`release_lease` 领域迁移并满足其守卫，不得直接更新租约状态。

## 跨故事约束

1. **菜单**：套打中心使用 ADR-0039 定义的七个带前缀三级菜单；本故事只定义页面需求，
   页面、菜单、查询配置、真实数据 E2E 和截图在实现时同步落地。
2. **参数层级**：通用参数覆盖为客户、货主+仓库、全局；业务匹配规则留在对应配置页面。
3. **协议**：Agent 主动 HTTP 长轮询；普通 HTTP 仅限受控仓库局域网。首次激活只校验
   一次性注册码与 socket peer 精确源 IP；激活后机器端点同时校验 H9 机器凭据和该 IP，
   禁止信任 X-Forwarded-For/X-Real-IP。白名单只含心跳/运行和设备状态、长轮询/任务领取/
   清单、获授权 PDF 下载、打印结果/对账，以及 `/agent-releases`，具体以 ADR-0040 为准。
   Agent 必须直连由 WMS 进程终止 HTTP 的专用 listener，禁止经过改写源地址的 L7 代理、
   网关或 NAT。
   Web、H-FILE 通用上传/下载和其他模块仍必须使用 HTTPS。
   Agent 下载 PDF 走 H9 专用机器接口，由服务端校验任务和站点映射后解析 H-FILE 稳定文件
   引用；不得把 H-FILE 通用下载接口降级为 HTTP 或向 Agent 暴露长期文件 URL。
4. **分层**：服务端遵守 handler → service → domain/repository；本地 Agent 的 React UI
   不直接读取凭据、文件或调用 Windows 打印 API。
5. **不可冒充证据**：浏览器预览、PDF 生成、mock 打印机和本地静态截图都不能替代真实
   Windows Agent、打印机、纸盒、断网和产物人工核对的 S4 证据。
6. **首版兼容**：按 ADR-0038 不保留旧字段、旧 API 或旧 Agent 协议兼容层。
7. **多货主执行边界**：H9 物理打印站点显式映射多个 `owner_id + warehouse_id`，但不改变
   M1 仓库的单货主边界。Agent 使用站点机器身份，只能领取服务端已分配且属于本站映射的
   任务，不得通过请求参数自行选择货主。

## 测试维度覆盖

| 故事 | 最低重点 |
|---|---|
| US-H9-006/007 | L3 归集流程、L5 冻结一致性、L6 并发截单、L8 权限、L11 幂等 |
| US-H9-008/009 | L2 契约、L3 就绪策略、L4 文件缺失、L5 版本快照、L8 文件权限、L11 渲染幂等 |
| US-H9-010/011 | L3 顺序/重打、L4 设备失败、L6 队列和租约并发、L8 特权动作、L10 指标、L11 |
| US-H9-012/013 | L3 注册/分配/在线交接/断联入对账、L4 结果不明/离线禁转、L6 主备竞争/首项前断联/代次与租约 fencing、L8 机器凭据/跨 owner 全量授权/租约释放权限、L10 状态跃迁与安全事件、L11 全部写请求幂等键/机器幂等/遥测序号 |
| US-H9-014 | L3 断网恢复、L4 磁盘不足、L5 本地日志、L6 对账冲突、L10 告警、L11 |
| US-H9-015 | L3 恢复前禁用自动应用/启动更新/通道提升、L4 校验和完整包回退、L5 通道/版本原子变更、L8 更新鉴权/无发布者验真风险、L9 正式版后两阶段兼容、L10 更新状态、L11 写操作幂等 |

## 当前实现结论

- US-H9-001～005 的字段库、模板版本、浏览器预览/打印切片不等于本故事集实现。
- US-H9-006/007/008/009 已完成 V0-V3 开发软件验收；US-H9-010、012～015
  保持延期，继续按 outside-in TDD 分批实现。
- `print_document_category` 已由 US-H9-006 增加 `delivery_note` 受控分类，US-H9-008 已扩展
  药检单、发票及其 `source_mode = external_file`，不倒推扩大 US-M1-011 已验收范围。
- US-H9-008 的组套版本、四层解析、就绪/失败策略与实例快照已闭环；US-H9-009 已用
  `attachments` + `h9_document_file_bindings` 取代占位表，并闭环服务端分类 PDF、
  H-FILE、源版本/模板版本/SHA-256、留存、失败重试、部分/全部临时合并和独立权限审计。
- US-H9-009 的开发验收使用真实 PostgreSQL/API、独立 hiprint/Chromium Render Worker、
  浏览器与进程内 H-FILE 适配器；Worker 只渲染冻结模板和数据，不持有数据库、对象存储、
  队列或业务状态。开发 H2/staging 已提供 Worker 与 MinIO 编排，但正式环境 KMS、
  `aes256` 与对象存储恢复证据按环境分层保留，不影响本轮软件故事 PASS，也不得被开发
  截图抵扣。
- US-H9-010～015 涉及真实 Windows、打印机和网络，最终验收层级为 S4。
- ADR-0040 已关闭机器身份、普通 HTTP 白名单、一次性秘密重放和 pilot 授权边界；
  ADR-0041 已关闭归集硬边界、文件来源、必需项失败和安全故障转移的业务设计冲突；
  US-H9-010、012～015 的软件与适用外部证据仍未实现。

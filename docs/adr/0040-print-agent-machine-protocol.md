# ADR-0040：H9 Print Agent 机器身份与协议闭环

- 状态：Accepted
- 决策日期：2026-07-25
- 决策人：项目主人（2026-07-25 授权按 H9 复审建议修复并循环复审）
- 起草人：AI 助手
- 关联：H9 / H1 / H2 / H3 / H-FILE /
  [ADR-0024 鉴权模型](0024-auth-model.md) /
  [ADR-0039 打印组套与 Print Agent](0039-print-suite-and-agent.md)

---

## 背景

ADR-0039 已确定 Print Agent 使用受控仓库局域网普通 HTTP、独立机器凭据和精确源 IP，
但没有闭合以下边界：

- 首次激活前尚无机器凭据，不能同时满足“凭据 + IP”。
- 心跳、结果回报和对账不在原普通 HTTP 例外清单中。
- H1 外部 API Key 强制绑定单一货主，不能表达一个打印站点映射多个货主仓。
- 一次性秘密不能在幂等重放时再次返回明文。
- pilot 只在客户端要求用户登录，服务端尚不能阻止机器凭据直接选择 pilot。

本 ADR 只补 H9 机器协议和凭据边界，不建设通用机器身份平台，也不改变 ADR-0039 的业务
归集、组套、设备、更新完整性/无签名选择和受控局域网选择。

## 候选方案

### A. 放宽 H1 外部 API Key 的货主约束

复用最多，但会把 `auth_api_keys.owner_id` 从强约束改成多态可空字段，并让现有 owner-scoped
中间件承担站点身份，扩大 H1 回归面。

### B. H9 专用机器凭据，复用 H1 生命周期规则

H9 保存 Agent 专用凭据和 `MachineAuthContext`；复用既有密钥哈希、过期、轮换、吊销、
限流和审计规则，不复用 H1 的 owner-scoped 数据模型与中间件。

### C. 改用 HTTPS 或逐请求 HMAC

能加强传输和重放防护，但违反项目主人已确认的当前受控局域网“普通 HTTP + H9 机器凭据 +
精确 IP”边界，并增加证书或签名运维。

## 决策

采用方案 B。方案 C 保留为信任边界扩大后的升级路径。

### 1. 机器身份

- 每个 Agent 使用一个 H9 专用机器凭据，绑定 `agent_id + print_site_id` 和最小机器权限。
- H9 使用独立 `MachineAuthContext`，至少包含 `credential_id`、Agent、打印站点和权限标识；
  不含可由 Agent 选择的 `owner_id`，不构造普通 `AuthContext`。
- 任务货主由服务端已分配任务决定；服务端再次校验
  `owner_id + warehouse_id` 位于 Agent 所属站点映射内。
- H9 保持独立凭据记录，不放宽现有 `auth_api_keys.owner_id NOT NULL`。只复用既有密钥哈希、
  过期、轮换、吊销、失败锁定和审计规则，不新增通用凭据框架。
- Agent 明文密钥只返回一次，并由 Rust 端写入 Windows Credential Manager；React、本地日志、
  H2 diff 和业务数据目录不得读取或保存明文。

### 2. 首次激活

- 管理端使用密码学安全随机数生成器创建至少 128 bit 熵的短期、单次注册码；服务端只保存
  注册码哈希。
- 注册码绑定 `agent_id + print_site_id + 精确源 IP + expires_at`，不得调用其他接口。
- 激活端点是唯一的预凭据普通 HTTP 例外，只校验注册码、socket peer IP、有效期和未使用状态。
- 激活失败按请求中的 Agent 标识和 socket peer IP 双维度限流，达到受控阈值后临时锁定和
  告警；能够解析到 Agent/站点时按其映射 owner 写 H2，无法归属的探测只写脱敏安全日志和
  指标，不伪造 `owner_id`。响应不得泄露注册码、Agent 存在性或具体失败字段，日志和指标
  不得记录注册码或机器秘密。
- 激活成功后原子标记注册码已使用、创建机器凭据并返回一次明文密钥。
- 激活幂等只保证不创建第二个 Agent 或凭据；重复请求只返回资源元数据，不重显秘密。
  首次响应丢失时必须吊销凭据并重新注册。

### 3. 机器协议白名单

激活完成后，普通 HTTP 仅允许以下 H9 机器协议，并全部校验机器凭据与 socket peer IP：

1. 心跳、运行状态、磁盘和设备状态上报。
2. 长轮询、任务领取、冻结清单和任务状态读取。
3. 经任务、Agent、站点和货主仓校验后的 PDF 流式下载。
4. 打印尝试结果、任务结果、重连对账和冲突上报。
5. `/agent-releases` 的 stable 清单/包，以及具备有效 pilot 授权时的 pilot 清单/包。

初始安装包只能由已登录且具备 Agent 管理权限的 H1 用户通过常规 HTTPS Web 端点下载当前
stable 完整包，不依赖尚未创建的机器凭据，不得借初装选择 pilot 或复用普通 HTTP
`/agent-releases`。H-FILE 通用下载、Web 和其他模块不继承上述例外。

### 4. 源 IP 与重放边界

- H9 机器协议使用独立端口的 WMS 专用局域网 listener，并由承载 `MachineAuthContext` 的
  WMS 进程直接终止 HTTP；该 listener 只挂载本 ADR 的 H9 机器白名单，其他路由全部拒绝。
  Agent 必须直连该 listener，链路禁止会改写源地址的 L7 反向代理、网关或 NAT。
  若部署必须经过这些设施，精确 Agent IP 校验不成立，必须另建 ADR 选择可信源地址传递或
  传输保护，不能退化为校验代理 IP。
- H9 机器路由必须从该 listener 的服务端连接信息读取 socket peer IP，不读取
  `X-Forwarded-For`、`X-Real-IP` 或客户端自报地址。
- 机器路由不得直接复用当前 H1 owner-scoped API Key 中间件。
- 所有机器写请求都携带 `Idempotency-Key`，客户端重试必须复用原键。非遥测命令还绑定
  端点对应的资源标识和请求哈希；打印结果绑定任务和尝试。同一键不同载荷拒绝，已进入
  后续状态的尝试不得重复触发物理打印。心跳/磁盘/设备状态不落通用幂等行，实际去重和
  顺序校验按 §7 的单调序号处理。
- 普通 HTTP 不提供机密性，静态凭据仍可能在被控制的主机或网络中被窃取。项目主人接受在
  当前受控局域网内不增加 HTTPS 或逐请求 HMAC 的残余风险；跨网段、公网、访客无线网或
  安全事件发生后必须新建 ADR 重新选择传输保护。

### 5. pilot 授权

- stable 更新继续使用机器凭据与精确源 IP，不需要用户登录。
- pilot 由正常 H1 用户鉴权端点创建短期服务端授权，绑定
  `agent_id + startup_id + target_version + SHA-256`。
- 只有具备 Agent 管理权限的用户能创建授权；机器凭据本身不能创建或扩大 pilot 授权。
- Agent 下载 pilot 时仍使用机器凭据与源 IP，服务端额外校验上述授权；用户 token 和密码
  只在本次选择期间留在内存，不写入 Credential Manager、配置或日志。
- 授权在本次启动结束、目标变化、使用完成或到期后失效，下次启动仍回到 stable。

### 6. 凭据轮换

- 首版不增加在线双凭据切换协议。轮换复用“一次性注册码 + 激活”流程。
- 管理员先暂停 Agent 接单并确认没有运行中、结果不明或等待对账任务，再吊销旧机器凭据，
  签发新的短期单次注册码。
- Agent 使用新注册码从原精确源 IP 重新激活；Rust 成功写入 Windows Credential Manager 后
  才恢复接单。秘密响应丢失或本地写入失败时，继续保持暂停，吊销新凭据并重新注册。
- 整个轮换过程记录旧/新 `credential_id`、Agent、操作者和结果，不记录明文秘密。

### 7. 多货主授权、审计与幂等

- 物理打印站点至少存在一条有效 `owner_id + warehouse_id` 映射后才允许激活 Agent。
- Web 端站点/Agent 读取、映射变更、激活、轮换和单 Agent pilot 授权等站点级动作，由 H9
  service 通过 H1 port 校验操作者对映射前后 owner 并集逐一具备对应 Agent 管理权限。
  pilot 提升 stable 和全局最低版本变更还要求专用平台权限
  `h9.agent_version.global.write`，并校验全部未删除站点映射的 owner 并集；任一鉴权失败
  即整体 403，owner 并集为空时返回 409 且不得修改全局版本，只写脱敏安全日志/指标，
  不伪造 owner 写 H2。请求中的 `owner_id` 只表示目标，不得注入普通 `AuthContext` 或绕过
  全量校验。
- 任务/尝试动作的 H2 `owner_id` 取服务端任务快照。站点、Agent、凭据和安全动作按映射前后
  owner 并集分别写一条 H2 事件；全局版本动作按全部未删除站点映射 owner 分别写一条
  H2 事件。
  同一动作产生的这些事件复用 H2 既有 `request_id` 关联，不新增 `correlation_id`，不得写
  明文凭据。
- 心跳、磁盘容量和设备读数只更新 H9 运行状态/指标，不逐次写 H2；online/suspected/offline/
  paused 状态跃迁、阈值告警和安全事件才按受影响 owner 写 H2，避免无界审计洪泛。
- 激活失败只有在能解析到 Agent/站点时才按其映射 owner 写 H2；无法归属的探测进入脱敏
  安全日志、指标、限流和告警，不得为满足 H2 的非空约束伪造 owner。
- H9 机器事件映射到既有 `AuditActor` 时，`actor_id = agent_id`，`actor_name` 使用事件发生
  时冻结的 Agent 编码/名称，`jti = h9-machine:<credential_id>`；可解析的预凭据激活事件
  使用 `jti = h9-activation:<activation_code_id>`。Web 动作仍使用用户 `AuthContext`，
  不得把机器冒充成用户，也不得因此复用 H1 owner-scoped 中间件。
- H9 机器协议使用自有幂等记录，不放宽或复用 `owner_id NOT NULL` 的共享业务幂等表。激活
  作用域为 `activation_code_id + method + resource_id + Idempotency-Key + request_hash`；
  激活后作用域把 `activation_code_id` 换成 `credential_id`。同键变载荷拒绝。
- 高频遥测仍携带 `Idempotency-Key` 且重试复用原键，但不累积通用机器幂等行；服务端以
  `agent_id + boot_id + sequence` 单调更新，重复序号同载荷折叠、变载荷拒绝并告警，
  旧序号忽略。任务/尝试结果仍使用 H9 机器幂等记录，并由领域状态和唯一约束共同防止重复
  物理打印。

## 后果

- H1 外部 API Key 的单货主不变量不被削弱。
- H9 增加一套很小的机器凭据记录和专用中间件，但不增加通用认证平台。
- 激活响应丢失需要管理员重新注册，换取秘密不落库和不重显。
- 普通 HTTP 的窃听与来源验真风险继续存在，并由局域网边界、精确 IP、幂等和状态机缩小影响，
  但不能宣称已消除。

## 实施约束

1. 先冻结 H3 OpenAPI 机器路由白名单，再实现 handler → service → domain/repository。
2. L2/L4/L8/L11 至少覆盖弱/过期/已用注册码、激活限流、专用 listener 非白名单路由、伪造
   转发头、跨站点任务、跨 owner 权限缺失、空站点映射、秘密重放、机器幂等同键变载荷、
   全局版本权限缺失/空 owner 集、遥测缺失/重试变更 `Idempotency-Key`、遥测序号冲突和
   无 pilot 授权下载。
3. 机器凭据、注册码、用户 token、PDF 正文不得写入 H2 diff 或日志。
4. ADR-0039 继续作为历史业务基线；其中任何机器身份、激活、凭据、机器协议、专用 listener、
   端点白名单、更新鉴权、机器审计或机器幂等表述与本 ADR 冲突时，无论位于 §8、§10、§12、
   后果或实施约束，均由本 ADR 局部取代。业务编排细化另见 ADR-0041。

## 参考

- [H9 打印组套用户故事](../domain/user-stories-h9-print-orchestration.md)
- [ADR-0041 H9 打印编排细化](0041-print-orchestration-refinement.md)
- [基础设施技术规格](../infra/technical-specs.md)
- [ADR-0031 H-FILE](0031-file-attachment-capability.md)
- [ADR-0038 首版前兼容策略](0038-pre-v1-compatibility-policy.md)

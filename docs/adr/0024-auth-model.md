# ADR-0024：鉴权模型（JWT + AuthContext + 多租户隔离）

- 状态：Accepted
- 决策日期：2026-05-24
- 修订日期：2026-05-24（v0.2，修 review 标注风险 1+2）
- 决策人：项目主人
- 来源：SPIKE-001 验证结果（accept，10/10 测试通过）
- 关联：ADR-0001（技术栈）/ ADR-0010（错误码模式）/ ADR-0011（可观测）/ ADR-0013（配置/密钥）/ ADR-0026（跨端契约）

---

## 1. 背景

H1 横向能力（auth-tenant）覆盖：JWT 签发 / 验签 / 撤销 + 多租户 owner_id 隔离 + 权限码校验。
ADR-0001 选定 jsonwebtoken crate；clarifications C1 决定 PDA 离线 24h；docs/architecture-dependencies.md 把 H1 列为所有业务模块的硬前置。

但 ADR-0001 仅定方向，不定型——claim 字段、撤销机制、PDA 离线策略、错误码、与审计衔接等都没说清。
SPIKE-001 通过最小可行实现验证 5 个核心假设（H1-H5 全 accept），本 ADR 把验证结果固化为 Wave 1 W1.A 的硬约束。

---

## 2. 决策

### 2.1 JWT Claims 字段（锁定）

```rust
pub struct Claims {
    pub sub: String,         // user_id (UUID)，标准字段
    pub iat: usize,          // 签发时间（Unix 秒），标准字段；权限失效检测必需
    pub exp: usize,          // 过期时间（Unix 秒），标准字段
    pub jti: String,         // token unique ID（UUID v4），用于 blacklist 撤销
    pub owner_id: String,    // 货主 UUID（多租户隔离）
    pub user_name: String,   // 审计 actor 用
    pub permissions: Vec<String>,  // 权限码列表（内嵌；配 Redis 失效机制，见 §2.1.1）
}
```

**字段选型理由**：
- `sub` 用 user_id 不用 user_name：user_name 可改，user_id 不变；审计追溯靠 user_id 主键
- `iat` 必填：用于检测 token 签发后是否权限被改（详见 §2.1.1 混合失效模式）
- `jti` 必填：撤销机制依赖；不允许"无 jti 的 token"通过验签
- `owner_id` 内嵌 token：避免每次 API 查 user→owner 关系（性能 + 撤销与登录原子）
- `permissions` 内嵌 token + Redis 失效双保险：见 §2.1.1
- **不内嵌 user 详细信息**（部门 / 工号等）：变化频繁；前端按需查 /me

### 2.1.1 permissions 混合失效模式（v0.2 修订加入）

**问题**：spike-001 验证了 permissions 内嵌可行（性能优秀），但带来"权限变更后必须等 token 自然过期或主动撤销"的滞后。GSP 合规对"撤职 / 转岗 / 撤换权限"敏感（仓库主管被撤换的极端场景）。

**纯 stateless 方案** A（access TTL 缩到 5 分钟）= 12x 刷 token 请求开销，否决。
**完全不内嵌** 方案 B（每次查 RBAC 表）= 每请求 1 次 SQL，性能损 ~10%，否决。
**混合方案 C**（采纳）：

```
正常态：
  - JWT 内嵌 permissions（性能）
  - access TTL = 1h（既有设计）

权限变更（H1 故事，admin 改用户角色 / 撤销账号 / 转岗）：
  - 应用层：UPDATE 用户 RBAC 表
  - SAME txn：SET Redis key  user:{user_id}:permissions_changed_at = now_unix
  - 可选：Redis pub/sub 通知所有 axum 实例（W1.A 评估，spike 范围外）

AuthContext extractor 增加一步（在 §2.4 验签后、blacklist 检查前）：
  IF redis.get("user:" + claims.sub + ":permissions_changed_at") > claims.iat:
    → 拒绝，返 401 AUTH-009 PermissionsRevoked
    → 前端必须重新登录（新 token 得新 permissions）
```

**Trade-off**：

| 场景 | 性能 | 实时性 |
|------|------|--------|
| 正常请求 | 1 次 Redis GET（< 1ms 局域网）| 0 滞后 |
| 权限变更后下次请求 | 同上 + 1 次 401 + 重登 | < 1 秒 |
| Redis 故障 | 跳过此检查（降级，见 §2.3）| 退化到 max 1h 滞后 |

**与 §2.3 blacklist 共用 Redis 连接**：单次请求最多 2 次 Redis GET（jti blacklist + permissions_changed_at），可以合并到一个 pipeline 调用。

**实施细节留 Wave 1 W1.A**：Redis key TTL 设 access TTL × 2（覆盖最长 token 寿命）+ 自动清理过期 key。

### 2.2 双 token + PDA 离线策略

依据 `pda-offline-state.md` 文档：

| Token | 寿命 | 存储 |
|-------|------|------|
| Access Token | 1 小时（可配置） | 内存 + mmkv（PDA 加密持久化） |
| Refresh Token | 24 小时（可配置） | 仅 mmkv 加密；不入内存 |

5 状态机（PDA）：
- S1 在线工作 / S2 在线刷新中 / S3 离线工作 / S4 离线只读 / S5 锁定
- 转换矩阵详见 `spikes/spike-001-axum-jwt/pda-offline-state.md` §4

PC 端不需要状态机（始终在线，access 过期就重定向到登录）。

### 2.3 撤销机制

- **单机起步**：`Arc<RwLock<HashSet<String>>>`（jti），TTL 由 token exp 推断
- **生产化（Wave 1 W1.A）**：Redis SETEX，TTL = token 剩余有效期
- **撤销触发**：
  - 主动 logout
  - 管理端"踢下线"按钮（H1 故事）
  - 密码改/工号改/转岗（业务事件 → 撤销该 user 的所有 jti）
  - 长期未活跃自动撤销（Wave 4+ 评估）

#### 2.3.1 Redis 故障降级策略（v0.2 修订加入）

**风险**：blacklist + permissions_changed_at 都存 Redis，Redis 不可用 → 登录全停 = 不可接受。

**降级策略**：

```
extractor 拿到 jwt 验签通过的 claims 后：
  IF redis available (健康探针 / 上次操作 < 5s):
    1. GET user:{sub}:permissions_changed_at （§2.1.1）
       → 若 > claims.iat 则 401 AUTH-009
    2. SISMEMBER blacklist {jti} （§2.3）
       → 若命中则 401 AUTH-004
  ELSE:
    1. 跳过两项 Redis 检查
    2. 接受最长 access TTL（1h）的滞后窗口期
       - permissions 变更：等 token 自然过期
       - 已撤销 jti：等 token 自然过期或 admin 重启 Redis 后追溯
    3. 记录 WARN log + ADR-0011 可观测告警 P1
    4. 业务正常服务
```

**为什么接受 1h 滞后**：
- Redis 故障应是分钟级（哨兵自愈）/ 小时级（人工介入）罕见事件
- "停服 Redis 修好" 比"接受 1h 撤销窗口" 对业务影响大得多（GSP 仓库不能停业作业）
- 1h 窗口期内被撤换岗位的极端用户能做的破坏，远小于"全停服"的损失

**Redis HA 仍是 Wave 1 W1.A 优先**：本降级策略是"故障兜底"，不是"放任 Redis 单点"。Wave 1 W1.A 实施时：
- Redis 配 Sentinel 或 Cluster（运维层）
- 应用层用 redis-rs 内置连接池 + 重试 + 健康探针
- 告警阈值：连续 3 次失败 / 5s 无响应触发 P1

### 2.4 AuthContext extractor

`backend/crates/api/src/auth.rs`：

```rust
pub struct AuthContext {
    pub user_id: Uuid,
    pub user_name: String,
    pub owner_id: Uuid,
    pub permissions: Vec<String>,
    pub jti: String,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where AppState: FromRef<S>, S: Send + Sync,
{
    type Rejection = AuthError;
    async fn from_request_parts(...) -> Result<Self, Self::Rejection> {
        // 1. 取 Authorization Bearer
        // 2. jsonwebtoken::decode 验签 + 校验过期
        // 3. 检查 blacklist
        // 4. 构造 AuthContext
    }
}
```

实测 45 行（< 50 行假设成立）。Wave 1 落地时新增 5-10 行（含 tracing span / metric counter）仍可控。

### 2.5 业务 handler 编码模板（强制）

```rust
async fn list_items(
    State(state): State<AppState>,
    ctx: AuthContext,                  // 鉴权 + claim 提取自动完成
    Query(filter): Query<ItemFilter>,
) -> Result<Json<Vec<Item>>, ApiError> {
    // 必须用 ctx.owner_id 过滤（无论查询条件如何）
    let items = state.repo.list_items(ctx.owner_id, filter).await?;
    Ok(Json(items))
}
```

**红线**：
- 任何写操作必须用 `ctx.user_id` 作 audit actor
- 任何业务读 / 写必须以 `ctx.owner_id` 作隔离条件（即使前端没显式传）
- 权限校验用 `ctx.has_permission("xxx")`，错误返回 AUTH-005

### 2.6 错误码（入 docs/error-codes.md）

| 码 | HTTP | 触发 |
|---|------|------|
| AUTH-001 | 401 | 缺少 Authorization 头 |
| AUTH-002 | 401 | Authorization 格式错（必须 Bearer xxx）|
| AUTH-003 | 401 | token 无效或已过期（验签失败 / exp 过期 / 解析失败）|
| AUTH-004 | 401 | token 已撤销（blacklist 命中）|
| AUTH-005 | 403 | 权限不足 |
| AUTH-006 | 403 | 跨货主越权 |
| AUTH-007 | 401 | refresh_token 无效或过期（spike-001 未实现 refresh，留 Wave 1） |
| AUTH-008 | 401 | 密码错（统一返 AUTH-003 防用户枚举？待业务方决定） |
| AUTH-009 | 401 | permissions 已失效（用户权限变更后旧 token 仍持旧 permissions；客户端必须重新登录）|

### 2.7 多租户隔离边界

| 层 | 实现方式 | 风险 |
|---|---------|------|
| Token 层 | claim 内嵌 owner_id；登录时锁定 | 低（user.owner_id 不会变） |
| API 层 | handler 用 `ctx.owner_id` 过滤 query | 中（漏写过滤 = 越权）|
| 数据层 | 起步不引入 PostgreSQL RLS | 中 |
| 审计层 | SPIKE-002 每条 audit 记 actor_owner / row_owner，不一致告警 | — |

**"中"风险的缓解**：
- spike-002 audit 兜底（漏过滤会被 audit 对账抓出）
- W1.A 单元测试模板含"跨 owner 访问应 403"
- Wave 2+ 评估 PostgreSQL RLS（写新 ADR）

### 2.8 与 SPIKE-002 / SPIKE-006 衔接

| Spike | 衔接点 |
|-------|--------|
| SPIKE-002 H2 审计 | audit_event 表必含 `actor_id` `actor_name` `owner_id` `jti` 字段；来源全是 AuthContext |
| SPIKE-006 错误码（拟） | AUTH-001..008 进 docs/error-codes.md（统一注册），前端按 code 切换提示 |
| ADR-0026 跨端契约 | LoginRequest / LoginResponse 用 utoipa::ToSchema；前端 packages/api-client 自动生成类型 |

---

## 3. 候选方案

### A. 本决策（JWT + AuthContext + middleware 注入 + Redis blacklist）— 接受

理由：SPIKE-001 全 5 假设 accept；代码量可控（45 行 extractor）；社区标准模式；与 ADR-0001 一致。

### B. session-cookie + server-side session store — 否决

理由：
- PDA 端 cookie 跨域复杂
- 服务端 session 与 stateless API 模式冲突
- 不便于 microservice 扩展（虽然当前是单服务）
- ADR-0001 已选 JWT，无理由翻案

### C. axum-login / tower-sessions 高层封装 — 否决

理由：
- 透明度低（出问题难调试）
- 自实现 45 行已够，封装收益小
- spike-001 实测无技术阻塞

### D. PostgreSQL Row-Level Security 起步引入 — 推迟

理由：
- 学习曲线陡（policy DSL）
- 调试和审计难度大
- 当前 SQLx 与 RLS 的协作模式未定型（SPIKE-004 没覆盖）
- W1.A 起步用 middleware 注入 + audit 兜底足够；Wave 2+ 评估硬化

### E. 完全 stateless（不引入 blacklist） — 否决

理由：GSP 撤销岗位/解雇的硬要求 = 立即登出；纯 access TTL 失败（最长等到 access 自然过期 = 1h，不可接受）。

---

## 4. 实施 checklist（Wave 1 W1.A 启动时）

- [ ] `backend/crates/api/src/auth.rs` 含 Claims / AuthContext / AuthError
- [ ] `backend/crates/api/src/handlers/auth/{login,refresh,logout}.rs` 三个 handler
- [ ] Redis 引入：`backend/crates/infra/src/redis.rs` + AppState 加 `blacklist: redis::Client`
- [ ] JWT secret 走配置中心（ADR-0013）：`config/auth.yaml` 中 `jwt.secret_ref` 引用密钥源
- [ ] `docs/error-codes.md` 加 AUTH-001..008
- [ ] handler 模板：每个写 handler 必须用 `ctx.user_id` 作 audit actor
- [ ] 单元测试模板：每个业务 handler 必须含"跨 owner 越权应 403"测试
- [ ] H2 审计：audit_event 表 schema 含 `actor_id` `actor_name` `owner_id` `jti`（依赖 SPIKE-002 决策）
- [ ] 集成测试：从 SPIKE-001 tests/auth.rs 迁移 10 个测试到 backend/tests/

---

## 5. 后果

### 正面

- **handler 代码极简**：`async fn handler(State, ctx: AuthContext, ...)` 一行拿到全部鉴权信息
- **跨端一致**：access/refresh token 与 PDA 离线状态机统一在本 ADR 决定，后续所有端复用
- **审计可追溯**：每条业务操作都能定位到 user + owner + jti，与 SPIKE-002 衔接
- **撤销立即生效**：blacklist 命中后下次请求即 401

### 负面

- **handler 漏写 `ctx.owner_id` 过滤会越权**：必须靠模板 + 测试 + 审计三重保护
- **token 内嵌 permissions 导致权限变更滞后**：用户改权限后需要等 access 过期或主动撤销才生效；缓解：高频权限变更场景缩短 access TTL（5 分钟）
- **JWT secret 是单点**：泄露后所有 token 失效；缓解：走配置中心 + 轮换流程（ADR-0013 + ADR-0014）

### 风险

- **handler 不用 ctx 直接拿 owner_id 而用 query 参数 owner_id** → 越权写入：W1.A 编码模板 + grep 治理脚本（禁止 query 接收 owner_id）
- **Redis 故障导致登录失败**：Redis 是 critical path；ADR-0011 可观测体系必须监控 + 告警 P1
- **PDA mmkv 失窃裸露 token**：丢机风险；缓解：mmkv 加密 + S5 锁定快速生效（24h 内）

---

## 6. 关联文档

- [SPIKE-001 验证记录](../spikes/spike-001-axum-jwt.md)
- [PDA 离线状态机](../../spikes/spike-001-axum-jwt/pda-offline-state.md)
- [Spike 代码](../../spikes/spike-001-axum-jwt/)
- [ADR-0001 技术栈](0001-tech-stack.md)
- [ADR-0010 错误码](0010-error-codes.md)
- [ADR-0011 可观测](0011-observability.md)
- [ADR-0013 配置/密钥](0013-config-secrets.md)
- [ADR-0026 跨端契约](0026-cross-end-contract-pipeline.md)（同期产出，spike 链上下游）


---

## 7. 修订记录

### v0.2 — 2026-05-24（review 后修风险 1+2）

针对 Wave 0.5 退出前的集中 review 标注的两处风险（详 retro 与 commit f2614bb 后的 review 报告）：

**风险 1：permissions 内嵌 → 权限变更滞后**
- 修：§2.1 Claims 加 `iat` 字段；§2.1.1 新增"混合失效模式"段
  - JWT 仍内嵌 permissions（性能）
  - 权限变更时 SET Redis `user:{user_id}:permissions_changed_at = now`（与 RBAC 表 UPDATE 同事务）
  - extractor 检查该值 > claims.iat 则 401 AUTH-009 PermissionsRevoked
  - 与 §2.3 blacklist 共用 Redis 连接，pipeline 优化
- 加 §2.6 错误码表新增 AUTH-009

**风险 2：Redis 是 critical path**
- 修：§2.3.1 新增"Redis 故障降级策略"段
  - Redis 不可用时跳过 blacklist + permissions_changed_at 检查
  - 接受 ≤ 1h（access TTL）滞后窗口期；记录 WARN log + ADR-0011 P1 告警
  - 业务正常服务（不停服）
- Redis HA（Sentinel/Cluster）+ 连接池 + 健康探针仍是 W1.A 优先

### v0.1 — 2026-05-24（初版，SPIKE-001 验证后产出）

详见 §1-§6。

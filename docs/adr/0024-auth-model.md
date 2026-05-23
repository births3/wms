# ADR-0024：鉴权模型（JWT + AuthContext + 多租户隔离）

- 状态：Proposed
- 决策日期：2026-05-24
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
    pub exp: usize,          // 过期时间，Unix 秒，标准字段
    pub jti: String,         // token unique ID（UUID v4），用于 blacklist 撤销
    pub owner_id: String,    // 货主 UUID（多租户隔离）
    pub user_name: String,   // 审计 actor 用
    pub permissions: Vec<String>,  // 权限码列表
}
```

**字段选型理由**：
- `sub` 用 user_id 不用 user_name：user_name 可改，user_id 不变；审计追溯靠 user_id 主键
- `jti` 必填：撤销机制依赖；不允许"无 jti 的 token"通过验签
- `owner_id` 内嵌 token：避免每次 API 查 user→owner 关系（性能 + 撤销与登录原子）
- `permissions` 内嵌 token：避免 RBAC 表查询 N 次；权限变更需要等 token 自然过期或主动撤销
- **不内嵌 user 详细信息**（部门 / 工号等）：变化频繁；前端按需查 /me

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

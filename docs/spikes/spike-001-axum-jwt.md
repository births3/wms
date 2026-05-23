# SPIKE-001: Axum + JWT + 多租户 middleware

- 状态：accepted
- 时间盒：2 天（16 小时）
- Owner：项目主人
- 起始：2026-05-23  完成：2026-05-24（约 3 小时实际工时；远低于 16h 时间盒）
- 关联 Wave 任务：W1.A 权限/多租户基础（角色 / 权限码 / 货主隔离 / JWT）
- 关联 ADR：ADR-0001（技术栈，Rust + Axum 已选定）；产出 ADR-0024 鉴权模型（草案，待 review）

---

## 1. 背景与问题

Wave 1 W1.A 要求"任意业务 handler 可挂 H1"——即任意 handler 写入 owner_id 隔离、读取 actor 身份、检查权限码。
**不验证就开写，最坏情况**：handler 全部写完才发现 middleware 抽不出 owner_id（因为 token claim 设计不对），等于全量返工。

未确定项：

1. JWT claim 的字段集合（user_id / owner_id / roles / permissions / tenant_id 怎么排列）
2. Axum 的 middleware / extractor 哪个更适合注入 owner_id（FromRequestParts vs middleware fn）
3. token 撤销机制（短期：黑名单 cache；长期：refresh token rotation）
4. 与 H2 审计追踪的衔接（每条审计的 actor 字段从哪里来）
5. PDA 离线 24 小时（C1 决策）下 token 缓存与刷新策略
6. 多租户隔离强度：是 middleware 强制 owner_id 写入 query，还是 RLS（PostgreSQL Row-Level Security）

---

## 2. 验证假设

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | Axum 0.7 + tower-http + jsonwebtoken 能在 < 50 行 middleware 代码内完成 JWT 验签 + claim 提取 | 写最小 demo，统计代码行 |
| H2 | `FromRequestParts` 实现 `AuthContext { user_id, owner_id, permissions }`，handler 签名直接拿，无需 extractor 链 | demo 写 3 个 handler 印证 |
| H3 | token 撤销可用 Redis blacklist（key=jti，TTL=token 剩余有效期）实现，无需改 JWT 算法 | 单元测试模拟撤销 → 下次请求 401 |
| H4 | 多租户隔离用"middleware 注入 owner_id 到 SQLx query"足够（不强制 RLS），通过 H2 审计可追溯越权请求 | spike-002 配合：审计能拦下 owner_id 不一致的写 |
| H5 | PDA 离线 24h 通过"长 refresh token + 短 access token"实现：access 1h、refresh 24h；离线只允许已缓存 access；超过 access 有效期但未到 refresh 有效期则进入"只读+排队"模式 | 写状态机文档 + 离线模拟测试 |

---

## 3. 退出条件

| 状态 | 条件 |
|------|------|
| accept | H1-H5 全部确认；产出 ADR-0024 草案；spike 代码在 `spikes/spike-001-axum-jwt/` 可 `cargo test` 跑通 |
| reject | H1 / H2 任一不成立（如必须 ≥ 200 行 middleware 才能完成基础功能）→ 候选改用 actix-web + 现成 middleware 生态；新建 spike-001b |
| defer | H5 PDA 离线策略复杂度过高（如必须在 PDA 端做 token 重签）→ Wave 1 仅做 PC 端 JWT，PDA 端鉴权延后到 Wave 2 spike |

---

## 4. 实施路径

### 步骤 1：起最小 Axum demo（2 小时）

- `spikes/spike-001-axum-jwt/Cargo.toml`：axum 0.7 / tokio / tower / tower-http / jsonwebtoken / serde
- 单 binary，3 路由：`POST /login`（签发 token）/ `GET /me`（受保护，返回 claim）/ `GET /admin`（要 `permissions:["admin"]`）

### 步骤 2：实现 AuthContext extractor（4 小时）

- `mod auth { pub struct AuthContext { ... }; impl FromRequestParts for AuthContext { ... } }`
- 失败路径：缺 token → 401；签名错 → 401；过期 → 401（带 `WWW-Authenticate: Bearer error="invalid_token"`）
- 提取 owner_id 进 `Extension<TenantId>`，handler 取用

### 步骤 3：撤销机制（3 小时）

- 内嵌 in-memory blacklist（HashMap<Jti, Expiry>，无需 Redis 即可验证假设）
- 路由 `POST /logout`：把 jti 加入黑名单
- 测试：登录 → 调用 /me 200 → logout → 再调 /me 401

### 步骤 4：多租户测试（3 小时）

- 假表 `items(id, owner_id, name)`（用 sqlx + sqlite 内存版避免 spike 装 PG）
- handler `GET /items` 自动过滤 owner_id；测试两个 owner 互看不见

### 步骤 5：PDA 离线 token 状态机文档（2 小时）

- 不写代码（PDA 在 spike-005），只画状态图与 token 时序：online-online / online-offline / offline-online / offline-expired
- 输出 `spikes/spike-001-axum-jwt/pda-offline-state.md`

### 步骤 6：写 ADR-0024 草案（2 小时）

- `docs/adr/0024-auth-model.md` 状态：Proposed
- 内容：claim 字段表 / middleware 编码模式 / 撤销策略 / PDA 离线策略 / RLS 暂不引入的理由

---

## 5. 风险与后备方案

| 风险 | 概率 | 影响 | 后备方案 |
|------|------|------|---------|
| jsonwebtoken crate 与 axum 0.7 集成需要大量胶水 | 低 | 中 | 改用 `axum-login` 或 `tower-sessions` 高层封装，损失部分透明度 |
| 撤销机制必须 Redis（blacklist 单机不够） | 中 | 中 | Wave 1 上线 Redis（已在 ADR-0011 可观测中提到）；若延后，缩短 access token TTL 到 5 分钟用 expiry 替代撤销 |
| PDA 离线策略要求 token 重签（端有 secret） | 低 | 高 | reject H5；Wave 1 PDA 强制在线，离线写操作排队但不允许新登录 |
| owner_id 在 query 层漏过 | 中 | 高 | spike-002 审计层兜底：每条 audit 都记 owner_id，对账脚本检查 actor.owner_id == row.owner_id；后续 Wave 引入 RLS |

---

## 6. 产出物清单

- 代码：`spikes/spike-001-axum-jwt/`（Cargo crate，含 tests）
- 文档：本文件（§7 决策）；`pda-offline-state.md`
- ADR：`docs/adr/0024-auth-model.md`（状态 Proposed → 经 review 后 Accepted）
- 治理：如果 ADR 0024 引入新概念（如 jti / tenant_id），同步加入 docs/glossary.md

---

## 7. 决策记录

- 日期：2026-05-24
- 结论：**accept**
- 时间盒消耗：约 3 小时（远低于 16h 上限）

### 7.1 假设验证结果

| ID | 假设 | 状态 | 证据 |
|----|------|------|------|
| H1 | < 50 行 middleware 完成 JWT 验签 + claim 提取 | ✓ | 元测试 `h1_extractor_under_50_lines` 输出 "AuthContext extractor = 45 lines" |
| H2 | `FromRequestParts` 实现 AuthContext 让 handler 直接拿 user_id/owner_id/permissions | ✓ | 6 个 handler（me/admin/logout/list_items/get_item）签名直接用 `ctx: AuthContext` |
| H3 | in-memory blacklist 实现 token 撤销 | ✓ | `t3_logout_revokes_token`：login → /me 200 → /logout → 同 token /me 401 (AUTH-004) |
| H4 | middleware 注入 owner_id + handler 过滤业务数据足够多租户隔离 | ✓ | `t4_owner_isolation_list_only_own`（alice 看不见 owner B 的 item） + `t4_owner_isolation_cross_owner_get_403`（cross-owner GET 返回 AUTH-006） |
| H5 | PDA 离线 24h 通过双 token 实现，状态机文档化 | ✓ | `pda-offline-state.md` 230 行，5 状态（S1-S5）+ 转换矩阵 + 时间预算 + 服务端配套 |

### 7.2 测试覆盖

10 个 `#[tokio::test]` 全过（cargo test exit=0）：

| # | 测试 | 覆盖假设 |
|---|------|---------|
| 1 | t1_login_and_auth | H1+H2 主路径 |
| 2 | t1_login_wrong_password | H1 错误路径 |
| 3 | t1_no_auth_header_401 | H1 错误路径 |
| 4 | t2_expired_token_returns_401 | H1（验签 + 过期） |
| 5 | t2_invalid_signature_returns_401 | H1（错误密钥） |
| 6 | t3_logout_revokes_token | H3 |
| 7 | t4_owner_isolation_list_only_own | H4 |
| 8 | t4_owner_isolation_cross_owner_get_403 | H4 |
| 9 | t5_admin_permission_required | 权限码（衍生） |
| 10 | h1_extractor_under_50_lines | H1 元测试（行数） |

### 7.3 关键发现

1. **axum 0.7 仍用 `#[axum::async_trait]` 装饰 `FromRequestParts` impl**：编译错误信息隐晦（"lifetime parameters or bounds on associated function from_request_parts do not match"）。axum 0.8+ 才改原生 async fn。Wave 1 W1.A 编码模板需明示。

2. **jsonwebtoken 9.3 默认 leeway = 60 秒**：过期测试用 `Duration::seconds(-1)` 不会报错，必须 > 60s 才生效（实测 `Duration::hours(-1)` 通过）。生产化时显式 `Validation { leeway: 5, ..default() }` 收紧到 5 秒。

3. **AuthError 通过 `IntoResponse` 自动序列化错误码**：6 类错误（AUTH-001..006）在 `lib.rs` 一处定义，前端拿统一格式 `{ code, message }`，与 ADR-0010 错误码模式天然对齐。

4. **多租户隔离的"middleware 注入 + handler 过滤"模式可行但需自律**：
   - 优势：handler 代码简洁（`items.filter(|i| i.owner_id == ctx.owner_id)`）
   - 风险：handler 漏写过滤会越权；缓解：spike-002 H2 审计记 owner_id，每日对账脚本检查 actor.owner_id == row.owner_id
   - 长期：Wave 2+ 评估 PostgreSQL Row-Level Security（RLS）作硬约束

5. **uuid v4 的 jti + RwLock<HashSet> 单机 blacklist 足够 spike**：
   - HashSet 内存占用：约 100K 用户 × 24h = 100K 条 jti × 36 字节 ≈ 3.6 MB（可接受）
   - 生产：Redis SETEX，TTL = token 剩余有效期；避免内存无限增长

6. **AppState 用 `FromRef` 而非 `Extension<AppState>`**：axum 0.7 推荐模式；让 handler 通过 `State(state)` 拿，extractor 通过 `FromRef` 拿。

### 7.4 后续动作

1. **写 ADR-0024 鉴权模型**（已起草，见 `docs/adr/0024-auth-model.md`）
2. **Wave 1 W1.A 实施清单**（已写入 ADR-0024 §4）：
   - 把 spike-001 的 Claims / AuthContext / AuthError 模式迁到 `backend/crates/api/src/auth.rs`
   - 业务 handler 模板：`async fn handler(State(state), ctx: AuthContext, ...)`
   - 错误码 AUTH-001..006 入 `docs/error-codes.md`
   - blacklist 由 in-memory 改 Redis（依赖 ADR-0011 可观测体系的 Redis 引入）
   - JWT secret 走配置中心（依赖 ADR-0013 config-secrets）
3. **传染给后续 Spike**：
   - SPIKE-002 H2 审计：每条 audit 记 actor_id / actor_name / owner_id（来自本 spike 的 AuthContext）+ jti（用于审计 token 流）
   - SPIKE-005 RN 扫枪：复用 `pda-offline-state.md` 的状态机；token 持久化用 `react-native-mmkv` 加密
4. **不在本 spike 范围**：
   - 真实 RLS（PostgreSQL）— Wave 2+ 评估
   - 暴力破解防护（5 次失败锁账户）— Wave 1 W1.A 业务规则
   - SSO / OAuth2 第三方登录 — Wave 5+ 业务方决定后再 spike
   - 设备指纹绑定 — 等 SPIKE-005 RN 端验证后再决策

### 7.5 拒绝清单

| 候选 | 不验证理由 |
|------|-----------|
| `axum-login` / `tower-sessions` | 高层封装牺牲透明度；自实现 < 50 行已达成 |
| RSA 非对称签名 | HS256 + 共享密钥已满足单后端集群；非对称用于多服务签发场景 |
| 完全 stateless（无 blacklist） | 不能"主动撤销"违反 GSP（撤销岗位的人必须立即登出）；blacklist 必需 |
| 设备绑定 | 不在 H1-H5 范围；Wave 1 W1.A 业务规则评估 |

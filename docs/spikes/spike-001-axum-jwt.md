# SPIKE-001: Axum + JWT + 多租户 middleware

- 状态：起草
- 时间盒：2 天（16 小时）
- Owner：项目主人
- 起始：— 完成：—
- 关联 Wave 任务：W1.A 权限/多租户基础（角色 / 权限码 / 货主隔离 / JWT）
- 关联 ADR：ADR-0001（技术栈，Rust + Axum 已选定）；拟产出 ADR-0024 鉴权模型

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

> spike 完成后填写。

- 日期：—
- 结论：—
- 关键发现：—
- 后续动作：—

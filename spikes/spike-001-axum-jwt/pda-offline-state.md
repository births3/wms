# PDA 离线 token 状态机

> SPIKE-001 H5 验证：PDA 离线 24h 通过"长 refresh + 短 access"实现，
> 状态机文档化（不写代码；PDA 端代码在 SPIKE-005）。
> 关联：clarifications.md C1（PDA 离线 24h 默认可配置）

---

## 1. 双 token 设计

| Token | 寿命 | 存储 | 用途 |
|-------|------|------|------|
| Access Token (JWT) | 1 小时（可配置） | 内存 + mmkv 持久化 | 每次 API 请求带 |
| Refresh Token | 24 小时（可配置）| 仅 mmkv 加密存储；不入内存 cache | access 过期时换新 access |

**为什么不直接 24h access**：
- access 长寿命撤销代价高（blacklist 须存 24h）
- access 长寿命 = 失窃后影响窗口大
- 业界标准模式（OAuth2 / OpenID Connect）

**为什么用 refresh 而不是直接重登**：
- 重登要求用户输入工号 + 密码 / 工牌扫码
- PDA 现场作业，不可能每小时打断流程让用户重登
- refresh 可在后台静默换 access

---

## 2. 状态定义

```
┌────────────────────────────────────────────────────────────────┐
│                           ONLINE                                │
│  ┌─────────────────┐   access expire    ┌──────────────────┐  │
│  │ S1: 在线工作    │ ────────────────→ │ S2: 在线刷新中   │  │
│  │ (access 有效)   │                    │ (调 /refresh)    │  │
│  └─────────────────┘                    └──────────────────┘  │
│         ↑                                       │              │
│         │ refresh 成功 → 新 access              │              │
│         └───────────────────────────────────────┘              │
└────────────────────────────────────────────────────────────────┘
              ↕ network on/off               ↕ refresh 过期
┌────────────────────────────────────────────────────────────────┐
│                          OFFLINE                                │
│  ┌─────────────────┐   access expire    ┌──────────────────┐  │
│  │ S3: 离线工作    │ ────────────────→ │ S4: 离线只读     │  │
│  │ (access 有效)   │                    │ (access 过期但   │  │
│  │                 │                    │  refresh 未过期) │  │
│  └─────────────────┘                    └──────────────────┘  │
│         │                                       │              │
│         │ refresh 也过期                        │              │
│         ▼                                       ▼              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ S5: 锁定（必须联网重登；本地数据保留待恢复）              │  │
│  └────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

---

## 3. 状态详细

### S1：在线工作（正常态）
- access token 有效（exp > now）
- 网络在线
- 行为：所有 API 直发；扫码立即得到响应；状态实时同步
- 任务队列：空（或仅有"待提交但服务端处理中"的项）

### S2：在线刷新中
- 触发：access 即将过期（< 5 分钟）或刚过期
- 行为：后台 POST /auth/refresh，带 refresh_token
- 成功：得到新 access → 回 S1
- 失败（refresh 也过期）：→ S5
- UI：可选显示 "正在刷新登录" 微提示，不阻塞用户操作

### S3：离线工作（access 仍有效）
- 触发：网络断开（飞行模式 / WiFi 切换 / 仓库信号死角）
- 行为：
  - 写操作（收货 / 验收 / 上架）：写入本地任务队列（mmkv），UI 显示"已暂存（X 项待同步）"
  - 读操作：用本地缓存（最后一次成功响应）
  - 扫码：本地 GS1 解析 + 资质校验（缓存的资质表）
- 任务队列：累积 N 项，每项带 idempotency_key (UUID v4)
- 联网恢复：自动 flush 任务队列；服务端按 idempotency_key 去重

### S4：离线只读（access 过期，refresh 未过期）
- 触发：S3 状态下 access 自然过期
- 行为：
  - 写操作：**仍允许入队**（写本地，标 status=pending）
  - 读操作：仅本地缓存
  - UI：顶部 banner 警示 "登录将于 N 小时后过期，请尽快联网"
- 联网恢复：先调 /auth/refresh 换新 access → 再 flush 任务队列

### S5：锁定
- 触发：S2 refresh 失败 / S4 refresh 也过期
- 行为：
  - **不允许新业务操作**（扫码 / 收货 / 出库全部阻塞）
  - 显示重登界面（工号 / 工牌扫码）
  - **本地任务队列保留**（不允许清空，避免数据丢失）
  - 重登成功后：用户身份必须与队列内 actor 匹配；不匹配则要求人工介入
- 安全考虑：S5 是丢卡 / 转岗 / 撤销账号场景的唯一硬阻塞点

---

## 4. 状态转换矩阵

| from | event | to | 备注 |
|------|-------|-----|------|
| S1 | access 即将过期 | S2 | 后台触发，用户无感 |
| S1 | 网络断开 | S3 | UI banner 切换 |
| S2 | refresh 成功 | S1 | 新 access；记录续期事件到 audit |
| S2 | refresh 失败 | S5 | 触发重登；保留任务队列 |
| S3 | 网络恢复 | S1 | flush 队列；UI banner 消失 |
| S3 | access 过期 | S4 | UI 警示 banner |
| S4 | 网络恢复 | S2（继而 S1） | 先 refresh 再 flush |
| S4 | refresh 过期 | S5 | 锁定；保留队列 |
| S5 | 重登成功 | S1 | actor 校验后 flush 队列 |
| S5 | 重登失败 N 次 | S5 | 不离开；可触发账号锁定（H1 暴力破解防护，Wave 1） |

---

## 5. 时间预算示例

业务方默认配置（C1 决策）：
- access TTL: 1 小时
- refresh TTL: 24 小时

**最坏情况线 1**：仓库 8:00 上班登录，4 小时持续掉线作业，12:00 联网

```
8:00  S1 登录（access 8:00-9:00 / refresh 8:00-次日 8:00）
8:30  S3 网络断开
9:00  S4 access 过期（自动转 S4，不需服务端交互）
12:00 S2 联网，调 refresh，换新 access（refresh 仍有效，至次日 8:00）
12:01 S1 flush 任务队列（4 小时累积的扫码记录）
```

**最坏情况线 2**：丢卡 / 转岗（S5 锁定）

```
8:00  S1 alice 登录
9:00  S2 access 过期，refresh 成功
... 
次日 7:55 S4 access 过期（约 23h 后）
次日 8:01 S5 refresh 也过期
       本地任务队列含 N 条 alice 扫码记录
       PDA 锁定，等待 alice 重登
       
情景 A: alice 当天补登 → 队列以 alice 身份提交
情景 B: alice 丢卡 → 主管介入，将队列标记 "alice 离职前最后操作"
       由仓库主管手工补提交（保留 actor=alice + 备注=主管复核）
```

---

## 6. 服务端配套

### 6.1 /auth/login

```
POST /auth/login
{ user_name, password | scan_code }
→ 200 { access_token, refresh_token, access_expires_in: 3600,
        refresh_expires_in: 86400 }
```

### 6.2 /auth/refresh

```
POST /auth/refresh
{ refresh_token }
→ 200 { access_token, refresh_token (rotated), ... }
   401 { code: "AUTH-007", message: "refresh_token 无效或过期" }
```

**rotation 策略**：每次 refresh 都签发新 refresh_token，旧 refresh_token 立即失效（防重放）。

### 6.3 /auth/logout

```
POST /auth/logout
Authorization: Bearer <access_token>
→ 200 { revoked_jti }
   服务端：access jti 入 blacklist；refresh_token 也作废（关联 user 的所有 refresh）
```

### 6.4 服务端 blacklist 持久化

- 当前 SPIKE-001 用 in-memory HashSet（单机）
- 生产化方案：Redis SETEX，TTL = token 剩余有效期；jti 自动过期
- 详细见 ADR-0024（拟产出）

---

## 7. PDA 端实现要点（spike-005 代码）

| 项 | 设计 |
|---|---|
| Token 持久化 | `react-native-mmkv`（加密；丢卡时数据不裸露在文件系统）|
| 任务队列 | `mmkv` + idempotency_key (UUID v4) + retries 计数 |
| 状态机驱动 | `zustand` store + `useEffect` 监听网络变化 / token 过期 |
| 自动 refresh | axios interceptor / fetch wrapper：401 时尝试 refresh，成功重试原请求 |
| Banner UI | 顶部 sticky banner，颜色按状态（绿 / 黄 / 红）|

---

## 8. 与 SPIKE-002 审计追踪的衔接

每次 token 状态变化都生成审计事件：

| 事件 | actor | action | 备注 |
|------|-------|--------|------|
| login | user | `auth.login` | IP / device / user_agent |
| refresh | user | `auth.refresh` | 新 access jti |
| logout | user | `auth.logout` | 撤销的 jti |
| token_expired | user | `auth.token_expired` | 自然过期，非用户主动 |
| token_revoked | admin/system | `auth.token_revoked` | 强制踢下线 |
| relogin_after_lock | user | `auth.relogin_after_lock` | S5 → S1，含 lock 期间累积任务数 |

审计写入路径见 SPIKE-002 决策记录。

---

## 9. 拒绝清单（spike 不验证）

| 候选 | 不验证理由 |
|------|-----------|
| WebAuthn / FIDO2 硬件 token | PDA 仓库现场不实用；超出 GSP 要求 |
| 多设备同时登录限制 | 业务规则待 H1 故事补充；当前 SPIKE 假设单 PDA 单用户 |
| 时间漂移容差 | jsonwebtoken 默认 leeway=60s 已足够；时区不一致由后端 UTC 统一处理 |
| 离线 push 通知 | 需要 FCM / APNs 基建；超出 PDA 离线 token 范围 |
| Biometric unlock（指纹）| Wave 4+ 评估 |

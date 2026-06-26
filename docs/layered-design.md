# 前后端分层设计规范

> 本文档定义 WMS 生产代码的分层边界。代码书写细节见 `docs/coding-standards.md`，前端组件细节见 `docs/frontend-coding-standards.md`，原型转生产见 `docs/adr/0029-frontend-as-prototype-workflow.md` 和 `docs/prototypes/prototype-to-production.md`。

---

## 1. 目标

分层设计解决 4 个问题：

1. **业务规则可定位**：看到规则就知道属于 domain / app service / frontend page 哪一层。
2. **依赖方向可审查**：底层不反向依赖上层，避免改 UI 牵动领域逻辑。
3. **测试边界清楚**：handler 测契约和权限，service 测业务流程，repository 测持久化一致性。
4. **原型不污染生产**：prototype 表达流程，production 重新按 API、权限、审计、幂等落地。

本规范适用于：

| 范围 | 适用 |
|---|---|
| 后端 | `backend/crates/*`、`backend/migrations/*` |
| Web 管理端 | `apps/web-admin/*`、`packages/api-client/*`、`packages/ui/*` |
| PDA | `apps/pda-mobile/*`、共享契约和 UI 约束 |
| 原型 | `prototypes/*`，但原型只承担走查，不承担生产分层 |

---

## 2. 总原则

### 2.1 依赖方向

后端：

```text
bin/runtime -> api handler -> app service -> domain
                              app service -> repository trait -> repository impl -> PostgreSQL/Redis/外部系统
```

前端：

```text
page -> feature hooks/services -> api-client -> OpenAPI schema
page -> @wms/ui business -> @wms/ui ui
app shell -> session/router/providers -> pages
```

跨端唯一契约：

```text
backend utoipa -> shared/openapi -> @wms/api-client -> Web/PDA
```

### 2.2 红线

- `domain` 不依赖 `api`、`infra`、数据库、HTTP、Redis、环境变量。
- `api handler` 不写业务规则，不直接散落复杂 SQL。
- `repository` 不决定业务流程，只负责数据读写和事务一致性。
- 前端页面禁止裸 `fetch` / 手写 API 类型；生产 API 只走 `@wms/api-client`。
- `packages/ui` 禁止调用 API、读取 token、读取业务 store。
- `prototypes/src/pages/*` 禁止直接复制进 `apps/web-admin` 或 `apps/pda-mobile`。
- 密码、token、API key 不进入日志、审计 diff、前端可见错误详情。

---

## 3. 后端分层

### 3.1 层级职责

| 层 | 典型位置 | 职责 | 不可做 |
|---|---|---|---|
| Runtime / Composition Root | `backend/crates/api/src/bin/*.rs` | 读取配置、连接 DB/Redis、组装 router/state、启动服务 | 业务规则、SQL 查询、权限判断细节 |
| API Handler | `backend/crates/api/src/*_handlers.rs` | 提取请求、调用 app/service、转换响应、挂 OpenAPI 契约 | 跨表业务规则、事务编排、复杂 SQL |
| App Service | `backend/crates/api/src/*_service.rs` 或未来 `backend/crates/app/*` | 编排用例、事务边界、权限后的业务流程、调用 repository | HTTP 细节、Axum extractor、SQL 字符串散落 |
| Domain | `backend/crates/domain` 或独立 context domain 模块 | 实体、值对象、状态机、纯业务规则、DTO schema | IO、时间源直读、环境变量、数据库类型 |
| Repository Trait | `*_repository.rs` trait 区域或未来 app/infra 边界 | 定义持久化能力接口 | HTTP 响应、前端 DTO |
| Repository Impl | `*_repository.rs` impl 区域 | SQLx 查询、事务、幂等表、审计落库协作 | 业务取舍、权限策略发明 |
| Migration | `backend/migrations/*.sql` | 表结构、索引、约束、DB 层不可变约束 | 种子账号明文、业务默认数据乱插 |

### 3.2 handler 规则

handler 只能做 5 件事：

1. 用 Axum extractor 取 `State` / `Path` / `Query` / `Json` / `HeaderMap` / `AuthContext`。
2. 做轻量格式校验，例如缺 `Idempotency-Key`。
3. 调用 app service 或明确的 repository façade。
4. 把领域/app 错误转换成 `ErrorResponse`。
5. 返回 OpenAPI 已声明的 DTO。

超过以下任一条件，必须把逻辑从 handler 拆出去：

| 条件 | 拆到哪里 |
|---|---|
| handler 需要连续调用 2 个以上 repository | app service |
| handler 内 SQL 超过 1 条查询或出现事务 | repository |
| handler 内出现状态迁移、库存数量规则、审批规则 | domain/app service |
| 同一查询被 2 个 handler 复用 | repository |
| 同一业务流程被 Web/PDA/API 重用 | app service |

### 3.3 app service 规则

app service 是用例层，负责“这个用户动作如何完成”：

- 权限已由 `AuthContext` 提供，但 service 必须继续使用 `ctx.owner_id` 做货主隔离。
- 写操作在 service 层定义事务边界。
- 写操作必须安排 H2 审计、L11 幂等、必要的领域事件。
- service 只依赖 repository trait，不依赖 Axum 类型。

### 3.4 domain 规则

domain 层只表达业务事实和不可变规则：

- 状态机迁移、数量闭合、批号/效期判断放 domain。
- 领域错误用明确 enum，不用 HTTP 状态码命名。
- `Utc::now()`、`env::var()`、`PgPool`、Redis client 不进入 domain。
- DTO 可以在 `wms-domain` 中作为 OpenAPI schema，但不能把 handler 行为塞进 DTO。

### 3.5 repository 规则

repository 负责“怎么存取”，不负责“该不该做”：

- SQLx 查询集中在 repository。
- 查询必须显式带 `owner_id`，除非是全局字典或 migration 元数据。
- 写操作涉及审计时，业务数据写入和审计写入应在同一事务或有明确补偿策略。
- 幂等查询和写入应复用既有 `idempotency_request` 模式。
- 密码哈希、API key 哈希等安全校验属于 repository/service 协作：repository 取 hash，service 调校验函数。

### 3.6 当前仓库过渡规则

当前后端还没有独立 `app` / `infra` crate，允许在 `wms-api` crate 内按模块模拟分层：

```text
auth_handlers.rs      # handler + 小型 service façade
auth_repository.rs    # auth SQL 查询，达到拆分条件后新增
auth_service.rs       # 登录、锁定、权限变更等用例编排，达到拆分条件后新增
auth.rs               # JWT/AuthContext runtime contract
```

过渡期允许“单个很小的只读查询”留在 handler 模块内；一旦模块继续扩展到用户管理、角色管理、踢下线、refresh/logout，必须拆出 service/repository。

### 3.7 H1 auth 示例边界

| 行为 | 放置 |
|---|---|
| JWT claim、AuthContext extractor、Redis 撤销检查 | `auth.rs` |
| `/api/v1/auth/login` HTTP handler | `auth_handlers.rs` |
| 查用户、查角色、查权限 | `auth_repository.rs`，过渡期可在 `auth_handlers.rs` 私有函数 |
| 密码 bcrypt 校验、失败计数、锁定策略 | `auth_service.rs`，过渡期可在 `auth_handlers.rs` 私有函数 |
| 用户/角色/权限表 | `backend/migrations/*_h1_auth_tables.sql` |
| 前端 token 保存与注入 | `apps/web-admin/src/lib/auth-session.ts` + `src/lib/api.ts` |
| 登录页面 | `apps/web-admin/src/pages/auth/LoginPage.tsx` |

---

## 4. 前端分层

### 4.1 层级职责

| 层 | 典型位置 | 职责 | 不可做 |
|---|---|---|---|
| App Shell | `apps/web-admin/src/App.tsx`、`main.tsx` | Provider、session bootstrap、路由/页面切换、全局错误边界 | 业务表单细节、API 裸调用 |
| Page | `apps/web-admin/src/pages/<context>/*` | 页面编排、表单状态、页面级布局、调用 feature hooks | 复用组件内部实现、手写 API 类型 |
| Feature | `apps/web-admin/src/features/<context>/*` | TanStack Query hooks、mutation、页面专用转换、权限门控 helper | UI 原子组件实现、OpenAPI 类型复制 |
| App Lib | `apps/web-admin/src/lib/*` | API client 实例、session storage、通用 env、轻量工具 | 业务流程、页面状态 |
| API Client | `packages/api-client` | OpenAPI 生成类型、统一 auth header 注入 | 业务判断、token 存储策略 |
| UI Primitive | `packages/ui/src/ui/*` | shadcn 原子组件、主题 token | WMS 业务状态、API 调用 |
| UI Business | `packages/ui/src/business/*` | WMS 业务复合组件，可复用展示/交互 | 页面路由、真实 API 调用、session |

### 4.2 Web 管理端推荐目录

```text
apps/web-admin/src/
├── main.tsx                 # React root + providers
├── App.tsx                  # app shell / session bootstrap / route outlet
├── lib/
│   ├── api.ts               # @wms/api-client 实例和 token provider
│   ├── auth-session.ts      # token 读写、清除、过期处理
│   └── env.ts               # Vite env 类型和读取
├── features/
│   └── auth/
│       ├── auth.queries.ts  # login / me query hooks
│       └── auth.types.ts    # 页面内部派生类型，不复制 OpenAPI schema
└── pages/
    └── auth/
        └── LoginPage.tsx    # Layer 3 页面
```

小型 Wave 1 壳工程可以暂时不引入 React Router；一旦出现 2 个以上正式页面，必须引入路由层，不能继续靠 `if/else` 堆页面。

### 4.3 页面规则

页面层负责组合，不负责复用组件实现：

- 页面可以持有表单状态、筛选条件、当前 tab、局部弹窗状态。
- 页面通过 feature hook 调 API，不直接 `api.GET/POST`，除非是一个尚未形成 feature 的单接口启动页。
- 页面 `.tsx` 达到 600 行警告并建议拆分；达到 800 行阻断。
- 页面文本必须是业务 UI 文案，不写“这里演示了某功能”的说明性文字。
- 登录页、审计页、库存页等正式生产页必须关联用户故事。

### 4.4 feature 规则

feature 层负责“前端用例”：

- 查询：`useCurrentUserQuery`、`useInventoryBatchesQuery`。
- 写入：`useLoginMutation`、`useReceiveOrderMutation`。
- 转换：把 OpenAPI DTO 转成页面 view model，但不改变业务语义。
- 权限：提供 `canReadAudit(user)` 这类只读 helper；最终权限仍以后端为准。

禁止在 feature 层：

- 复制 `packages/api-client/src/schema.ts` 类型。
- 写 fetch/axios。
- 存储密码、token 以外的敏感凭据。
- 发明后端没有的字段。

### 4.5 session 与 token

- token 存取集中在 `auth-session.ts`。
- `api.ts` 只通过 token provider 注入 `Authorization: Bearer <token>`。
- 页面不拼 Authorization header。
- logout 必须清 token，并使当前用户 query 失效。
- 密码只存在于登录表单受控 state，提交后不写日志、不落 localStorage。

### 4.6 UI 组件边界

`packages/ui` 的两层含义不变：

```text
business -> ui
ui 不依赖 business
business 不依赖 page / feature / api-client
```

跨 3 个以上页面复用的 WMS 交互，应提取为 Layer 2 business 组件，并按 `docs/frontend-coding-standards.md` 注册和加文档头。

---

## 5. 原型到生产的分层迁移

原型转生产不是复制文件，而是重建分层：

| 原型内容 | 生产落点 |
|---|---|
| mock 字段 | OpenAPI schema / domain DTO |
| mock 数据列表 | 后端 repository + API client query |
| 页面交互流程 | `apps/web-admin/src/pages/*` |
| 可复用 UI 模式 | `packages/ui/src/business/*` |
| 权限显示控制 | feature helper + 后端 AuthContext |
| 失败/异常状态 | 后端 ErrorResponse + 前端错误态 |

迁移前必须完成：

1. 用户故事覆盖。
2. OpenAPI 契约冻结或已有草案。
3. 权限码和审计要求明确。
4. 原型 visual baseline 已 review。
5. `docs/prototypes/prototype-to-production.md` checklist。

---

## 6. 跨端契约边界

### 6.1 OpenAPI

- 后端新增/修改 handler，必须同步 utoipa 注解。
- `shared/openapi` 是前端类型来源。
- 前端不能手写后端响应类型。
- 字段命名以 OpenAPI/domain schema 为准，不在前端做别名漂移。

### 6.2 ErrorResponse

后端统一返回 `ErrorResponse`。前端只按 `code` 做交互分支，不解析中文 message。

| 场景 | 前端处理 |
|---|---|
| `AUTH-001/002/003/009` | 清 session，回登录页 |
| `AUTH-005` | 显示无权限状态，不隐藏后端错误 |
| `AUTH-008` | 停留登录页，清密码输入 |
| `H1_LOGIN_LOCKED` | 停留登录页，展示锁定提示 |

### 6.3 审计与幂等

- 后端写操作必须安排审计；前端不得伪造审计字段。
- 前端写操作必须生成并传 `Idempotency-Key`，除非该 API 明确声明不幂等。
- 幂等 key 的生成放 feature hook，不放 UI 组件。

---

## 7. 放置决策表

| 你正在写的东西 | 应放位置 |
|---|---|
| 新 HTTP endpoint | `*_handlers.rs` + OpenAPI 注解 |
| 一个业务用例跨多表写入 | `*_service.rs` |
| SQL 查询 / 事务 / `PgPool` | `*_repository.rs` |
| 状态迁移是否合法 | domain |
| 账号登录 token 存储 | `apps/web-admin/src/lib/auth-session.ts` |
| 调登录 API 的 mutation | `apps/web-admin/src/features/auth/auth.queries.ts` |
| 登录页面表单和布局 | `apps/web-admin/src/pages/auth/LoginPage.tsx` |
| 可复用状态徽标 | `packages/ui/src/business/StatusBadge` |
| Button/Input/Label | `packages/ui/src/ui` |
| mock 原型页 | `prototypes/src/pages/*` |

---

## 8. Review 清单

每个前后端功能 PR 至少检查：

- [ ] 后端 handler 没有复杂业务规则或散落 SQL。
- [ ] 所有业务查询/写入使用 `AuthContext.owner_id` 隔离。
- [ ] 写操作有审计、错误路径、权限测试；需要幂等的有 L11。
- [ ] OpenAPI schema 与前端 api-client 类型同步。
- [ ] 前端没有裸 `fetch` / `axios` / 手写 API 类型。
- [ ] token/session 只在 `lib/auth-session.ts` 和 `lib/api.ts` 周边处理。
- [ ] `packages/ui` 没有依赖 app/page/feature/api-client。
- [ ] 原型代码没有直接复制进生产 app。
- [ ] 页面达到 600 行已拆分或有治理豁免说明。

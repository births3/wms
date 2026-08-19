# 用户故事：H3 跨端契约（contract / OpenAPI）

> 模块：H3 横向能力 / Wave 1 W1.C
> 性质：后端 → 前端 + PDA 的契约同步基础
> 依赖：无（与 H1/H2 并行）
> 关联 ADR：0001（utoipa 选型）、0002（包结构）

---

## 背景

WMS 后端 Rust + 前端 Vite/React + PDA React Native，需要类型一致。本模块通过 utoipa 在后端生成 OpenAPI Spec，前端用 openapi-typescript 生成 TS 类型，PDA 复用前端 packages/api-client。

---

## 跨故事约束

1. **单一来源**：所有 API 定义在后端 utoipa derive；不允许前端独立维护类型
2. **CI 验证**：前后端 schema 不一致 → CI 失败
3. **文档**：自动生成 Swagger UI（开发环境）+ ReDoc（生产）
4. **GSP 合规**：API 调用全程审计（外部系统通过 H1 API Key 鉴权）
5. **测试覆盖**：所有 API 必含 L2（契约测试）
6. 测试要求：写操作必须含 L4（错误路径）+ L5（数据一致）+ L8（权限）+ L11（幂等）；读操作必须含 L4 + L8


---
## US-H3-001：后端 OpenAPI 生成

**作为** 后端开发者
**我要** 用 utoipa 注解 API 定义，自动生成 OpenAPI 3.0 Spec
**以便** 前端类型自动同步、外部系统对接有标准文档

### 验收标准

1. **utoipa 注解**：
   - `#[utoipa::path(...)]` 路由层声明
   - `#[derive(ToSchema)]` 数据模型声明
   - `#[openapi(paths(...), components(schemas(...)))]` 顶层聚合
2. **Spec 输出**：`shared/openapi/openapi.yaml`（仓库版本管理）
3. **CI 集成**：构建时自动生成；与上次比较有变化 → 提示"需更新前端类型"
4. **版本策略**：API 路径预留 `/v1/`；首个正式版本发布前直接同步当前契约，不保留旧版本；发布后 breaking change 才按 ADR-0016 建立 `/v2/` 和退役窗口
5. **错误码标准**：所有 API 错误返回标准格式 `{ code, message, details? }`
6. **Spec 覆盖率**：所有 public API 必须有 Spec 定义；CI 检查覆盖率
7. **示例**：每个 API 至少 1 个 request + response 示例
8. **Tag 分组**：按业务模块（M1/M2/.../H1）+ 横向能力 tag 分组
9. **审计**：Spec 文件变更进入 Git commit；不写运行时审计

---

## US-H3-002：前端 TS 类型生成

**作为** 前端开发者
**我要** 从 OpenAPI Spec 自动生成 TS 类型
**以便** 前端写代码时编辑器即时提示 + 类型错误编译期发现

### 验收标准

1. **生成工具**：openapi-typescript（CLI）
2. **生成产物**：
   - `packages/api-client/types.ts`（接口类型）
   - `packages/api-client/client.ts`（fetch 封装，复用类型）
3. **CI 集成**：openapi.yaml 变更 → 自动重新生成 + commit；前端 PR 与后端 PR 同步合并
4. **类型完整性**：
   - request body / response body 类型完整
   - 错误响应类型完整
   - 枚举类型对齐后端
5. **PDA 复用**：apps/pda-mobile 直接 import packages/api-client；不重复定义
6. **测试覆盖**（L2 契约）：前端 mock 服务必须满足 OpenAPI Spec
7. **运行时校验**（可选）：开发环境用 zod 在 fetch 边界做运行时类型校验

---

## US-H3-003：API 限流与熔断

**作为** 系统架构师 / 运维
**我要** 所有 API 有限流和熔断保护
**以便** 防止单调用方拖垮系统、单服务故障扩散

### 验收标准

1. **全局限流**（默认值，可配置）：
   - 全局 1000 QPS
   - 单用户 100 QPS
   - 单 API Key 100 QPS（外部系统）
2. **限流算法**：令牌桶（允许短时突发容量）
3. **超限响应**：429 Too Many Requests + Retry-After header
4. **熔断**：依赖服务（如外部冷链系统）连续失败 → 熔断 30 秒（默认值，可配置）→ 半开恢复
5. **降级策略**：熔断期间返回降级响应（如缓存数据 + 标记"数据可能滞后"）
6. **监控**：限流命中数、熔断触发次数、降级响应数 → Prometheus
7. **审计**：限流 / 熔断事件写审计（actor = 调用方 / API Key 标识）
8. **测试覆盖**（L4 错误路径 + L7 性能）：
   - 突发流量 → 限流生效
   - 依赖服务故障 → 熔断生效 + 降级响应
   - 恢复后 → 半开 → 关闭

---

## US-H3-004：API 文档可访问性

**作为** 外部系统对接方 / 内部开发者
**我要** 在线浏览 API 文档（Swagger UI / ReDoc）
**以便** 不查代码就能对接

### 验收标准

1. **环境差异**：
   - 开发环境：Swagger UI（`/api-docs`）+ "Try it out" 功能
   - 测试/预发环境：Swagger UI 仅限内网
   - 生产环境：ReDoc（只读，无 try it）+ 限内网
2. **认证流程文档**：单独"如何对接"页面，含 H1 API Key 申请流程
3. **示例代码**：每个 API 至少 curl 示例
4. **变更日志**：每次 Spec 变更生成 changelog（mkdocs 集成）
5. **mkdocs 集成**：API 文档作为 mkdocs 站点的子模块入口
6. **多语言**（暂不强制）：默认中文 + 字段英文 ID
7. **审计追踪**：Spec 变更通过 Git commit 追踪；文档访问日志由 Web 服务器记录（不写业务审计表）

### 测试维度覆盖

#### US-H3-001 后端 OpenAPI 生成

| 维度 | 场景 |
|------|------|
| L4 错误路径 | utoipa 注解缺失 → CI 报错"API 未覆盖 Spec"; Spec 生成失败（语法错误）→ CI 阻断 |
| L5 数据一致 | 生成的 openapi.yaml 与实际 API 行为一致（L2 契约测试验证）; Spec 版本号与 Git tag 一致 |
| L8 权限 | Spec 中每个 endpoint 声明 security 要求; 未声明的 → CI 报错 |
| L11 幂等 | Spec 生成是确定性的（相同代码 → 相同 yaml，无随机字段顺序） |

#### US-H3-002 前端 TS 类型生成

| 维度 | 场景 |
|------|------|
| L4 错误路径 | openapi.yaml 格式错误 → openapi-typescript 报错 + CI 阻断; 类型冲突（同名不同结构）→ 报错 |
| L5 数据一致 | 生成的 types.ts 与 openapi.yaml 100% 对应; 枚举值与后端 Rust enum 一致 |
| L8 权限 | 不涉及运行时权限（编译期工具） |
| L11 幂等 | 类型生成确定性（相同 yaml → 相同 ts） |

#### US-H3-003 API 限流与熔断

| 维度 | 场景 |
|------|------|
| L4 错误路径 | 超限 → 429 + Retry-After; 熔断触发 → 降级响应 + 标记"数据可能滞后"; Redis 不可用 → 降级到本地计数器（精度下降但不拒绝所有请求） |
| L5 数据一致 | 限流计数器与实际请求数一致; 熔断状态与实际失败率一致 |
| L8 权限 | 限流按用户/API Key 维度隔离; 内部服务调用豁免用户限流 |
| L11 幂等 | 限流/熔断是状态机，不涉及业务幂等; 但降级响应必须标记（前端不缓存降级数据） |

#### US-H3-004 API 文档可访问性

| 维度 | 场景 |
|------|------|
| L4 错误路径 | Spec 文件不存在 → Swagger UI 显示错误页; 生产环境访问 /api-docs → 404（仅 ReDoc 可用） |
| L5 数据一致 | 文档内容与实际 API 行为一致（CI 自动验证） |
| L8 权限 | 生产 ReDoc 限内网访问; 开发环境无限制 |
| L11 幂等 | 文档访问天然幂等（只读） |

### PDA 弹性

> 适用故事：US-H3-002（PDA 端复用 api-client 类型）

#### 离线策略

| 场景 | 行为 |
|------|------|
| PDA 离线时调用 API | api-client 层拦截 → 返回本地缓存数据（如有）或提示"网络不可用" |
| PDA 与 API 类型不一致 | 首个正式版本前禁止混跑；同步更新 OpenAPI、api-client、PDA 调用方和测试后再交付 |

#### 扫码交互

不适用（H3 为基础设施模块，不涉及扫码）。

#### 异常恢复

| 场景 | 行为 |
|------|------|
| PDA 构建使用过期类型 | 构建和契约检查失败；重新生成 api-client 并修正调用方 |

---

## 业务边界声明

> H3 OpenAPI 模块的边界：
>
> 1. **单一来源**：utoipa 是唯一的 API 定义入口；前端不独立维护
> 2. **不主管鉴权**：鉴权由 H1 实现，本模块仅在 Spec 中声明"需要鉴权"
> 3. **不主管业务**：本模块只做契约同步，不做业务逻辑
> 4. **限流默认值开箱即用**：不配置也能跑（默认 1000 QPS / 100 QPS）
> 5. **生产环境保护**：Swagger UI 不暴露生产 API；ReDoc 只读

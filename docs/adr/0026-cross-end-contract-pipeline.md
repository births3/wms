# ADR-0026：跨端契约管线（utoipa → OpenAPI → openapi-typescript → openapi-fetch）

- 状态：Accepted
- 决策日期：2026-05-23
- 修订日期：2026-05-24（review 通过，无内容修订）
- 决策人：项目主人
- 来源：SPIKE-003 验证结果（accept）
- 关联：ADR-0001（技术栈）/ ADR-0002（monorepo 结构）/ ADR-0028（packages/ui 抽离）

---

## 1. 背景

ADR-0001 §接口契约 早已确定方向："utoipa（Rust）→ openapi.json → openapi-typescript（前端）"。但 ADR-0001 只定向，不定型——具体工具版本、目录结构、CI 检测、错误处理都没说清楚。

SPIKE-003 验证了完整链路（5 个假设 H1-H5 全部 accept），本 ADR 把验证结果固化为 Wave 1 W1.C 实施时的硬约束。

---

## 2. 决策

### 2.1 工具栈与版本（锁定）

| 角色 | 工具 | 版本（Wave 1 W1.C 启动时）|
|------|------|---------------------------|
| 后端注解 | `utoipa` | `^4.2`（5.x 升级评估留 Wave 2） |
| OpenAPI 导出 | `cargo run --bin openapi-export` | 自建，纯静态 println! |
| 前端类型生成 | `openapi-typescript` | `^7.13` |
| 前端 fetcher | `openapi-fetch` | `^0.13`（轻量，类型推导优秀） |
| OpenAPI 规范 | OpenAPI 3.0.3（utoipa 4.2 默认） | 3.1 留 Wave 2 升级评估 |

### 2.2 目录结构（Wave 1 W1.C 落地形态）

```
wms/
├── backend/
│   ├── Cargo.toml                     # workspace
│   └── crates/
│       ├── domain/                    # ToSchema 业务类型
│       ├── api/                       # utoipa::path + ApiDoc
│       ├── openapi-export/            # bin: 序列化 ApiDoc 到 stdout
│       └── ...                        # 其他 crate（infra / app / ...）
│
├── shared/openapi/
│   └── openapi.json                   # 入 git；前端消费源；CI 校验同步
│
├── packages/api-client/               # @wms/api-client（前端共享包，Wave 1 W1.C 新建）
│   ├── package.json                   # name=@wms/api-client, type=module
│   ├── src/
│   │   ├── schema.ts                  # openapi-typescript 自动生成（不手改）
│   │   ├── client.ts                  # createClient<paths>() 封装
│   │   └── index.ts                   # barrel export
│   └── README.md
│
└── apps/web-admin/                    # 业务页面 import @wms/api-client
```

### 2.3 数据类型映射（编码契约）

实施时业务类型必须按下表设计，否则前端类型不可预期：

| Rust 类型 | OpenAPI 表示 | TypeScript 类型 | 备注 |
|-----------|-------------|----------------|------|
| `Uuid` | `string + format: uuid` | `string` | 前端不做 uuid 解析；后端校验 |
| `chrono::DateTime<Utc>` | `string + format: date-time` | `string`（RFC3339） | 前端用 dayjs / date-fns 解析 |
| `chrono::NaiveDate` | `string + format: date` | `string`（YYYY-MM-DD） | 同上 |
| `Option<T>` | `T \| null`（`required` 不含该字段） | `T \| undefined` | undefined 而非 null（openapi-typescript 默认） |
| `Vec<T>` | `array<T>` | `T[]` | — |
| `enum 简单`（无数据） | `enum: ["A", "B"]` | `"A" \| "B"` | union literal |
| `enum 带数据 + serde tag/content` | `oneOf` 含 type 区分 | discriminated union | 前端用 `switch (x.type)` narrowing |
| `serde_json::Value` | `type: object` + `additionalProperties: true` | `unknown` | 前端必须运行时校验（zod 等） |
| `f64` | `number` | `number` | — |
| `u32 / i32` | `integer + format: int32` | `number` | TS 没有 i32 类型 |
| `u64` | `integer + format: int64` | `number` | JS Number 精度 < 2^53；超出走 string |

### 2.4 utoipa 编码约束（强制）

1. **每个 Path/Query 参数必须带 description**（否则 macro 编译报错信息隐晦）
2. **每个 schema struct 必须 `#[derive(ToSchema, Serialize, Deserialize)]`**
3. **tagged enum 默认用 `#[serde(tag = "type", content = "data")]`**（保持前端 discriminated union 一致性）
4. **`serde_json::Value` 必须 `#[schema(value_type = Object)]`**
5. **泛型容器**：起步用具体化版本（如 `PaginatedItems`），不引入 `#[aliases]`；Wave 2+ 评估
6. **响应体必须用 `body = T` 显式声明**，不允许默认 `body = ()`
7. **错误统一用 `ErrorResponse`**（含 `code: String`, `message: String`）
8. **每个 `#[utoipa::path]` 必须有 `tag`**（用于前端 group 展示与 Swagger UI 分组）

### 2.5 CI / 开发流

| 触发时机 | 命令 | 治理 |
|---------|------|------|
| 后端开发改 schema | `just openapi-sync`（实际：`cd backend && cargo run --bin openapi-export > ../shared/openapi/openapi.json`） | 手动 |
| 前端开发跑类型 | `pnpm gen:schema`（实际：`openapi-typescript ../shared/openapi/openapi.json -o packages/api-client/src/schema.ts`） | 手动 |
| 提交前 | T2 `check_openapi_in_sync.py`：跑 cargo + diff committed | 自动 |
| CI | T2 同上 + 前端 `tsc --noEmit` | 自动 |

`shared/openapi/openapi.json` **入 git**——前端在没启动后端时也能生成类型（避免 CI / 新机器部署阻塞）。

### 2.6 前端 fetcher 模式

`packages/api-client/src/client.ts`：

```ts
import createClient from "openapi-fetch";
import type { paths } from "./schema";

export type ApiClient = ReturnType<typeof createClient<paths>>;

export function createApiClient(opts: {
  baseUrl: string;
  authToken?: () => string | null;  // 与 SPIKE-001 鉴权契约对接
}): ApiClient {
  return createClient<paths>({
    baseUrl: opts.baseUrl,
    fetch: async (input, init) => {
      const headers = new Headers(init?.headers);
      const token = opts.authToken?.();
      if (token) headers.set("Authorization", `Bearer ${token}`);
      return fetch(input, { ...init, headers });
    },
  });
}
```

`apps/web-admin/src/lib/api.ts` 单例化：

```ts
import { createApiClient } from "@wms/api-client";
import { useAuthStore } from "@/stores/auth";

export const api = createApiClient({
  baseUrl: import.meta.env.VITE_API_BASE_URL,
  authToken: () => useAuthStore.getState().accessToken,
});
```

### 2.7 治理脚本

`scripts/governance/check_openapi_in_sync.py`（Wave 0.5 SPIKE-003 已落盘草案）：
- Wave 1 W1.C 启动时，BACKEND_DIR 从 spike-003 改为 `backend/`
- 进 T2 治理（`gate-rules.toml` match `backend/crates/api/**` 或 `shared/openapi/**`）
- 当前 `tier="T1"` 不合适（需要 cargo run），落盘时定 `tier="T2"`

`justfile` 加：
```makefile
openapi-sync:
    cd backend && cargo run --quiet --bin openapi-export > ../shared/openapi/openapi.json
    pnpm --filter @wms/api-client gen:schema

openapi-check:
    python3 scripts/governance/check_openapi_in_sync.py
```

---

## 3. 候选方案

### A. 本决策方案（utoipa + openapi-typescript + openapi-fetch）— 接受

理由：SPIKE-003 全 5 假设 accept；类型一致性强；社区活跃；体积轻。

### B. 手写 OpenAPI YAML + paperclip 编译期 macro 校验 — 否决

理由：违反单一真相源（手写 YAML 与代码必然漂移）；ADR-0001 已否决；spike-003 无理由翻案。

### C. orval 替代 openapi-fetch — 推迟

理由：orval 生成 hooks 代码冗余，对当前简单需求过重；openapi-fetch 类型推导已够用。如未来需要 React Query 自动包装可重新评估，写新 ADR。

### D. utoipa 5.x 升级 — 推迟到 Wave 2

理由：utoipa 5.x 改了部分 macro 语义；Wave 1 起步用 4.2 已验证；升级评估留 Wave 2 W2.A 实施期。

### E. 不入 git 的 openapi.json（CI 时生成） — 否决

理由：前端在没启动后端时无法生成类型；新机器 / CI runner 必须先装 cargo + Rust 工具链才能开发前端，违反 monorepo 易上手原则；本决策选"入 git + CI 校验同步"。

---

## 4. 实施 checklist（Wave 1 W1.C 启动时）

- [ ] `backend/crates/{domain,api,openapi-export}` 按 spike-003 模式建立
- [ ] `shared/openapi/openapi.json` 首次生成入 git
- [ ] `packages/api-client/` 新建（`@wms/api-client`，`type=module`，`exports`）
- [ ] `packages/api-client/src/schema.ts` 自动生成（脚本 `gen:schema`）
- [ ] `packages/api-client/src/client.ts` 含 `createApiClient` 封装（与 SPIKE-001 鉴权对接）
- [ ] `apps/web-admin/` 安装 `@wms/api-client` workspace 依赖
- [ ] `scripts/governance/check_openapi_in_sync.py` 改 BACKEND_DIR + 进 T2
- [ ] `justfile` 加 `just openapi-sync` / `just openapi-check`
- [ ] `governance/gate-rules.toml` 加规则：改 `backend/crates/api/**` 或 `shared/openapi/**` 触发 `check_openapi_in_sync`
- [ ] 文档：`packages/api-client/README.md` 含集成 + 错误处理范例

---

## 5. 后果

### 正面

- **后端字段错改 → 前端 tsc 立刻报错**：跨端契约真正闭环；不再有"后端改了前端不知道"的 bug
- **类型一处定义 / 前后端共享**：utoipa 注解既驱动 OpenAPI 又驱动 Rust handler 签名
- **fetcher 类型 100% 推导**：path / query / body / response 全程类型安全；新人不用记接口
- **Spike 代码模式可复用**：spike-003 4 个 crate（domain/api/openapi-export + 隐式）+ frontend 结构直接迁到 backend / packages/api-client

### 负面

- **utoipa macro 错误信息差**：`expected ','` 这种隐晦错误在 Wave 1 W1.C 早期会折腾新人；缓解：编码模板 + check_openapi_in_sync 早期介入
- **utoipa 4.2 泛型不友好**：`Paginated<T>` 需要 alias，目前用具体化版本绕开；Wave 1 W1.C 实施期需积累 5-10 个具体化分页类型才能稳定
- **`shared/openapi/openapi.json` 入 git 增加 commit 噪声**：每次后端改 schema 都会有大 diff；缓解：`git diff --stat` 标记 + commit message 明示

### 风险

- **utoipa 维护风险**：开源单维护者；缓解：保留迁移到手写 OpenAPI YAML 的退路（成本可控，类型映射已固化在本 ADR §2.3）
- **openapi-typescript 7.x 行为变化**：升级 8.x 时类型生成可能微调；缓解：lock 在 7.x，升级评估写新 ADR
- **OpenAPI 3.0 vs 3.1 不一致**：utoipa 4.2 输出 3.0.3，3.1 工具消费可能需要 polyfill；当前不阻塞，留 Wave 2

---

## 6. 关联文档

- [SPIKE-003 验证记录](../spikes/spike-003-utoipa-openapi-ts-pipeline.md)
- Spike PoC 代码：已在 ADR 固化和生产实现后从仓库移除；保留验证记录
- [ADR-0001 技术栈](0001-tech-stack.md)
- [ADR-0028 组件库抽离](0028-component-library-extraction.md)（同期产出，packages/ui 与 packages/api-client 是姊妹包）

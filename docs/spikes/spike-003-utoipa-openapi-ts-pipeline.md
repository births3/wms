# SPIKE-003: utoipa → OpenAPI → openapi-typescript 全链路

- 状态：起草
- 时间盒：1.5 天（12 小时）
- Owner：项目主人
- 起始：— 完成：—
- 关联 Wave 任务：W1.C OpenAPI 契约工具链
- 关联 ADR：ADR-0001（utoipa + openapi-typescript 已选定）；拟产出 ADR-0026 跨端契约管线

---

## 1. 背景与问题

ADR-0001 决定了"后端 utoipa 注解 → openapi.json → 前端 openapi-typescript 消费"。
但**没验证全链路**：到底能否真正做到"后端改字段，前端 tsc 立刻报错"。

未确定项：

1. utoipa 0.5+ 对复杂类型（newtype、JSONB、嵌套 enum、tagged union）的支持完整度
2. 生成的 openapi.json 是 3.0 还是 3.1（影响下游工具）
3. 前端怎么消费：openapi-typescript 只生成类型 vs openapi-fetch 生成 fetcher
4. Cargo workspace 中如何只 build 出 openapi 而不启动全服务（用 binary `openapi-export`？）
5. CI 中如何检测"前端 schema.ts 与后端不同步"（git diff 检测 + 失败提示）
6. Hot-reload 体验：dev 模式下 `cargo watch` 改 handler → openapi.json 自动重生 → vite hmr？

---

## 2. 验证假设

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | utoipa `#[derive(ToSchema)]` 能完整描述 GSP 业务的复杂类型：嵌套 struct / enum 带 discriminator / Option / Vec / chrono::DateTime / serde_json::Value | 写最小 demo 含 5 种典型类型，导出 openapi.json，肉眼 + jsonschema validator 校验 |
| H2 | 通过专用 binary `openapi-export` 在 CI 跑 `cargo run --bin openapi-export > shared/openapi/openapi.json`，无需启服务 | demo crate 含 lib（业务）+ bin（导出），主 server 复用 lib |
| H3 | openapi-typescript 0.6+ 能把 H1 验证的 openapi.json 转成 TS 类型，前端 import 后 tsc 严格模式无报错 | 跑 `pnpm openapi-typescript shared/openapi/openapi.json -o packages/api-client/src/schema.ts` + tsc --noEmit |
| H4 | "后端改字段，前端 tsc 立刻报错" 链路成立：手动改 utoipa schema → 重生 → 前端 import 该字段的代码 tsc 报错 | 实测：删 `Item.batch_no` 字段，前端使用 `item.batch_no` 处必须报错 |
| H5 | 前端 fetcher 用 openapi-fetch（轻量、纯类型推导）而非 orval / openapi-generator（重、生成代码冗余）足够 | 写 3 个典型 handler 调用对比 |
| H6 | dev 模式可 `cargo watch` + `pnpm openapi-typescript --watch`，端到端反馈 < 5 秒 | 实测改 handler 字段到 vite hmr 重渲的总时间 |

---

## 3. 退出条件

| 状态 | 条件 |
|------|------|
| accept | H1-H5 全部确认；H6 不达标也接受（dev 体验降级到手动重生）；产出 ADR-0026 + 全链路 demo |
| reject | H1 不成立（utoipa 对 tagged union 等关键类型支持差）→ 候选改用 paperclip 或手写 OpenAPI YAML，新建 spike-003b |
| defer | H6 hot-reload：可延后到 Wave 1 实际开发体验时再优化 |

---

## 4. 实施路径

### 步骤 1：搭 demo workspace（2 小时）

```
spikes/spike-003-utoipa-openapi-ts-pipeline/
├── Cargo.toml                # workspace
├── crates/
│   ├── domain/               # lib：含 5 种典型类型
│   ├── api/                  # lib：含 utoipa 注解的 handler
│   └── openapi-export/       # bin：写 openapi.json
├── frontend/                 # pnpm 工程
│   ├── package.json
│   └── src/api.ts            # 引用 schema.ts
└── shared/openapi/openapi.json
```

5 种典型类型（覆盖 GSP 业务真实场景）：
1. `Item { id: Uuid, code: String, batch_no: Option<String>, expiry: NaiveDate }`
2. `enum InventoryStatus { Qualified, Isolated { reason: String }, Quarantined }`（tagged union）
3. `Audit { diff: serde_json::Value }`（任意 JSON）
4. `PaginatedItems { data: Vec<Item>, total: u64 }`（嵌套）
5. `ColdChainPoint { t: DateTime<Utc>, v: f64 }`（chrono）

### 步骤 2：utoipa 注解 + 导出 binary（3 小时）

```rust
// crates/api/src/lib.rs
#[utoipa::path(get, path = "/items/{id}", responses(...))]
pub async fn get_item(...) -> ... {}

#[derive(OpenApi)]
#[openapi(paths(get_item, ...), components(schemas(Item, InventoryStatus, ...)))]
pub struct ApiDoc;
```

```rust
// crates/openapi-export/src/main.rs
fn main() {
    println!("{}", api::ApiDoc::openapi().to_pretty_json().unwrap());
}
```

### 步骤 3：前端消费链路（2 小时）

```bash
cd frontend
pnpm add -D openapi-typescript openapi-fetch
pnpm exec openapi-typescript ../shared/openapi/openapi.json -o src/schema.ts
```

```ts
// frontend/src/api.ts
import createClient from "openapi-fetch";
import type { paths } from "./schema";
const client = createClient<paths>({ baseUrl: "/api/v1" });
const { data } = await client.GET("/items/{id}", { params: { path: { id: "..." } } });
data?.batch_no  // ← 改后端 utoipa schema 删除 batch_no 后此行 tsc 报错
```

### 步骤 4：tsc 反向验证（2 小时）

- 在 `domain::Item` 删 `batch_no` 字段
- 重跑 export + openapi-typescript
- 跑 `pnpm tsc --noEmit`，期望失败：`Property 'batch_no' does not exist on type ...`
- 截图 / 命令日志保留作证

### 步骤 5：CI 集成脚本（1.5 小时）

```yaml
# .github/workflows/openapi-sync.yml（spike 内的草案，不上 main CI）
- run: cargo run --bin openapi-export > /tmp/openapi.json
- run: diff /tmp/openapi.json shared/openapi/openapi.json
  # 失败提示：openapi.json 与代码不同步，请重生
```

### 步骤 6：写 ADR-0026（1.5 小时）

`docs/adr/0026-cross-end-contract-pipeline.md` Proposed：
- 链路图（utoipa → openapi.json → schema.ts → openapi-fetch）
- 工具版本锁定（utoipa X.Y / openapi-typescript X.Y）
- shared/openapi/openapi.json 是否进 git（结论：是，+ CI 检测同步）
- dev / CI / 前端三种触发场景
- 反向验证作为 T2 治理脚本（写入 governance/gate-rules.toml）

---

## 5. 风险与后备方案

| 风险 | 概率 | 影响 | 后备方案 |
|------|------|------|---------|
| utoipa 对 tagged union 生成的 OpenAPI 不符合 3.0 spec | 中 | 中 | 退到手写 OpenAPI YAML + paperclip 编译期 macro 校验 |
| openapi-fetch 类型推导在嵌套泛型时失效 | 低 | 低 | 改用 orval（接受重一些的代码生成） |
| serde_json::Value 在 schema 里变成 `{}` 空 object | 中 | 低 | 接受不强类型，前端用 `unknown` + 运行时 zod 二次校验 |
| CI diff 在 OS / Cargo 版本差异下产生噪声 | 低 | 低 | 用 jq normalize 后再比较 |

---

## 6. 产出物清单

- 代码：`spikes/spike-003-utoipa-openapi-ts-pipeline/`
- 文档：本文件 §7
- ADR：`docs/adr/0026-cross-end-contract-pipeline.md`
- 治理：T2 加 `check_openapi_in_sync.py`（diff 后端导出 vs git 内 openapi.json）
- 配置模板：`scripts/governance/check_openapi_in_sync.py` + `justfile` 加 `just openapi-sync` 入口

---

## 7. 决策记录

> spike 完成后填写。

- 日期：—
- 结论：—
- 关键发现：—
- 后续动作：—

/**
 * SPIKE-003 demo — 前端消费后端 utoipa 生成的 OpenAPI 类型
 *
 * 验证假设：
 * - H3: openapi-typescript 把 openapi.json 转 schema.ts，tsc strict 全过
 * - H4: 删后端字段后重生 schema.ts，本文件使用该字段处 tsc 立刻报错
 * - H5: openapi-fetch 类型推导成立（path / query / body / response 全程类型安全）
 */
import createClient from "openapi-fetch";
import type { components, paths } from "./schema";

// 类型重导出方便业务代码使用
type Item = components["schemas"]["Item"];
type Audit = components["schemas"]["Audit"];
type InventoryStatus = components["schemas"]["InventoryStatus"];
type ColdChainPoint = components["schemas"]["ColdChainPoint"];
type PaginatedItems = components["schemas"]["PaginatedItems"];

const client = createClient<paths>({ baseUrl: "http://localhost:8080" });

// === 场景 1：GET /api/v1/items/{id} — 单条查询 ===
async function fetchItem(id: string): Promise<Item | null> {
  const { data, error } = await client.GET("/api/v1/items/{id}", {
    params: { path: { id } },
  });
  if (error) {
    console.error("error code:", error.code, "message:", error.message);
    return null;
  }
  // ↓ 类型推导：data 是 Item
  console.log("item code:", data.code);
  console.log("batch_no:", data.batch_no ?? "(无批号)"); // Option<String> → string | null
  console.log("expiry:", data.expiry); // NaiveDate → string (格式 "YYYY-MM-DD")
  return data;
}

// === 场景 2：GET /api/v1/items — 分页 + tagged union ===
async function listItems(): Promise<PaginatedItems | null> {
  const { data, error } = await client.GET("/api/v1/items", {
    params: { query: { page: 1, page_size: 20 } },
  });
  if (error) return null;

  // 遍历 Item，演示 tagged union 类型 narrowing
  for (const item of data.data) {
    const status = item.status;
    switch (status.type) {
      case "Qualified":
        console.log(`${item.code}: 合格`);
        break;
      case "Isolated":
        // ↓ TS 自动推导：data 字段存在
        console.log(`${item.code}: 隔离 — ${status.data.reason}`);
        break;
      case "Quarantined":
        console.log(`${item.code}: 待检`);
        break;
      case "PendingDestruction":
        console.log(`${item.code}: 待销毁审批 ${status.data.approver_id}`);
        break;
    }
  }
  return data;
}

// === 场景 3：GET /api/v1/audit/events/{id} — JSONB 字段 ===
async function fetchAudit(id: string): Promise<Audit | null> {
  const { data, error } = await client.GET("/api/v1/audit/events/{id}", {
    params: { path: { id } },
  });
  if (error) return null;
  // diff: serde_json::Value → unknown （前端需要运行时校验）
  console.log("actor:", data.actor_name, "action:", data.action);
  console.log("diff (raw):", JSON.stringify(data.diff));
  return data;
}

// === 场景 4：GET /api/v1/cold-chain/points — DateTime + Vec ===
async function fetchColdChain(from: string, to: string): Promise<ColdChainPoint[] | null> {
  const { data, error } = await client.GET("/api/v1/cold-chain/points", {
    params: { query: { from, to } },
  });
  if (error) return null;
  // data: ColdChainPoint[]
  return data.map((p) => ({
    t: p.t,                    // DateTime<Utc> → string (RFC3339)
    v: p.v,                    // f64 → number
    out_of_range: p.out_of_range, // bool → boolean
  }));
}

// === 场景 5：POST /api/v1/items/{id}/isolate — 写操作 + tagged union body ===
async function isolateItem(id: string): Promise<Item | null> {
  const body: InventoryStatus = {
    type: "Isolated",
    data: { reason: "外观破损 3 件，待质量负责人复核" },
  };
  const { data, error } = await client.POST("/api/v1/items/{id}/isolate", {
    params: { path: { id } },
    body,
  });
  if (error) {
    console.error("isolate failed:", error.code, error.message);
    return null;
  }
  return data;
}

// === 主流程（演示，不真发请求） ===
async function main() {
  console.log("SPIKE-003 demo: 仅做类型推导验证，不发真实请求");
  // 这些函数都通过 tsc strict 校验 → 证明 H3 / H5 假设成立
  void fetchItem;
  void listItems;
  void fetchAudit;
  void fetchColdChain;
  void isolateItem;
}

main().catch(console.error);

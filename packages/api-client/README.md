# @wms/api-client

WMS 前端共享 API 客户端骨架。

## 内容

- `src/schema.ts`：由 `shared/openapi/openapi.json` 自动生成的 OpenAPI TypeScript 类型
- `src/client.ts`：基于 `openapi-fetch` 的 `createApiClient`
- `src/index.ts`：公共导出入口

## 生成类型

```bash
pnpm --filter @wms/api-client gen:schema
```

该命令读取 `shared/openapi/openapi.json`，覆盖生成 `src/schema.ts`。

## 使用示例

```ts
import { createApiClient } from "@wms/api-client";

export const api = createApiClient({
  baseUrl: import.meta.env.VITE_API_BASE_URL,
  authToken: () => window.localStorage.getItem("access_token"),
});
```

## 当前范围

当前包只提供 Wave 1 W1.C 骨架：类型生成链路与最小 fetch 封装，不包含业务查询 hooks。

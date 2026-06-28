# apps/AGENTS.override.md

`apps/` 前端和应用模块规则。

- 技术栈：Vite + React + TypeScript + shadcn/ui + Zustand + TanStack Query。
- 生产 API 调用必须走 `@wms/api-client`；禁止裸 `fetch`。
- 禁止 `any`；优先使用生成的 OpenAPI 类型或收窄后的本地视图模型。
- 页面代码调用功能层钩子 / 服务；功能层代码调用 API 客户端；UI 包不得访问 API / 会话。
- 页面 `.tsx` 文件：`>= 600` 行警告，`>= 800` 行门禁。
- UI 控件遵守 [docs/frontend-coding-standards.md](../docs/frontend-coding-standards.md)。
- 新增组件前先复用 `@wms/ui` 和现有组件模式。
- 前端改动必须先查现有页面局部组件、feature hook、api-client、类型和 `lib`/`utils`；能复用就复用。
- 没有现成能力时，新增为标准可复用单元：通用 UI 放 `@wms/ui`，业务复合放 `@wms/ui business` 或业务模块，页面私有组件放页面目录，工具函数放现有 `lib`/`utils`。
- 新增组件或工具函数需说明复用缺口、放置理由和后续复用点；禁止为单一场景复制相似页面、表单或请求逻辑。
- 不得把 `prototypes/src/pages/*` 直接复制到生产应用；必须走 [docs/prototypes/prototype-to-production.md](../docs/prototypes/prototype-to-production.md)。
- PDA 生产应用启动受 ADR-0027 和 `check_pda_production_gate.py` 门禁约束。
- 验证用 `just gov-t1`；触及应用行为时补跑对应构建 / 测试。

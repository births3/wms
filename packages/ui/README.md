# @wms/ui

WMS 前端共享 UI 包。

- **Layer 1 primitives**：9 个 shadcn/ui 组件（Button / Input / Label / Card / Tabs / Select / Checkbox / Table / Dialog）
- **Layer 2 业务复合**：16 个 WMS 业务组件（StatusBadge / ScanInput / DualSignPanel / AuditTimeline / ApprovalFlow / KanbanBoard / PrintPreview / RuleEditor / TempChart / OfflineIndicator / FieldTable / StepFlow / DiffPanel / PageHeader / DataTable / EmptyState）
- **设计 token**：CSS 变量（`src/styles/globals.css`）+ Tailwind preset（`tailwind-preset.cjs`，含 wms-warning / wms-success / wms-cold 业务色）
- **工具**：`cn()`（clsx + tailwind-merge）

## 使用方

| 消费方 | 用途 |
|---|---|
| `prototypes/` | 高保真原型走查（ADR-0021） |
| `apps/web-admin/`（Wave 1+） | PC + PAD 真业务前端 |

## 集成步骤

```jsonc
// 消费方 package.json
{
  "dependencies": { "@wms/ui": "workspace:*" }
}
```

```js
// 消费方 tailwind.config.js
import preset from "@wms/ui/tailwind-preset";
export default {
  presets: [preset],
  content: [
    "./src/**/*.{ts,tsx}",
    "../packages/ui/src/**/*.{ts,tsx}", // 必须扫源码（monorepo 源码导入模式）
    "./index.html",
  ],
};
```

```css
/* 消费方应用入口（main.tsx 之前 import） */
@import "@wms/ui/styles/globals.css";
```

```ts
// 业务页面
import { Button, StatusBadge, cn } from "@wms/ui";
```

## 不在本包内

- 业务页面（在 `apps/web-admin/src/pages/`）
- API 客户端（在 `packages/api-client/`，未来 Wave 1 W1.C 创建）
- 业务类型（在 `packages/domain-types/`，未来）

## 治理

- 组件契约：[ADR-0022 prototype-component-spec](../../docs/adr/0022-prototype-component-spec.md)
- 注册表：[docs/prototypes/component-registry.md](../../docs/prototypes/component-registry.md)
- 治理脚本：`scripts/governance/check_component_*.py`

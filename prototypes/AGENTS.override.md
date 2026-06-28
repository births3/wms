# prototypes/AGENTS.override.md

`prototypes/` 原型模块规则。

- 原型用于表达业务流程并支持走查，不是生产页面。
- 遵守 ADR-0029 和 [docs/prototypes/prototype-to-production.md](../docs/prototypes/prototype-to-production.md)。
- 新增原型页面必须三同步：页面、`Tabs.tsx`、`manifest.toml`，并补基线 PNG。
- UI 结构变化时运行视觉截图 / 基线相关检查。
- 测试环境截图查看入口以 [Matrix E2E 截图门禁](../docs/prototypes/matrix-e2e-screenshot-gate.md) 的“测试环境查看标准”为准，不另起截图服务或嵌入生产前端。
- 模拟字段必须对齐用户故事和 API 草案；不得未经确认发明新规范字段。
- 原型转生产必须走检查清单；禁止直接复制原型页面到 `apps/`。
- 验证用 `just gov-t1`；视觉类改动还要跑相关截图门禁。

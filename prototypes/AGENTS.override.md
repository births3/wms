# prototypes/AGENTS.override.md

`prototypes/` 历史原型资产维护规则。

- ADR-0043 生效后禁止新增原型页面；新需求直接在生产应用中用开发 Mock 走查。
- 既有原型仅用于历史追溯和回归，不是生产页面。
- 维护既有原型时参考已废止的 ADR-0029 和 [docs/prototypes/prototype-to-production.md](../docs/prototypes/prototype-to-production.md)。
- 修复既有原型登记缺口时必须三同步：页面、`Tabs.tsx`、`manifest.toml`，并补基线 PNG。
- UI 结构变化时运行视觉截图 / 基线相关检查。
- 测试环境截图查看入口以 [Matrix E2E 截图门禁](../docs/prototypes/matrix-e2e-screenshot-gate.md) 的“测试环境查看标准”为准，不另起截图服务或嵌入生产前端。
- 模拟字段必须对齐用户故事和 API 草案；不得未经确认发明新规范字段。
- 禁止把历史原型页面直接复制到 `apps/`；生产页面须按当前故事、契约和 TDD 独立实现。
- 验证用 `just gov-t1`；视觉类改动还要跑相关截图门禁。

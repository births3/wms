# 原型转生产迁移清单

> 历史文档：ADR-0043 已取消原型先行流程。本清单仅用于识别和处理既有原型资产，不适用于新增页面。
>
> 用途：定义 `prototypes/` 高保真原型迁移到 `apps/web-admin/` 或 `apps/pda-mobile/` 的准入条件。
> 关联：ADR-0029 前端原型先行工作流。

## 1. 适用范围

本清单适用于所有从原型进入生产应用的页面、流程和组件：

- `prototypes/src/pages/*` → `apps/web-admin/src/pages/*`
- `prototypes/src/pages/*` → `apps/pda-mobile/src/screens/*`
- 原型中沉淀出的新业务组件 → `packages/ui/src/business/*`

`packages/ui` 中已注册组件的视觉调整不走本清单，但仍需跑组件治理和视觉回归。

## 2. 迁移准入

| # | 检查项 | 要求 | 证据 |
|---|------|------|------|
| 1 | 用户故事覆盖 | 页面必须关联至少一个 `US-*`；没有故事先走缺口确认 | 故事文件链接 |
| 2 | 业务走查 | 原型已被业务方走查 approved | `governance/visual-baselines/manifest.toml` reviewed 字段 |
| 3 | 字段来源 | mock 字段全部来自用户故事字段表或 OpenAPI 草案 | 字段表 / schema 链接 |
| 4 | API 契约 | 生产页面依赖的接口已进入 OpenAPI 草案或正式 spec | `shared/openapi/openapi.json` 或 API 文档 |
| 5 | 权限模型 | 页面入口、按钮、数据范围有 H1 权限说明 | H1 权限码 / AuthContext 设计 |
| 6 | 审计要求 | 写操作明确是否接 H2 审计 | H2 审计事件或豁免理由 |
| 7 | 幂等要求 | 写操作明确幂等键或重复提交处理 | 测试用例 / API 契约 |
| 8 | 错误路径 | 用户可见错误、权限错误、网络错误有处理 | L4 测试 |
| 9 | 数据一致 | 乐观更新、分页、缓存失效、并发刷新有策略 | TanStack Query key / invalidation |
| 10 | 视觉基线 | 原型 baseline 当前 T3 视觉回归通过 | `check_visual_regression.py` 输出 |
| 11 | Matrix E2E 截图 | 合并前全量矩阵 E2E 截图通过 | `just matrix-e2e-full` / `check_matrix_e2e_report.py` 输出 |

## 3. 迁移步骤

1. 从原型页提取可复用 UI 到 `packages/ui`；只保留无 API、无路由、无生产状态的展示组件。
2. 在 `apps/web-admin` 建生产页面壳，使用生产路由、布局和权限门控。
3. 用 `@wms/api-client` + TanStack Query 替换 mock 数据。
4. 删除原型专用数据、演示账号、演示说明和假交互。
5. 为写操作补 outside-in 失败测试，再实现最小生产行为。
6. 同步页面关联文档：用户故事、OpenAPI、必要 ADR 或澄清记录。
7. 跑治理脚本和相关测试。

## 4. 禁止项

- 禁止把 `prototypes/src/pages/*` 直接复制到 `apps/web-admin` 后只改 import。
- 禁止在生产页保留 mock 数据作为 fallback。
- 禁止生产页裸 `fetch`；必须使用 API client。
- 禁止生产页自行判断 GSP 业务规则；业务规则归后端领域层或 ADR-0015 允许的多端规则层。
- 禁止无用户故事、无权限说明、无审计判断的写操作进入生产应用。

## 5. 验证命令

```bash
python3 scripts/governance/governance_checks.py --tier T1
python3 scripts/governance/governance_checks.py --tier T3
pnpm --dir prototypes build
pnpm --dir prototypes build-storybook
```

生产应用启动后，迁移 PR 还必须补充对应 `apps/web-admin` 或 `apps/pda-mobile` 的类型检查、单元测试和 E2E 命令。

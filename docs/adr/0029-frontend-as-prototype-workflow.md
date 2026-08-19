# ADR-0029：前端原型先行工作流

- 状态：Superseded by ADR-0043
- 决策日期：2026-05-24
- 决策人：项目主人
- 相关：ADR-0006（TDD + 11 层测试）/ ADR-0015（多端业务规则放置）/ ADR-0021（高保真原型策略）/ ADR-0022（原型组件规范）/ ADR-0028（组件库抽离）

---

## 背景

Wave 0.5 已经交付 `prototypes/` 高保真原型、`packages/ui` 共享组件库和视觉 baseline；截至 2026-05-24，原型覆盖已扩展为 37 个手工高保真页 + 167 个全量矩阵页，共 204 个可走查 tab。用户明确希望"把前端做出来直接当原型用"。

这个方向符合 ADR-0021 的高保真原型策略，但需要补清边界：前端页面可以作为业务走查原型，不能绕过领域模型、API 契约、权限、审计、幂等和数据一致性。否则容易把 mock 交互误认为生产实现，后续在 H1/H2/H3 接入时返工。

因此需要把"前端原型先行"升级为正式工作流：先用真实 React 前端表达流程，再经业务走查确认，之后按领域模型 + OpenAPI 契约 + outside-in TDD 迁移到生产应用。

## 候选方案

### A. 正式采用前端原型先行，并写 ADR

优点：
- 业务方能直接操作真实页面，反馈比静态图更准确。
- Wave 0.5 已有 `@wms/ui`、视觉基线和治理脚本，复用成本低。
- 原型和生产组件共享 design tokens，降低视觉漂移。
- 可以把"能否迁移生产"做成 checklist，避免 mock 页面直接混入生产代码。

缺点：
- 需要维护 `prototypes/` 与 `apps/web-admin/` 的边界。
- 每个新页面要同步 Tabs / manifest / baseline，治理成本更高。

### B. 只作为临时实践，不写 ADR

优点：零文档成本。

缺点：
- 后续开发容易争论"原型能不能直接复制到生产"。
- 新人无法从规范中判断 `prototypes/`、`packages/ui`、`apps/web-admin/` 的职责。
- 不符合重大流程变更需 ADR 的治理原则。

### C. 不采用，回到后端优先或文档优先

优点：领域模型与 API 契约先行，后端边界更稳。

缺点：
- WMS 一线操作流程复杂，PDA / PAD / PC 交互不先走查会增加返工。
- 已完成的原型和组件库资产无法充分复用。

## 决策

采用方案 A：**前端原型先行，确认后迁移生产；原型页面不得直接作为生产实现合入 `apps/web-admin`。**

标准流程调整为：

```text
用户故事
→ 概念审计
→ 模式提炼
→ 领域模型 / API 草案
→ 前端高保真原型
→ 业务走查确认
→ 领域模型 / API 契约冻结
→ outside-in TDD 生产实现
```

职责边界：

| 位置 | 职责 | 可包含 | 不可包含 |
|---|---|---|---|
| `prototypes/` | 高保真原型与业务走查 | mock 数据、流程交互、视觉状态、走查壳 | 生产 API 调用、真实权限判断、持久化写操作 |
| `packages/ui` | 共享 UI 组件和 design tokens | shadcn primitive、业务复合组件、样式变量、Storybook stories | 页面级业务编排、API 调用、领域规则 |
| `apps/web-admin` | 生产 PC/PAD 前端 | 路由、TanStack Query、API client、权限门控、生产页面 | 未确认原型、mock-only 流程、绕过 OpenAPI 的裸 fetch |
| `apps/pda-mobile` | 生产 PDA 前端 | RN 页面、离线队列、扫码设备适配、API client | 直接复用 web-only DOM 组件 |

## 实施约束

1. 新业务页面默认先进入 `prototypes/`，除非该页面已在业务故事和原型矩阵中明确豁免。
2. 原型页必须关联用户故事；没有故事覆盖时，先走缺口确认流程。
3. mock 数据字段必须对齐用户故事字段表或 OpenAPI 草案，禁止为展示随意发明字段。
4. 新增原型页必须同步：
   - `prototypes/src/Tabs.tsx`
   - `governance/visual-baselines/manifest.toml`
   - `governance/visual-baselines/<tab>.png`
   - `docs/prototypes/index.toml`（如属于正式原型清单）
5. 原型转生产必须通过 `docs/prototypes/prototype-to-production.md` checklist。
6. 生产页面必须通过 `@wms/api-client` / TanStack Query 接入 API，禁止裸 `fetch`。
7. 生产写操作必须满足 ADR-0006 必备维度：L4 错误、L5 数据一致、L8 权限、L11 幂等；涉及审计的写操作必须接 H2。
8. 原型页面可以复用 `@wms/ui`，生产页面也必须优先复用 `@wms/ui`；但不得复制 `prototypes/src/pages/*` 后直接改成生产页。
9. Wave 1 允许启动 `apps/web-admin` 壳工程，只接 H1/H2/H3，不提前实现业务模块生产页面。

## 后果

### 正面

- 业务流程先被真实交互验证，再进入生产实现，降低返工。
- `@wms/ui` 成为原型与生产前端的共同基础，组件投资能延续。
- 生产实现仍受领域模型、OpenAPI 和 TDD 约束，不把 mock 页面误当成系统行为。
- 视觉回归、baseline completeness 等治理脚本继续发挥作用。

### 负面

- 流程多一个"原型走查确认 → 迁移生产"关口，短期速度看起来变慢。
- 需要维护 mock 数据和 OpenAPI 草案字段的一致性。
- `apps/web-admin` 与 `prototypes/` 双应用并存，新人需要理解边界。

### 风险

- **风险：原型被误复制成生产页。** 缓解：迁移 checklist + PR review 检查 `prototypes/src/pages/*` 复制痕迹。
- **风险：mock 字段先行导致字段漂移。** 缓解：字段必须引用用户故事字段表 / OpenAPI 草案，T1 治理继续检查故事同步。
- **风险：前端先行诱导后端补接口迁就 UI。** 缓解：走查确认后仍需领域模型与 API 契约冻结，生产实现按 outside-in TDD。
- **风险：视觉治理成本增加。** 缓解：保持 Tabs / manifest / baseline 三同步，T3 视觉回归在 PR 前执行。

## 参考

- [ADR-0006：TDD + 11 层测试维度](0006-tdd-and-test-layers.md)
- [ADR-0015：多端业务规则放置](0015-multi-end-rules.md)
- [ADR-0021：高保真原型策略](0021-high-fidelity-prototype-strategy.md)
- [ADR-0022：原型组件规范](0022-prototype-component-spec.md)
- [ADR-0028：组件库抽离至 packages/ui](0028-component-library-extraction.md)
- [docs/prototypes/prototype-to-production.md](../prototypes/prototype-to-production.md)

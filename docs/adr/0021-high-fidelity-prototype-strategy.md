# ADR-0021：高保真原型策略

- 状态：Superseded by ADR-0043
- 决策日期：2026-05-22
- 决策人：项目主人
- 关联：ADR-0001 / ADR-0006 / ADR-0015 / docs/infra/usability-baseline.md

---

## 背景

169 个用户故事中，按 `docs/prototypes/prototype-matrix-r3.md` 当前矩阵统计，137 个故事行涉及 UI 交互；按端展开后形成 167 个必做原型页（PC 111 / PDA 30 / PAD 20 / H5 6）。PDA 端操作密集（14 步验收、双人签字、离线模式），不做原型直接写 React Native 代码，返工成本极高。

## 决策

**采用 Storybook + 真 shadcn/ui 组件** 作为高保真原型工具（方案 C）。

### 核心约束

1. 原型代码独立于 `apps/`，放 `prototypes/`（仓库根目录）
2. 使用 mock 数据（`prototypes/src/fixtures/`），禁止接真后端 API
3. 禁止写业务逻辑（状态机/验收规则/权限判定）
4. Wave N 启动前，N 涉及的 UI 故事必须有高保真原型 + ≥1 次业务方走查 approved
5. 组件库分三层：Design Tokens → shadcn/ui 主题覆盖 → 12 个业务复合组件
6. PDA 端组件强制 `minTouchTarget ≥ 48pt` / `fontSize ≥ 16pt`

### 否决方案

- **Figma**：二进制不可 git diff，治理只能管导出快照
- **Penpot**：国内企业用得少，协作弱
- **不做原型**：PDA 返工成本 ≈ 6-8 周，远超原型成本 1-2 周/波

### 治理

- 真相源：`docs/prototypes/index.toml`（全量 167 个 story/end 原型页，按 P0-P4 渐进交付）
- 治理脚本：5 个（index_consistency / story_sync / freshness / usability_baseline / review_signoff）
- 节奏铁律 9：涉及一线高频操作的 UI 页面，进入实现 Wave 前必须有高保真原型

### 生命周期

原型代码在对应 Wave 实现完成后标记 `deprecated`。Wave 1 启动写 `apps/web-admin` 时，人工搬运组件到正式代码，不允许软链共享。

---

## 后果

- 增加 Wave 0.5（2 周）专门做组件库 + P0 原型
- 每波尾增加 0.5 周原型交付窗口
- 矩阵估算总原型工时 ≈ 963h（约 24 周/1 人），分摊到 5 个 Wave；当前通过 37 个手工高保真页 + 167 个全量矩阵页形成 204 个可走查 tab

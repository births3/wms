# ADR-0028：组件库抽离至 packages/ui

- 状态：Accepted
- 决策日期：2026-05-23
- 决策人：项目主人
- 相关：ADR-0001（技术栈）/ ADR-0002（monorepo 结构）/ ADR-0021（高保真原型策略）/ ADR-0022（原型组件规范）

---

## 背景

ADR-0021 决定原型用 `prototypes/`，ADR-0022 把 Layer 1（shadcn primitive）和 Layer 2（业务复合组件）放在 `prototypes/src/components/`。这套结构在 Wave 0.5 期间运转良好——38 个原型页面 + 16 个业务组件 + 9 个 shadcn primitive 全部就位、视觉基线签字、Storybook 接入。

进入 Wave 1 准备阶段，问题浮现：

1. **重复画 UI 风险**：用户提出"前端直接代替原型"，希望复用已经画完的视觉/交互工作。如果 `apps/web-admin/`（Wave 1 启动）从空目录写起，等于把 `prototypes/` 的 16 个业务组件 + 9 个 shadcn primitive 重画一遍——浪费已签字的设计成果，且容易造成"原型页面" vs "生产页面"视觉漂移。

2. **ADR-0002 §pnpm Workspace 已规划但未实施 packages/ui**：仓库结构图明确写了 `packages/{api-client, domain-types, ui, utils}`，但落地时只建了 `.gitkeep`。组件库的实际位置成了 `prototypes/src/components/`，与既定结构脱节。

3. **跨端共享需求**：ADR-0001 规划 PC + PAD 共用 `apps/web-admin`，PDA 用 `apps/pda-mobile`（React Native），但 RN 也需要部分组件（如 StatusBadge / ScanInput / OfflineIndicator）。如果组件继续留在 `prototypes/`，三个消费方（prototypes / web-admin / pda-mobile）都要绕道引用，违反单向依赖。

---

## 决策

把原型代码中的"可复用层"全部抽到 `packages/ui/`，构成 `@wms/ui` workspace 包。

### 抽离范围

| 原位置 | 新位置 | 说明 |
|---|---|---|
| `prototypes/src/components/business/` | `packages/ui/src/business/` | 16 个业务复合组件 |
| `prototypes/src/components/ui/` | `packages/ui/src/ui/` | 9 个 shadcn primitive |
| `prototypes/src/lib/utils.ts` | `packages/ui/src/lib/utils.ts` | `cn()` 工具 |
| `prototypes/src/globals.css` | `packages/ui/src/styles/globals.css` | 设计 token CSS 变量 |
| `prototypes/tailwind.config.js` 中的 `theme.extend` | `packages/ui/tailwind-preset.cjs` | Tailwind 设计 token preset |

### 不抽离的部分

| 留在原位 | 原因 |
|---|---|
| `prototypes/src/pages/` | 38 个原型页面是"业务方走查工具"，与生产页面边界不同；不进 `packages/ui` |
| `prototypes/src/{App,Tabs,main}.tsx` | 原型壳（sidebar 导航 / hash 路由）专属于走查场景 |
| `prototypes/.storybook/` | Storybook 配置仍在 prototypes（消费方），stories 跟着组件搬到 packages/ui |
| `apps/web-admin/src/pages/` | 真业务页面，Wave 1 W1.A 后开始写，**不得复制 prototypes/src/pages/ 内容**（业务逻辑必须 TDD 重写） |

### 包元信息

```jsonc
// packages/ui/package.json
{
  "name": "@wms/ui",
  "type": "module",
  "exports": {
    ".":                       "./src/index.ts",         // barrel
    "./business":              "./src/business/index.ts",
    "./ui":                    "./src/ui/index.ts",
    "./lib/utils":             "./src/lib/utils.ts",
    "./styles/globals.css":    "./src/styles/globals.css",
    "./tailwind-preset":       "./tailwind-preset.cjs"
  }
}
```

消费方（prototypes 与未来 apps/*）声明：

```jsonc
{ "dependencies": { "@wms/ui": "workspace:*" } }
```

### Workspace 拓扑

仓库根 `pnpm-workspace.yaml`：

```yaml
packages:
  - "packages/*"
  - "prototypes"
  - "apps/*"
```

prototypes 自有的 `pnpm-workspace.yaml` 删除，统一进 root workspace。

### 源码导入模式（不预编译）

`@wms/ui` **不**做 build step（无 dist/）。消费方直接 import 源码 `.tsx`：

- 优点：vite HMR 跨包生效；开发体验等同单仓库
- 代价：消费方 tailwind 必须扫 `../packages/ui/src/**/*.{ts,tsx}` 才能生成 class（已在 prototypes/tailwind.config.js 配置）
- 未来如发布给外部使用，可加 `tsup` build 步骤，不影响内部消费方

---

## 候选方案

### A. 留在 prototypes/src/components/（不抽）

**优点**：零工作量
**缺点**：
- 违 ADR-0002 §pnpm Workspace 既定结构
- Wave 1 启动时要么重画（浪费）要么 `apps/web-admin` 反向 import `prototypes/`（违单向依赖）
- 跨端共享（PDA RN）做不到

**否决**。

### B. 抽到 packages/ui（本决策）

**优点**：
- 符合 ADR-0002 既定结构
- 三个消费方（prototypes / web-admin / pda-mobile）平等引用
- 单向依赖：apps/* → packages/ui，不反向
- 视觉一致：原型走查与生产前端用同一套组件 + 同一套设计 token

**缺点**：
- 一次性重构成本（约 117 文件改动）
- pnpm workspace 拓扑变更（prototypes 进 root workspace）
- 治理脚本路径需同步（4 个组件治理 + check_file_naming）

**接受**。

### C. 拆更细：packages/ui-primitive + packages/ui-business

**优点**：边界更清晰
**缺点**：当前规模过早；ADR-0001 规划只有 `packages/ui` 单包；过度设计
**否决**：起步单包，未来需要可拆。

---

## 实施

实施已在 commit `e3ce5a0` 完成，关键步骤：

1. 仓库根建 `pnpm-workspace.yaml`（含 `packages/*` + `prototypes` + `apps/*`）
2. 删 `prototypes/pnpm-workspace.yaml`、`prototypes/.npmrc`、`prototypes/pnpm-lock.yaml`
3. 建 `packages/ui/{package.json, tsconfig.json, tailwind-preset.cjs, README.md, src/index.ts}`
4. `git mv` 16 业务组件 + 9 shadcn primitive + utils.ts + globals.css 到 `packages/ui/src/`
5. 修内部 import：`@/lib/utils` → 相对路径；`@/components/ui/X` → `../../ui/X`
6. 修消费方 import：153 处 `@/components/(ui|business)/*` → `@wms/ui`
7. 修配置：`prototypes/{package.json, tailwind.config.js, src/main.tsx, .storybook/{main,preview}.ts}`
8. 修治理脚本：4 个 `check_component_*.py` 改 `BUSINESS_DIR` + `check_file_naming.py` 扩展 shadcn 路径识别 + `governance/gate-rules.toml` 改 match
9. 验证：vite build / storybook build / T1 24/24 全过

---

## 治理影响

| 治理资产 | 变更 |
|---|---|
| `governance/gate-rules.toml` | `match = "packages/ui/src/business/**"`（原 prototypes 路径） |
| `check_component_doc_header.py` 等 4 个 | `BUSINESS_DIR` 改指 `packages/ui/src/business`；PAGES_DIR 不变 |
| `check_file_naming.py` | shadcn primitive kebab-case 路径识别扩展为 `/components/ui/` 或 `packages/ui/src/ui/` |
| ADR-0022 | 加 v0.2 修订记录（路径载体变更，约束不变） |
| `docs/prototypes/component-registry.md` | 同步现状（删除从未存在的 `src/tokens/` `src/theme/` 路径） |

---

## 后果

### 正面

- **Wave 1 启动时 `apps/web-admin/` 直接 `import { ... } from "@wms/ui"` 复用全部 UI**，无需重画 16 个业务组件 + 9 个 shadcn primitive
- **设计 token 唯一真相源**：CSS 变量 + Tailwind preset 在 `packages/ui` 集中维护，原型与生产前端共享，杜绝视觉漂移
- **ADR-0002 §pnpm Workspace 落地**：从"占位 .gitkeep" 升级到"真实 workspace 包"
- **Storybook stories 颗粒度对齐组件库**：16 个 stories 跟着组件迁移，未来发布 Storybook 静态站点即可作为组件文档

### 负面

- **prototypes/ 不再是"自给自足"项目**：理解 prototypes 必须同时理解 `@wms/ui`（双 workspace 概念门槛）
- **monorepo 配置复杂度**：root pnpm-workspace.yaml + 子包 package.json 协同；新人需要先看 ADR-0002 + 本 ADR
- **pnpm 11 严格模式**：每次 install 需要 `pnpm.onlyBuiltDependencies` 配合（已在 `packages/ui/package.json` 上游 root 配置）

### 风险

- **packages/ui 改动会同时影响 prototypes 和未来 apps/***：需要在 commit / PR 评审时双检查；缓解：T1 治理脚本 + 视觉回归覆盖 prototypes，Wave 1 起补充 apps/web-admin 的视觉/单元/契约测试
- **过早抽离的反向风险**：当前 packages/ 只有 ui 一个包，`api-client / domain-types / utils` 仍是空目录；后续抽离应延后到 SPIKE-003 完成、API 契约定型之后；缓解：本 ADR 只决定 ui 抽离，其他包延到 Wave 1 W1.C 决策

---

## 后续（非本 ADR 范围，仅记录）

- `packages/api-client/`：依赖 SPIKE-003（utoipa→OpenAPI→TS），Wave 1 W1.C 启动
- `packages/domain-types/`：与 api-client 同期，自动生成 + 手工补充
- `packages/utils/`：当前 `cn()` 已在 `@wms/ui/lib/utils`；其他通用工具按需后置
- `apps/pda-mobile/`：React Native，是否复用 `@wms/ui`（视觉规范）需要单独 ADR——RN 不支持 Tailwind，组件可能要 Native 版本；见 SPIKE-005 决策记录

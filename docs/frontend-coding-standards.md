# 前端编码规范

> 适用范围：`prototypes/`（当前）+ Wave 1 起 `apps/web-admin/` `apps/pda-mobile/`
> 关联：[ADR-0001](adr/0001-tech-stack.md) 技术栈 / [ADR-0015](adr/0015-multi-end-rules.md) 多端规则 / [ADR-0021](adr/0021-high-fidelity-prototype-strategy.md) 原型策略 / [ADR-0022](adr/0022-prototype-component-spec.md) 组件规范
> 前后端生产分层边界见 `docs/layered-design.md`。
> 治理：4 个 `check_component_*.py` 脚本（T1）

---

## 0. 规范层级

| 维度 | 强制级别 | 校验方式 |
|---|---|---|
| 项目结构 / 目录层级 | 🔴 强制 | `check_component_registry_consistency.py` |
| 命名规范 | 🔴 强制 | `check_file_naming.py` + `check_component_*` |
| 组件接口（forwardRef / className 转发） | 🔴 强制 | `check_component_props_classname.py` |
| 风格统一（无 inline style） | 🔴 强制 | `check_component_no_inline_style.py` |
| 文档头规范 | 🔴 强制 | `check_component_doc_header.py` |
| 状态枚举对齐 | 🟡 建议 | 人工 review |
| PDA 触控 / 字号基线 | 🔴 强制（PDA 故事） | `check_prototype_usability_baseline.py` |

---

## 1. 项目结构（强制）

```
prototypes/
├── src/
│   ├── components/
│   │   ├── ui/              # Layer 1 基础原子（shadcn/ui 直接复制 + 主题覆盖）
│   │   │   ├── button.tsx
│   │   │   ├── input.tsx
│   │   │   ├── select.tsx
│   │   │   └── ...
│   │   └── business/        # Layer 2 业务复合（WMS 专属语义）
│   │       ├── StatusBadge/
│   │       ├── OfflineIndicator/
│   │       ├── ScanInput/
│   │       └── ...
│   ├── pages/               # Layer 3 页面级（消费 Layer 1/2）
│   │   ├── h1-login-pda/
│   │   ├── h1-login-pc/
│   │   └── ...
│   ├── lib/
│   │   └── utils.ts         # cn() 等共享工具
│   ├── globals.css          # tailwind base + CSS 变量
│   └── main.tsx
└── docs/prototypes/
    ├── component-registry.md   # 业务复合组件唯一注册表
    └── index.toml              # 必有原型清单
```

### 1.1 依赖方向（红线）

```
pages → business → ui
ui ⇏ business      # ui 不允许依赖 business
business ⇏ pages   # business 不允许依赖 pages
```

违反 → `check_component_registry_consistency.py` 报错。

---

## 2. 命名规范

### 2.1 文件 / 目录

| 类型 | 规则 | 示例 |
|---|---|---|
| Layer 1 文件 | 小写连字符 `.tsx` | `button.tsx` / `select.tsx` |
| Layer 2 目录 | PascalCase | `StatusBadge/` |
| Layer 2 主文件 | `<目录名>.tsx` | `StatusBadge.tsx` |
| Layer 2 索引 | `index.ts` | — |
| Layer 3 目录 | 短横线 + 故事 ID 前缀 | `h1-login-pda/` |
| Layer 3 主文件 | PascalCase | `H1LoginPda.tsx` |
| Story 文件 | `<Component>.stories.tsx` | `StatusBadge.stories.tsx` |
| Test 文件 | `<Component>.spec.tsx` | `StatusBadge.spec.tsx` |

### 2.2 标识符

| 类型 | 规则 | 示例 |
|---|---|---|
| 组件 | PascalCase | `StatusBadge` |
| Props 接口 | `<Name>Props` | `StatusBadgeProps` |
| 类型别名 | PascalCase | `StatusKey` |
| Hooks | `use<Verb>` | `useScanFocus` |
| 工具函数 | camelCase | `formatBatchNumber` |
| 常量映射 | UPPER_SNAKE | `STATUS_META` / `MODE_LABEL` |
| CSS 变量 | `--<scope>-<name>` | `--wms-warning` |

### 2.3 状态 / Variant 值

| 维度 | 规则 |
|---|---|
| size 三档 | `sm` \| `default` \| `lg`（**对齐 shadcn**，禁止 md） |
| variant | 由各组件按 cva 自定义；常见 `default` / `secondary` / `outline` / `ghost` / `destructive` / `link` |
| 业务状态 | snake_case（`qualified` / `near_expiry` / `offline_cached`），中文展示由组件内部 META 映射 |
| 布尔 props | `is*` / `has*` / `can*` / 形容词；避免否定（不要 `notDisabled`） |
| 事件 props | `on<Event>`（`onScan` / `onModeChange`） |

---

## 3. 组件接口规范

### 3.1 必备模板（Layer 2 业务复合）

```typescript
import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

/**
 * <ComponentName> — <一句话用途>
 *
 * 层级：Layer 2 业务复合
 * 关联故事：<US-XX-NNN, US-YY-NNN, ...>
 * Wave：<Wave 0.5 / 1.5 / ...>
 * 业务约束：<可选>
 *
 * @example
 *   <ComponentName status="qualified" />
 */

const variants = cva("base classes", {
  variants: { /* ... */ },
  defaultVariants: { /* ... */ },
});

export interface ComponentNameProps
  extends Omit<React.HTMLAttributes<HTMLElement>, "conflicts">,
    VariantProps<typeof variants> {
  // 业务 props
}

export const ComponentName = React.forwardRef<HTMLElement, ComponentNameProps>(
  ({ /* destructure */, className, ...rest }, ref) => {
    return <element ref={ref} className={cn(variants(...), className)} {...rest} />;
  }
);
ComponentName.displayName = "ComponentName";
```

### 3.2 4 条强制约定

1. **必须 `React.forwardRef`** — 让父组件能访问 DOM 节点
2. **必须继承原生 HTMLAttributes** — 自动获得 `className` / `data-testid` / `aria-*` / `id` / `style`（仅作覆盖兜底）
3. **必须用 `cn()` 合并 className** — 外部样式覆盖内置
4. **必须设置 `displayName`** — React DevTools / 错误堆栈可读

### 3.3 Props 设计原则

- **少即是多**：每个 prop 必须有真实使用场景，禁止"将来可能用到"
- **boolean 优先于枚举**：当只有两态时（`disabled` / `loading`），不要做 `state="loading"`
- **受控优先**：对外暴露 value + onChange，内部状态管理在 page 层
- **禁止默认参数副作用**：`onScan = () => {}` ❌ → `onScan?: ...` ✅

---

## 4. 风格统一

### 4.1 颜色

| ✅ 推荐 | ❌ 禁止 |
|---|---|
| `bg-primary` | `bg-[#2563EB]` |
| `text-destructive` | `style={{ color: '#DC2626' }}` |
| `border-wms-warning` | `border-orange-500`（直写 tailwind palette） |
| 引用 CSS 变量 | 引用 tokens/colors.ts hex 值 |

**所有颜色必须经过 `globals.css` 的 CSS 变量定义**，便于主题切换 / dark mode。

### 4.2 间距 / 字号 / 圆角

| ✅ 推荐 | ❌ 禁止 |
|---|---|
| `p-4` `gap-2` `space-y-3` | `style={{ padding: 16 }}` |
| `text-sm` `text-base` `text-lg` | `style={{ fontSize: 14 }}` |
| `rounded-md` `rounded-lg` | `style={{ borderRadius: 6 }}` |
| `h-9` `h-12` | `style={{ height: 36 }}` |

### 4.3 inline style 例外（必须注释说明）

仅以下场景允许 inline style：

```typescript
// ✅ 动态计算（CSS 变量驱动 grid 列宽）
<div style={{ gridTemplateColumns: `${labelWidth} 1fr` }} />

// ✅ 数据可视化（图表宽度依赖运行时数据）
<div style={{ width: `${percent}%` }} />

// ❌ 静态值（禁止）
<div style={{ padding: 16, color: '#000' }} />
```

每处 inline style **必须**前面加 `// 动态：xxx` 注释，治理脚本会按注释豁免。

### 4.4 字体

`globals.css` 已全局设置中文字体栈。**业务复合组件不允许重复声明** `font-sans`。

---

## 5. 文档头规范（强制）

每个 Layer 2/3 组件 `.tsx` 顶部强制注释模板：

```typescript
/**
 * <ComponentName> — <一句话用途，≤ 30 字>
 *
 * 层级：<Layer 1 | Layer 2 业务复合 | Layer 3 页面>
 * 关联故事：<US-XX-NNN 列表 | "全部状态机故事" | "无业务关联（基础原子）">
 * Wave：<Wave 0.5 起步 / Wave 1.5 / ...>
 * 业务约束：<可选；如 PDA 触控 ≥ 48pt / GSP 字段映射对齐 §X / 离线降级>
 *
 * @example
 *   <ComponentName ... />
 */
```

**5 项强制字段**：
- 一句话用途
- 层级
- 关联故事（"无业务关联"也要明确写）
- Wave 归属
- `@example`

`check_component_doc_header.py` 缺一项即报错。

---

## 6. 状态枚举对齐

业务状态枚举（StatusKey 等）必须与 `docs/prototypes/component-registry.md §4.3` 状态映射表一一对应。

新增状态时：
1. 先改 component-registry.md §4.3
2. 再改组件代码 + STATUS_META
3. PR 描述说明业务理由

擅自新增枚举（registry 没有）→ wms-reviewer 拒绝合并。

---

## 7. PDA 端额外约束

PDA 端组件（含 PDA 模式）必须满足 `docs/infra/usability-baseline.md §2.1`：

| 项 | 阈值 |
|---|---|
| 触控目标最小尺寸 | ≥ 48pt（约 64px） |
| 字号最小 | ≥ 16pt（约 21px） |
| 行高 | ≥ 1.5 |
| 按钮间距 | ≥ 8pt |
| 离线兜底 | 必须支持 H1 §7 离线策略 |

强制方式：组件 `size="lg"` 或显式 className `h-12 text-base`。

---

## 7.5 页面级文件大小约束（强制）

单个页面 `.tsx` 文件**不允许成为巨石**：

| 行数 | 严重度 | 治理动作 |
|---|---|---|
| < 600 行 | ✅ 通过 | — |
| 600-799 行 | ⚠️ 警告 | `check_page_size.py` 报 warning，强烈建议提取组件 |
| ≥ 800 行 | 🔴 门禁 | `check_page_size.py` 报 error，PR 阻断（除非加豁免标签） |

**触发提取的信号**：
- 重复的 `<table>` 表格代码（≥ 30 行）→ 提取为 `<DataTable>`
- 重复的"标题 + 副标题 + 操作按钮"块 → 提取为 `<PageHeader>`
- 复杂子组件（≥ 50 行）→ 拆到同目录 `components/` 子目录
- 多个 SummaryCard / FilterBar / Pagination → 提取（按需新增组件）

**豁免**：极少数页面因业务复杂度合理超阈值，文件顶部加：

```typescript
// @governance: skip-page-size 三栏布局合理拆分代价高于价值
```

豁免必须有明确理由，PR 评审会重点 review。

### 提取参考

| 原始代码段 | 行数 | 应提取为 | 当前已有 |
|---|---|---|---|
| `<header>` 标题 + 副标题 + 按钮组 | ~15 行 | `<PageHeader>` | ✅ |
| `<table><thead><tbody>` 列表 | ~30-50 行 | `<DataTable>` | ✅ |
| 5-6 列筛选 + 重置/查询按钮 | ~30-50 行 | `<FilterBar>` | ⏸ 待加 |
| 状态卡片（label+value+sub） | ~15 行/个 | `<SummaryCard>` | ⏸ 待加 |
| 弹窗 | 任意 | `<Dialog>`（shadcn） | ✅ |
| 空数据展示 | ~10 行 | `<EmptyState>` | ✅ |

---

## 7.6 管理端布局、视觉风格与动作弹窗规则（强制）

管理型后台页面默认采用“左侧菜单 + 主列表 + 行内操作按钮”布局；页面主区用于检索、查看和选择对象，写操作统一通过按钮打开弹窗完成。

视觉风格规则：

- 后台页面必须以表格、筛选、指标和操作列为主要信息载体，不做营销页、展示页或大面积装饰视觉。
- 首屏优先展示左侧菜单、当前模块标题、关键指标、筛选区和主列表；不要用宣传型首屏、宣传文案、插画或装饰背景占用作业空间。
- 页面默认使用浅色中性底；颜色必须走 CSS 变量，暗色模式保持同等信息层级。内容区用细边框、弱阴影和小圆角区分层级；卡片只用于指标、表格容器、弹窗和重复条目。
- 列表型页面以表格为核心，按钮放在页头或列表行“操作”列；不要把当前单据、当前动作或明细做成页面常驻大卡片。
- 图标和文字按钮用于明确动作；状态用 `StatusBadge` 或同类状态组件表达，不用大色块堆叠。
- 间距保持紧凑但可读，标题不使用宣传型首屏字号；页面内标题、按钮和表格文本不得互相遮挡或溢出。
- 禁止渐变大背景、装饰光斑、嵌套卡片、纯展示型大封面、无业务信息的大图，以及和真实生产页无关的原型说明文字。

| 场景 | 放置位置 | 交互规则 |
|---|---|---|
| 新建对象 | `PageHeader.actions` | 主按钮打开 `<Dialog>`，弹窗内填写表单并提交 |
| 当前单据状态动作 | 列表行“操作”列 | 只展示动作按钮，例如“收货 / 验收 / 双人签字 / 上架”，点击后打开 `<Dialog>` |
| 单据只读详情 | 列表行“详情”按钮 | 打开 `<Dialog>` 查看单据详情、节点状态和明细；不在页面常驻大详情卡 |
| 筛选 / 查询 / 重置 | 筛选区 | 直接在页面内操作，不使用弹窗 |
| 单据明细 | 详情弹窗 | 入库明细、商品行、批号、数量等只读明细随单据详情查看，不在列表页下方常驻 |
| 打印 / 刷新 / 返回 | 页头或工具区 | 直接执行，非表单弹窗 |

按钮规则：

- 按钮文案必须是业务动词或动词短语：`新建 ASN`、`收货`、`验收`、`确认上架`。
- 写操作按钮必须带图标；主创建按钮用 `default`，普通动作用 `outline`。
- 未选择单据、权限不足或提交中时，动作按钮必须 `disabled`。
- 页面上不放常驻大表单；新建、收货、验收、签字、上架这类写操作必须放在 `<Dialog>`。
- 弹窗标题必须等于当前动作，说明文本必须带当前单据号或对象名。
- 弹窗底部统一为“取消 + 主提交按钮”；提交成功关闭弹窗，提交失败保持弹窗并显示错误。
- 同一动作不得同时存在“页面内表单”和“弹窗表单”两套入口。
- 列表型作业页禁止常驻“当前处理单”或“本环节操作”大卡片；当前单据详情和写操作都通过列表行按钮弹窗处理。
- 列表型作业页禁止常驻入库明细、订单明细或商品行明细表；明细随单据详情弹窗查看。
- 单据流程状态（例如收货、验收、双人签字、上架、完成）属于单据详情弹窗，不放在页面全局流程条里误导其他单据。

---

## 7.7 复用优先与缺口标准化规则（强制）

前端改动前必须先查现有实现：页面局部组件、feature hook、api-client 调用、类型、工具函数、`@wms/ui` 基础控件和业务复合组件。

| 优先级 | 复用对象 | 适用场景 |
|---|---|---|
| 1 | 已有 feature hook / api-client / 类型 | 读取、写入、状态刷新、错误处理 |
| 2 | 页面局部组件 | 同一页面内拆分表格、筛选、动作区、弹窗 |
| 3 | `@wms/ui business` | 跨页面复用的 WMS 业务展示或交互 |
| 4 | `@wms/ui ui` | 基础按钮、输入、弹窗、表格、状态徽标 |
| 5 | 浏览器原生能力或标准库 | 简单格式化、集合处理、表单行为 |
| 6 | 新增组件 / 工具函数 | 上述都覆盖不了，且当前任务真实需要 |

没有现成能力时，不在页面里临时堆代码，按层级新增标准单元：

| 新增内容 | 放置位置 | 要求 |
|---|---|---|
| 通用 UI | `packages/ui/src/ui` | 无业务语义，支持 `className` / 原生属性 |
| WMS 业务复合 | `packages/ui/src/business` 或业务模块 | 有业务语义，按组件文档头和 registry 规则 |
| 页面私有组件 | `apps/<app>/src/pages/<context>/` | 只服务当前页面，但能降低页面复杂度 |
| API / 写入编排 | `apps/<app>/src/features/<context>/` | 走 `@wms/api-client`，不裸 `fetch` |
| 通用工具 | 现有 `lib` / `utils` | 小函数、无 UI、无业务状态副作用 |

新增组件或工具函数必须满足：

- 有当前使用点，不为未来场景预留配置。
- props / 参数保持小而清晰，名称不绑定一次性页面变量。
- 可复用但不提前泛化；出现 3 个以上相似用例再提升层级。
- 最终汇报说明复用清单、新增清单、放置理由和验证命令。

禁止复制 3 个相似页面、表单、弹窗或 hook 后只改文案；禁止为小逻辑新增依赖；禁止 `any`、裸 `fetch` 和页面内散落重复请求逻辑。

## 7.8 DataGrid 浮层规则（强制）

DataGrid 表头和表格内触发的弹窗 / 浮层，例如字段筛选、字段显示、视图保存，不得依赖表格内部 `absolute` 定位。表格行数少、父容器存在 `overflow` 或横向滚动时，内部绝对定位会被裁剪。

强制规则：

- 带 `data-datagrid-popover` 的弹窗浮层必须使用 `createPortal` 渲染到 `document.body`，并使用 `fixed` 定位。
- 浮层位置必须根据触发按钮 `getBoundingClientRect()` 和视口尺寸计算，避免低行数表格、横向滚动、页面滚动时被遮挡。
- 列宽拖拽 handle 这类非弹窗交互可以继续使用 `absolute`，但不得标记为 `data-datagrid-popover`。
- 修改 `packages/ui/src/business/DataGrid/**` 后必须运行 `python3 scripts/governance/check_datagrid_popover_portal.py` 或 `just gov-t1`。

门禁：`check_datagrid_popover_portal.py` 会扫描 DataGrid 组件，发现 `data-datagrid-popover` 浮层使用 `absolute` 时直接失败。

---

## 8. 治理脚本（守门）

| 脚本 | Tier | 校验目标 | 例外豁免方式 |
|---|---|---|---|
| `check_component_doc_header.py` | T1 | 5 项强制字段齐全 | `@governance: skip-doc-header` |
| `check_component_props_classname.py` | T1 | Props 接口含 className + forwardRef + displayName（**泛型函数自动豁免 forwardRef**） | `@governance: skip-classname` |
| `check_component_no_inline_style.py` | T1 | 业务复合无静态 inline style | 紧邻上方 `// 动态：理由` 注释 |
| `check_component_registry_consistency.py` | T1 | 业务复合目录 ↔ component-registry.md §3.1 一一对应 | `[[component_exemptions]]` |
| `check_datagrid_popover_portal.py` | T1 | DataGrid 弹窗浮层使用 `createPortal + fixed`，禁止容器内 `absolute` 裁剪回归 | 无 |
| `check_page_size.py` | T1 | 页面 < 600 通过 / 600-799 警告 / ≥ 800 门禁 | `@governance: skip-page-size` |

---

## 8.5 组件选择决策树（流程类组件易混淆）

```
要展示什么？
├─ 业务流程"前进态"（pending → current → completed）
│   └─ 通用：<StepFlow orientation="horizontal | vertical">
│
├─ 已发生事件流（按时间倒序）
│   └─ <AuditTimeline>
│
├─ 审批节点（含审批人 + 意见 + 驳回）
│   └─ <ApprovalFlow>
│
└─ 双人签字特例（first/second/approval 三槽 + M-VR 策略档位）
    └─ <DualSignPanel>
```

**禁止**用 StepFlow 模拟审批流程（缺意见/驳回语义）；**禁止**用 ApprovalFlow 模拟通用进度（业务语义过重）。

---

## 9. 新增组件流程（PR 自查清单）

```markdown
## 自查清单

- [ ] 目录已按层级（ui/ 或 business/ 或 pages/）归位
- [ ] 文件命名符合 §2.1（Layer 2 用 PascalCase 目录）
- [ ] Props 接口继承原生 HTMLAttributes
- [ ] 用 React.forwardRef 转发 ref
- [ ] 用 cn() 合并 className（外部覆盖优先）
- [ ] displayName 已设置
- [ ] 顶部文档头 5 项字段齐全
- [ ] 无静态 inline style（动态计算已加注释）
- [ ] 颜色用 CSS 变量，不直写 hex
- [ ] 间距 / 字号 / 圆角用 tailwind token
- [ ] 状态枚举跟 component-registry.md §4.3 对齐
- [ ] component-registry.md §3.1 表格已注册
- [ ] PDA 端组件满足 usability-baseline §2.1
- [ ] 跑 T1 治理全绿（`just quick-check` 或 `python3 scripts/governance/governance_checks.py --tier T1`）
```

---

## 10. 反模式（FAQ）

### 10.1 ❌ 直接 import tokens hex 颜色

```typescript
// ❌ 旧版做法
import { colors } from "@/tokens";
<div style={{ background: colors.primary }} />

// ✅ 新做法
<div className="bg-primary" />
```

### 10.2 ❌ 自定义 testId 命名

```typescript
// ❌ 自创 testId prop
<StatusBadge testId="status-pending" />

// ✅ 用原生 data-testid
<StatusBadge data-testid="status-pending" />
```

### 10.3 ❌ size="md"

```typescript
// ❌ 跟 shadcn 不对齐
<StatusBadge size="md" />

// ✅
<StatusBadge size="default" />
```

### 10.4 ❌ inline style 设置静态颜色

```typescript
// ❌
<div style={{ background: "#16A34A" }} />

// ✅
<div className="bg-wms-success" />

// ✅（动态值，必须注释）
// 动态：根据温度上下限计算曲线高度
<div style={{ height: `${ratio * 100}%` }} />
```

### 10.5 ❌ business 组件依赖 page

```typescript
// ❌ business/StatusBadge/StatusBadge.tsx
import { somePageHelper } from "@/pages/h2-audit-query/helpers";

// ✅ 把 helper 抽到 lib/ 或 business/ 自己实现
```

### 10.6 ❌ 忘记 forwardRef

```typescript
// ❌
export function StatusBadge({ status }: Props) { ... }

// ✅
export const StatusBadge = React.forwardRef<HTMLSpanElement, Props>(
  ({ status }, ref) => <span ref={ref}>...</span>
);
StatusBadge.displayName = "StatusBadge";
```

---

## 11. 演进与变更

- 本规范遵循 ADR-0022 决策；如规则修订需新建 ADR
- 新增治理脚本须加入 `gate-rules.toml` + `governance_checks.py` T1
- 反模式发现新案例时，回写到 §10
- Wave 1 启动后，本文件适用范围扩展到 `apps/web-admin/`；生产页迁移规则见 §13

---

## 12. 视觉回归治理（T3）

每次 UI 改动**强制**对比 baseline 截图。流程：

```bash
cd prototypes && pnpm dev &                                    # 1. 起 vite
python3 scripts/governance/capture_visual_snapshots.py         # 2. 截 N 个 tab → .visual-snapshots/
python3 scripts/governance/check_visual_regression.py          # 3. 对比 baseline ↔ snapshot
just matrix-e2e-full                                           # 4. T4 全量矩阵 E2E 截图（合并前）
```

### 12.1 阈值（参 governance/visual-baselines/README.md）

| 指标 | 通过 | 警告 | 错误（PR 阻断） |
|---|---|---|---|
| `mean_diff`（64×64 灰度均值差，0-255） | ≤ 2 | 2-10 | > 10 |
| `pixel_diff_ratio`（像素级不同比例） | ≤ 0.5% | 0.5%-3% | > 3% |
| MD5 一致 | ✓ 直接通过 | — | — |
| **底部 30 行非白像素** | **< 5%** | — | **≥ 5%（疑似截断）** |

### 12.2 何时更新 baseline

- ✅ 业务方走查 approved 的视觉调整 → 先跑截图，再用 `accept_baseline.py --reviewer="<name>"` dry-run，确认通过后用 `--apply` 接受；禁止裸 `cp`
- ❌ 回归 bug 不能更新 baseline（先修代码让脚本回到 0 差异）

### 12.3 加新原型页的强制流程（红线）

加 page 必须**同步三处**，否则 `check_baseline_completeness.py` 会阻断 PR：

1. 写 `prototypes/src/pages/<kebab-name>/<PascalName>.tsx`
2. 在 `prototypes/src/Tabs.tsx` 的 `TABS` 数组追加：
   ```tsx
   { value: "kebab-name", label: "页面名", group: "M? 模块名", device: ["pc"|"pda"|"pad"],
     render: () => wrap(<PascalName />) },
   ```
3. 在 `governance/visual-baselines/manifest.toml` 追加：
   ```toml
   [[snapshots]]
   tab = "kebab-name"
   url_hash = "#kebab-name"
   viewport = "1500x1100"        # PDA 双视图最少 1500x1100；长 PDA 1500x1700+
   file = "kebab-name.png"
   reviewed_by = "项目主人"
   reviewed_at = "YYYY-MM-DD"
   related_story = "US-X-NNN ..."
   ```
4. 起 vite + 跑 capture + 通过接受门禁写入 baseline:
   ```bash
   python3 scripts/governance/capture_visual_snapshots.py
   python3 scripts/governance/accept_baseline.py --reviewer="项目主人" --tab=<kebab-name>
   python3 scripts/governance/accept_baseline.py --apply --reviewer="项目主人" --tab=<kebab-name>
   python3 scripts/governance/check_visual_regression.py    # 应 0 差异
   ```
5. **人工 review 截图视觉无异常**：底部不截断 / 无偏移 / 无遮挡 / 内容对齐

### 12.4 视觉打磨清单（review 时检查）

每张新截图必须依次确认（治理脚本 + 人眼）：

- [ ] **截断检测**通过（底部 30 行非白 < 5%）— 自动
- [ ] **响应式**：sidebar + main 不互相遮挡 — 人工
- [ ] **居中/对齐**：内容紧贴 main 起点（不要 `justify-center` 让 PDA 视图偏右）— 人工
- [ ] **复杂组件**（PrintPreview/TempChart/AuditTimeline）核心内容**完整可见**，不靠滚动 — 人工
- [ ] **viewport 高度**预留 200+ px 空白，避免内容贴底 — 人工
- [ ] **状态色阶**符合 design tokens（参 §3 颜色系统）— 人工

### 12.5 治理脚本一览

| 脚本 | Tier | 作用 |
|---|---|---|
| `check_baseline_completeness.py` | T1 | Tabs.tsx ↔ manifest.toml ↔ baseline PNG 三者一致（强制） |
| `check_e2e_matrix_completeness.py` | T1 | Matrix E2E 策略覆盖全部 baseline tab |
| `check_prototype_navigation.py` | T1 | 原型预览导航必须保持领域 / 模块 / 页面三层结构 |
| `capture_visual_snapshots.py` | 工具 | chrome headless 截图（按 manifest.toml 配置） |
| `check_visual_regression.py` | T3 | MD5 + 像素 + 64×64 感知差异 + 底部截断检测 |
| `run_matrix_e2e_screenshots.py` | T4 | Playwright 全量矩阵 E2E 截图 + DOM / 交互健康检查 |
| `check_matrix_e2e_report.py` | T4 | 校验 Matrix E2E 聚合报告 |

Matrix E2E 详细规范见 [docs/prototypes/matrix-e2e-screenshot-gate.md](prototypes/matrix-e2e-screenshot-gate.md)。

## 13. 前端原型先行与生产迁移（ADR-0029）

### 13.1 三个前端层的职责

| 位置 | 职责 | 禁止 |
|---|---|---|
| `prototypes/` | 高保真原型、业务走查、mock 流程、视觉 baseline | 生产 API 调用、真实权限判断、持久化写操作 |
| `packages/ui` | 共享 primitive / 业务复合组件 / design tokens / stories | 页面级编排、API 调用、领域规则 |
| `apps/web-admin` | 生产 PC/PAD 前端：路由、权限门控、API client、TanStack Query | 未确认原型、mock-only 逻辑、裸 `fetch` |

### 13.2 新原型页规则

1. 必须关联用户故事；没有故事覆盖时先走缺口确认流程。
2. mock 数据字段必须来自用户故事字段表或 OpenAPI 草案，禁止为展示随意发明字段。
3. 复用 `@wms/ui`，跨 3 个页面以上复用的交互必须提取到 Layer 2 组件。
4. 必须执行 §12.3 的 Tabs.tsx / manifest.toml / baseline PNG 三同步。
5. 原型走查 approved 后，才能进入生产迁移。

### 13.3 原型转生产规则

迁移到 `apps/web-admin` 或 `apps/pda-mobile` 前，必须完成 `docs/prototypes/prototype-to-production.md` checklist。

生产页必须：

- 使用 `@wms/api-client` 和 TanStack Query，禁止裸 `fetch`。
- 使用 H1 权限门控页面入口、按钮和数据范围。
- 写操作明确 H2 审计事件或豁免理由。
- 写操作覆盖 ADR-0006 必备维度：L4 错误、L5 数据一致、L8 权限、L11 幂等。
- 删除原型专用 mock、演示账号、演示说明和假交互。

禁止把 `prototypes/src/pages/*` 直接复制到 `apps/web-admin` 后只改 import；页面必须按生产路由、API 契约和 TDD 测试重新落地。

# 前端编码规范

> 适用范围：`prototypes/`（当前）+ Wave 1 起 `apps/web-admin/` `apps/pda-mobile/`
> 关联：[ADR-0001](adr/0001-tech-stack.md) 技术栈 / [ADR-0015](adr/0015-multi-end-rules.md) 多端规则 / [ADR-0021](adr/0021-high-fidelity-prototype-strategy.md) 原型策略 / [ADR-0022](adr/0022-prototype-component-spec.md) 组件规范
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

## 8. 治理脚本（守门）

| 脚本 | Tier | 校验目标 | 例外豁免方式 |
|---|---|---|---|
| `check_component_doc_header.py` | T1 | 5 项强制字段齐全 | 顶部注释加 `@governance: skip-doc-header` 标签 + 理由 |
| `check_component_props_classname.py` | T1 | Props 接口含 className（继承或显式） | 接口注释加 `@governance: skip-classname` |
| `check_component_no_inline_style.py` | T1 | 业务复合无静态 inline style | 紧邻上方 `// 动态：理由` 注释豁免 |
| `check_component_registry_consistency.py` | T1 | 业务复合目录 ↔ component-registry.md §3.1 一一对应 | `governance/check-data.toml` 配置 `[[component_exemptions]]` |

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
- Wave 1 启动后，本文件适用范围扩展到 `apps/web-admin/`，可能新增"路由约定 / 状态管理 / API 调用"等章节

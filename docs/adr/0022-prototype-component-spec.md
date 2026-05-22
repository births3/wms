# ADR-0022：原型组件规范

- 状态：Accepted
- 决策日期：2026-05-22
- 决策人：项目主人
- 关联：ADR-0001（技术栈）/ ADR-0021（高保真原型策略）/ docs/prototypes/component-registry.md

---

## 背景

ADR-0021 §2 决定 Layer 1 用 shadcn/ui，Layer 2 业务复合自制。批次 1 完成 5 个业务复合 + 1 个 DiffPanel 后，发现：

- 业务复合用 inline `style={{}}` 对象，shadcn 用 tailwind className → 风格混搭，主题不可统一
- 命名不一致（size 三档 sm/md/lg vs sm/default/lg；status vs variant）
- Props 接口缺标准（className 转发不一致；testId 命名不统一）
- 缺顶部文档头，新人接手成本高
- 缺治理脚本守门，规范靠人工 review

不规范化会让原型阶段产出**Wave 1 无法直接复用**，违背 ADR-0021 的核心收益。

---

## 决策

### §1 三层架构

```
prototypes/src/components/
├── ui/              # Layer 1: shadcn/ui 基础原子（Button/Input/Select/...）
└── business/        # Layer 2: 业务复合（StatusBadge/ScanInput/...）
prototypes/src/pages/  # Layer 3: 页面级
```

新增组件按层级归位，**禁止跨层依赖反向**（business 可依赖 ui，ui 不依赖 business）。

### §2 命名规范

| 项 | 规则 |
|---|---|
| 目录 | PascalCase（`StatusBadge/`）|
| 主文件 | PascalCase.tsx |
| 索引 | `index.ts` 转发导出 |
| Props 接口 | `<Name>Props` |
| size 三档 | `sm \| default \| lg`（对齐 shadcn）|
| 状态枚举 | 小写下划线（`qualified` / `near_expiry`）|
| 事件 props | `on<Event>`（如 `onScan`）|
| 布尔 props | `is*` / `has*` / `can*` 或形容词；避免否定 |

### §3 Props 接口约定

每个组件 Props 必须：
1. **继承原生 HTML 属性**（如 `extends React.HTMLAttributes<HTMLDivElement>`），自动支持 `className` / `data-testid` / `aria-*`
2. **用 `React.forwardRef`** 转发 ref
3. **用 `cn()` 合并 className**，外部覆盖优先
4. **不允许内联 `style={{}}`** （除动态计算外，如温度曲线宽度）

### §4 风格统一

- ✅ 所有样式用 tailwind className
- ✅ 颜色用 CSS 变量（`bg-primary` / `text-destructive`），不直写 hex
- ✅ 间距用 tailwind token（`p-4`），不写 `padding: 16px`
- ✅ Variant 用 `cva()`（class-variance-authority）
- ❌ 禁止 inline `style` 对象（动态计算除外，需注释说明）
- ❌ 禁止从 tokens/colors 直接 import 颜色 hex 值（用 CSS 变量替代）

### §5 文档头规范

每个组件 `.tsx` 顶部强制注释：

```typescript
/**
 * <ComponentName> — <一句话用途>
 *
 * 层级：Layer 1 / Layer 2
 * 关联故事：<故事 ID 列表 或 "全部"/"基础原子无业务关联">
 * Wave：<Wave 0.5 / 1.5 / ...>
 * 业务约束：<可选；如 PDA 触控基线 / GSP 字段映射 等>
 *
 * @example
 *   <ComponentName ... />
 */
```

### §6 必备产物

每个 Layer 2 业务复合组件目录必须含：
- `<Name>.tsx`（实现，含规范化文档头）
- `index.ts`（导出 + Props 类型）

可选（Wave 1 启动后强制）：
- `<Name>.stories.tsx`（Storybook story）
- `<Name>.test.tsx`（单元测试）

### §7 注册流程

新增 Layer 2 组件 PR 必须：
1. 在 `docs/prototypes/component-registry.md` §3.1 表格注册
2. 标注层级、覆盖故事、批次归属、状态
3. PR 描述包含自查清单（见 §8）
4. wms-reviewer 通过

### §8 PR 自查清单

```markdown
- [ ] 目录已按层级（ui/ 或 business/）归位
- [ ] Props 接口继承原生 HTML 属性
- [ ] 用 React.forwardRef 转发 ref
- [ ] 用 cn() 合并 className
- [ ] 无 inline style（动态计算除外，已加注释）
- [ ] 顶部文档头完整
- [ ] component-registry.md 已更新
- [ ] 跑过 T1 治理全绿
```

---

## 治理

- **check_component_doc_header.py**：组件顶部文档头校验
- **check_component_props_classname.py**：Props 接口必须支持 className
- **check_component_no_inline_style.py**：业务复合禁止 inline style
- **check_component_registry_consistency.py**：注册表 ↔ 实际目录一致

---

## 后果

- 6 个现有业务复合组件需重写为 tailwind + cva 风格（约 250 行净减少）
- 增加 4 个治理脚本进 T1
- Wave 1 启动时业务复合可直接复用，无重写
- 新人接手成本下降（统一接口 + 文档头 + 注册流程）

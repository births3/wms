# 原型组件库注册表（Component Registry）

> 定位：原型治理文档，Layer 2 业务复合组件的唯一注册源
> 关联：ADR-0021（待写）/ docs/infra/usability-baseline.md / prototype-matrix-r3.md
> 规则：新增组件必须在此注册；原型页禁止重复造已注册组件

---

## 1. Design Tokens（Layer 0）

设计 token 以 CSS 变量形式定义在 `packages/ui/src/styles/globals.css`，由 `packages/ui/tailwind-preset.cjs` 暴露给消费方使用。

| Token 类别 | 位置 | 说明 |
|---|---|---|
| 颜色 CSS 变量 | `packages/ui/src/styles/globals.css` `:root {}` | shadcn 标准 + WMS 业务色（`--wms-warning` / `--wms-success` / `--wms-cold`）|
| Tailwind 颜色映射 | `packages/ui/tailwind-preset.cjs` | `bg-primary` / `text-wms-cold` 等 utility |
| 字号 | tailwind 默认（`text-xs/sm/base/lg/...`） | PDA 业务硬约束见 §1.2 |
| 间距 | tailwind 默认（4px 基准 `p-1/p-2/...`） | 不引入自定义 token |
| 字体栈 | `packages/ui/tailwind-preset.cjs` `fontFamily.sans` | 中文字体 PingFang SC / Microsoft YaHei 兜底 |

### 1.1 颜色规范

| 语义 | Token 名 | 值（参考） | 用途 |
|---|---|---|---|
| 主色 | `--wms-primary` | `#2563EB` (blue-600) | 主按钮/链接/选中态 |
| 主色浅 | `--wms-primary-light` | `#DBEAFE` (blue-100) | 背景高亮 |
| 危险 | `--wms-danger` | `#DC2626` (red-600) | 不合格/过期/超标/删除 |
| 警告 | `--wms-warning` | `#D97706` (amber-600) | 近效期/待处理/异常 |
| 成功 | `--wms-success` | `#16A34A` (green-600) | 合格/完成/通过 |
| 冷链 | `--wms-cold` | `#0891B2` (cyan-600) | 冷链相关标识 |
| 中性 | `--wms-neutral-*` | gray-50~900 | 文字/边框/背景 |

### 1.2 PDA 端硬约束

| 约束 | 值 | 来源 |
|---|---|---|
| 最小触控目标 | 48×48pt | usability-baseline §2.1 |
| 最小字号 | 16pt | usability-baseline §2.1 |
| 最小行高 | 1.5 | WCAG 2.1 AA |
| 按钮间距 | ≥ 8pt | 防误触 |

---

## 2. 主题覆盖（Layer 1）

文件：`packages/ui/src/styles/globals.css`（消费方在应用入口 `import "@wms/ui/styles/globals.css"`）

shadcn/ui 标准 CSS 变量 + WMS 自定义业务色，一处定义两端共用：

```css
:root {
  --radius: 0.5rem;
  --primary: 217 84% 53%;            /* #2563EB blue-600 */
  --destructive: 0 72% 51%;           /* #DC2626 red-600 */

  /* WMS 业务色 */
  --wms-warning: 32 95% 44%;          /* 近效期/异常 */
  --wms-success: 142 71% 45%;         /* 合格/通过 */
  --wms-cold:    189 94% 43%;         /* 冷链 */
}
```

不新增组件库，**直接用 shadcn/ui 原生组件**（已迁至 `packages/ui/src/ui/`）。

---

## 3. 业务复合组件注册表（Layer 2）

### 注册规则

1. 跨 ≥3 个原型页复用的交互模式 → 必须提取为组件
2. 每个组件必须有 `.stories.tsx`（Storybook 即文档）
3. PDA 端组件必须有 `minTouchTarget` prop（默认 48）
4. 组件命名：PascalCase，前缀按端（无前缀=跨端）

### 3.1 组件清单

| # | 组件名 | 端 | 职责 | 覆盖故事（示例） | 状态 |
|---|---|---|---|---|---|
| 1 | **ScanInput** | PDA | 扫码输入框：摄像头/扫枪/手动切换，含扫码动画反馈 | M2-002/003, M4-003, TC-004/006, BA-003 | 已开发 |
| 2 | **StepFlow** | PDA | 多步骤流程指示器：当前步高亮，已完成打勾，支持回退 | M2-003(14步), M2-004(双人), M4-003(拣选), M2-005(上架) | 已开发 |
| 3 | **StatusBadge** | 跨端 | 状态标签：颜色自动映射（合格=绿/不合格=红/待处理=橙/隔离=灰） | 全部状态机故事 | 已开发 |
| 4 | **FieldTable** | PDA | 字段核对表：左标签右值，支持扫码自动填充高亮 | M2-003(验收), M4-004(复核), M3-005(盘点) | 已开发 |
| 5 | **OfflineIndicator** | PDA | 离线状态指示：顶部 banner + 暂存计数 + 同步进度 | 全部 PDA 故事 | 已开发 |
| 6 | **DualSignPanel** | PDA+PC | 双人签字面板：第一人签→等待→第二人签→完成，含策略档位显示 | M2-004, VR-006, BA-002 | 已开发 |
| 7 | **AuditTimeline** | PC | 审计时间线：纵向时间轴，展开详情，支持筛选 | H2-002, M6-002, M6-004 | 已开发 |
| 8 | **KanbanBoard** | PC+PAD | 看板大屏：多列卡片，实时刷新，支持拖拽（PC）/只读（PAD） | M2-008, M4-007, TE-009, DOCK-006 | 已开发 |
| 9 | **PrintPreview** | PC+PAD | 打印预览容器：A4/标签/面单三种模板，缩放+翻页 | M2-007, M4-005, H5-003, PK-006 | 已开发 |
| 10 | **TempChart** | PC | 温度曲线图：时间轴+温度线+超标区域着色+阈值线 | M5-002/003, M10-002, ST-006 | 已开发 |
| 11 | **RuleEditor** | PC | 规则配置编辑器：条件组合（AND/OR）+ 动作 + 测试运行 | VR-001, MPM-002, M9-001, AL-001 | 已开发 |
| 12 | **ApprovalFlow** | PC | 审批流程面板：节点图+当前节点高亮+意见+驳回 | QL-003, BA-002, DOCK-004 | 已开发 |
| 13 | **DiffPanel** | PC | 旧值-新值对比面板：变化字段加粗高亮 | H2-002, M-BA-001, M-VR-003, M1-008 | 已开发 |
| 14 | **PageHeader** | 跨端 | 管理页统一头部：标题+副标题+操作区+面包屑 | 全部 PC 管理页 | 已开发 |
| 15 | **DataTable** | 跨端 | 通用数据表格：列定义+选中态+空状态+翻页槽（基于 ui/Table） | 全部列表型管理页 | 已开发 |
| 16 | **DataGrid** | PC | 管理页数据网格：客户端分页、排序、字段筛选、列显隐、列宽拖拽、点击复制、视图保存 | M2 收货管理列表 | 已开发 |
| 17 | **EmptyState** | 跨端 | 空状态展示：图标+标题+描述+CTA | 全部列表/看板/详情 | 已开发 |
| 18 | **SystemDictionaryTwoPane** | PC | 系统字典两层展示：左侧字典分类，右侧选中分类的字典项、来源、启停状态与参数摘要 | M1-011 系统字典 / 单据类型参数配置 | 已开发 |

DataGrid 组件说明：字段筛选、字段显示、视图保存等表格浮层必须使用 `createPortal + fixed`，禁止在表格容器内用 `absolute` 渲染带 `data-datagrid-popover` 的弹窗，避免低行数表格或 `overflow` 容器裁剪；按钮触发浮层必须支持点击外部和 `Escape` 关闭。该约束由 `check_datagrid_popover_portal.py` 强制检查。

### 3.2 组件依赖关系

```
FieldTable → ScanInput（FieldTable 消费 ScanInput 的扫码自动填充）
DualSignPanel → StepFlow（双人签字是 StepFlow 的特例）
KanbanBoard → StatusBadge（卡片内状态标签）
AuditTimeline → StatusBadge（事件状态标签）
TempChart → OfflineIndicator（离线时显示缓存数据标记）
```

### 3.3 开发优先级

| 批次 | 组件 | 理由 | 交付时间 |
|---|---|---|---|
| **批次 1**（Wave 0.5 Week 1） | ScanInput, StepFlow, StatusBadge, FieldTable, OfflineIndicator | PDA 全部依赖这 5 个 | Day 3-5 |
| **批次 2**（Wave 0.5 Week 2） | DualSignPanel, AuditTimeline, KanbanBoard, PrintPreview, TempChart, RuleEditor, ApprovalFlow | PC/PAD 原型依赖 | Day 1-2 |

---

## 4. 组件接口规范

### 4.1 通用 Props

所有 Layer 2 组件必须支持：

```typescript
interface WmsComponentProps {
  className?: string;        // 样式覆盖
  testId?: string;           // E2E 测试锚点
}
```

### 4.2 PDA 组件额外 Props

```typescript
interface PdaComponentProps extends WmsComponentProps {
  minTouchTarget?: number;   // 默认 48
  offlineMode?: boolean;     // 离线模式下的降级行为
}
```

### 4.3 状态映射表（StatusBadge 消费）

| 状态值 | 颜色 Token | 文字 | 图标 |
|---|---|---|---|
| `qualified` | `--wms-success` | 合格 | ✓ |
| `unqualified` | `--wms-danger` | 不合格 | ✗ |
| `pending` | `--wms-warning` | 待处理 | ⏳ |
| `isolated` | `--wms-neutral-500` | 隔离 | 🔒 |
| `expired` | `--wms-danger` | 已过期 | ⚠ |
| `near_expiry` | `--wms-warning` | 近效期 | ⚠ |
| `in_progress` | `--wms-primary` | 进行中 | → |
| `completed` | `--wms-success` | 已完成 | ✓ |
| `offline_cached` | `--wms-neutral-400` | 离线暂存 | ☁ |

---

## 5. 变更记录

| 日期 | 变更 |
|---|---|
| 2026-05-22 | 初版：12 个组件 + Design Tokens + 主题规范 |
| 2026-05-23 | 扩到 16 个组件（+ DiffPanel / PageHeader / DataTable / EmptyState） |
| 2026-05-23 | **路径迁移**：Layer 0/1/2 全部从 `prototypes/src/` 抽离至 `packages/ui/src/`（commit e3ce5a0），构成 `@wms/ui` 共享包；详见 ADR-0022 v0.2 修订记录 + ADR-0028 |
| 2026-06-28 | 扩到 17 个组件（+ DataGrid），用于管理页分页、排序、字段筛选、字段显隐、列宽拖拽、点击复制和视图保存 |
| 2026-06-29 | 扩到 18 个组件（+ SystemDictionaryTwoPane），用于 M1 系统字典左右两层展示 |

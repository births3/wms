# H1 菜单管理设计

> 目的：把 PC 管理端菜单从前端写死升级为后端驱动的三层菜单树，并为 H1 菜单权限和按钮权限提供统一来源。

## 已确认决策

| 决策点 | 结论 |
|---|---|
| 菜单层级 | `业务域 -> 能力组 -> 页面` 三层 |
| 菜单结构 | 全局统一，货主不覆盖菜单结构 |
| 存储模型 | PostgreSQL 邻接表 |
| 管理入口 | `基础能力 -> H1 权限租户 -> H1 菜单管理` |
| 页面绑定 | 只能绑定前端已注册 `view_id` |
| 图标 | 后端存 `icon_key`，前端用 lucide 白名单映射 |
| 生效方式 | 草稿编辑，发布后一次性生效 |
| 版本 | 每次发布生成版本，可回滚上一版 |
| 权限点 | 菜单节点维护按钮权限点 |
| 按钮来源 | 内置标准动作 + 页面私有动作 |
| 故障回退 | fail-closed：菜单接口失败、空树或缺少工作台入口时只显示 Dashboard、错误和重试 |

## 数据模型

菜单采用邻接表，限制三层，不引入闭包表。

核心表：

- `admin_menu_draft_nodes`：当前草稿节点。
- `admin_menu_draft_button_permissions`：草稿节点按钮权限点。
- `admin_menu_versions`：发布版本头。
- `admin_menu_version_nodes`：历史版本节点快照。
- `admin_menu_version_button_permissions`：历史版本按钮权限点快照。

核心字段：

- `id`：节点 ID。
- `parent_id`：父节点 ID，一级为空。
- `level`：固定 1、2、3。
- `code`：稳定编码，如 `inbound.receiving`。
- `path`：完整路径，如 `inbound/inbound_operation/m2_receiving`。
- `title`：菜单展示名。
- `view_id`：前端页面 ID，仅三级页面必填。
- `icon_key`：图标白名单键。
- `permission_key`：菜单可见权限键。
- `sort_order`：同级排序。
- `enabled`：是否启用。

## API

| API | 用途 |
|---|---|
| `GET /api/v1/admin/menus/published` | 前端壳读取已发布三层菜单 |
| `GET /api/v1/admin/menus/draft` | 菜单管理页读取草稿 |
| `POST /api/v1/admin/menus/draft/nodes` | 新增草稿节点 |
| `PATCH /api/v1/admin/menus/draft/nodes/{id}` | 编辑节点、启停、换父级、排序 |
| `POST /api/v1/admin/menus/draft/batch-enable` | 批量启停 |
| `POST /api/v1/admin/menus/publish` | 校验并发布草稿 |
| `POST /api/v1/admin/menus/rollback` | 回滚到上一发布版本 |

写接口必须校验 `Idempotency-Key`。发布校验必须覆盖：

1. 只有三层。
2. 一级没有 `parent_id`，二/三级必须有父级。
3. 三级启用节点必须绑定已注册 `view_id`。
4. `code`、`path`、`permission_key` 唯一。
5. 同级 `sort_order` 可排序。
6. 图标必须来自白名单。

## 前端

管理端壳从 `published` API 获取菜单树。生产故障回退采用 fail-closed：接口失败、返回空树、当前视图被移除或用户无可用菜单时，只保留 Dashboard、错误提示和重试入口，不使用完整本地菜单树越权展示页面；开发菜单 mock 只用于接线走查。

H1 菜单管理页面使用左右分区：

- 左侧：三层菜单树，支持展开、搜索、拖拽排序、拖拽换父级、批量选择。
- 右侧：节点编辑、绑定 `view_id`、图标、启停、按钮权限点。
- 顶部：发布、回滚、刷新。

按钮权限点分两类：

- 标准动作：新增、编辑、删除、启停、详情、查询、刷新、导出、打印、汇总、字段、视图。
- 私有动作：页面按业务补充，如收货、验收、上架、发布模板。

## 不做的事

- 不支持货主自定义菜单树。
- 不把菜单存入系统字典。
- 不引入闭包表。
- 不做按钮权限和角色矩阵的完整授权页面；本次只维护权限点。

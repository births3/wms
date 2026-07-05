---
name: wms-page-query-governance
description: 管理端页面级查询条件治理技能。用户要求新增页面查询分类、核心/更多查询条件、QueryPanel 折叠、自动建议、check_admin_page_query_panel 失败修复，或新增菜单页后自动登记查询配置时使用。
---

# WMS Page Query Governance

用于把管理端页面的查询条件按“核心查询 + 更多查询”治理到可检查状态，避免新增菜单页后漏接公共 QueryPanel。

## 先读

- `AGENTS.md`
- `apps/AGENTS.override.md`
- `docs/frontend-coding-standards.md`
- `apps/web-admin/src/App.tsx`
- `apps/web-admin/src/pages/page-query-core-fields.json`
- `packages/ui/src/business/QueryPanel/QueryPanel.tsx`
- 目标页面文件

## 入口命令

1. 先运行 `python3 scripts/governance/check_admin_page_query_panel.py --suggest`。
2. 再运行 `python3 scripts/governance/check_admin_page_query_panel.py --json`。
3. 脚本失败时先修脚本能定位的问题，再做人工语义判断。

## 分类规则

- `page-query-core-fields.json` 是页面查询分类的事实源；新增 `App.tsx` 菜单页必须登记。
- 新增页面必须先判断展示结构：列表型、双栏目录型、配置型、详情弹窗型，不允许先拼页面再补分类。
- 列表型页面使用公共 `QueryPanel` + `DataGrid`；双栏目录型页面左侧分类 / 目录 / 树，右侧明细列表或编辑区；配置型页面按配置域分区；详情弹窗按业务信息分区上下展示。
- 承载业务列表、DataGrid 或批量操作的页面，默认需要页面上方 QueryPanel。
- 工作台总览、双栏目录内部搜索、配置中心这类非列表查询页，可以 `required=false`，但必须写明 `reason`。
- 历史私有 FilterBar 不能作为豁免理由；承载业务列表的页面要迁移到公共 `QueryPanel`。
- 核心查询只放用户最高频、需要首屏可见的条件：
  - 通用列表默认 `keyword`。
  - 有货主上下文或货主列时加入 `ownerKeyword`。
  - 有状态流转或状态列时加入 `statusFilter`。
- 更多查询放低频或占空间的条件：
  - 单据类型、业务日期、创建时间、批号、库位、商品扩展条件。
  - 日期范围默认放更多查询，除非用户明确要求首屏可见。
- 未知页面族不能自动造业务字段；先登记为 `required=false`，`reason` 写“新增页面待确认页面级查询分类”，再向用户确认。

## 修复流程

1. 根据 `--suggest` 找出缺失菜单页，优先补 `page-query-core-fields.json`。
2. 若页面应接入 QueryPanel，在页面内定义：
   - `xxxQueryFields`
   - `xxxCoreQueryFieldKeys`
   - `<QueryPanel fields={xxxQueryFields} defaultVisibleFieldKeys={xxxCoreQueryFieldKeys} />`
3. 不新造私有查询 UI；先复用公共 `QueryPanel` 和现有字段类型。
4. 对同一页面族复用已有字段常量；只有字段语义确实不同才新增页面级常量。
5. 发现 3 个及以上页面同类遗漏，优先增强 `check_admin_page_query_panel.py`，再批量修页面。

## 验证

- `python3 scripts/governance/check_admin_page_query_panel.py --suggest`
- `python3 scripts/governance/check_admin_page_query_panel.py --json`
- `python3 -m pytest scripts/governance/tests/test_admin_page_query_panel.py -q`
- 涉及前端页面时运行对应 self-check 或 `pnpm --filter @wms/web-admin build`
- 每次改文件后运行 `just gov-t1`

## 停止条件

- 脚本 `ok=true`。
- 新增菜单页已被分类，或明确写入不需要页面上方查询的原因。
- 核心查询首屏可见，更多查询默认折叠。
- 未确认的新业务字段、状态、模块或默认值没有被静默引入。

# 全链路质量矩阵方法

## 目标

质量矩阵用于把用户故事、字段、前端、API、后端、数据库、权限、审计、测试、证据和治理脚本放到同一张可检查表里。后续新增页面、接口、字段或故事时，先登记矩阵，再补实现和证据。

## 事实源

- 机器事实源：`governance/quality-matrix.toml`。
- 展示页：`docs/governance/quality-matrix.md`，由脚本生成，不手工修改。
- 检查脚本：`scripts/governance/check_quality_matrix.py`。

## 分层策略

| 层 | 名称 | 要求 |
|---|---|---|
| S0 | 登记层 | 每个进入执行的用户故事必须登记模块、故事文件、故事类型、页面和 API。 |
| S1 | 核心闭环层 | 每个维度只允许 `verified` 或 `not_applicable`；不适用必须写原因。 |
| S2 | 深度验证层 | 由故事类型自动推导 L1-L11 测试层，`tests=verified` 时必须覆盖全部推导层。 |
| S3 | 运行反馈层 | issue、Bug、review 漏检和线上反馈必须判断是否需要新增矩阵维度、故事类型或脚本规则。 |

上述 S0-S3 是治理成熟度。单个故事另按业务风险推导验收深度，二者不能混用：

| 验收层级 | 适用范围 | 最低验收要求 |
|---|---|---|
| S1 | 查询、展示、普通配置 | 需求、字段、前端/API 可达、权限和基本测试闭环 |
| S2 | 普通业务写操作 | S1 + PostgreSQL、审计、幂等和真实数据 E2E |
| S3 | 库存、状态、批号、冷链、GSP、并发资源和关键路径 | S2 + 并发、事务一致性、异常路径和性能证据 |
| S4 | PDA、硬件、外部系统和发布 | S3 + 真实环境、真实设备或外部回执及人工验收证据 |

验收层级由故事类型自动取最高级：`write` / `integration` 至少 S2，`inventory_change` / `concurrent_resource` / `critical_path` / `audit_compliance` 至少 S3，`pda_runtime` / `hardware_runtime` / `external_runtime` / `release_runtime` 为 S4。执行人不得手工降低层级。

## 完成标准

### 故事级

延期故事从 `deferred_stories` 移入 `stories` 前，必须满足：

1. 需求与验收条件无待定业务语义。
2. 字段、字典、状态和权限点对齐。
3. 前端页面、菜单、按钮、弹窗和错误状态可达；无页面时写明 `not_applicable` 原因。
4. OpenAPI 与生成的 `@wms/api-client` 同步。
5. 后端按既定 handler / service / domain / repository 边界落地。
6. PostgreSQL 表、约束、索引、迁移和货主隔离完成。
7. 写操作具备权限、幂等、审计和适用的并发控制。
8. 故事类型推导的 L1-L11 测试层全部覆盖。
9. 数据写入故事有真实 PostgreSQL 测试；有页面的故事有真实数据 E2E。
10. 十二个质量维度全部为 `verified` 或有理由的 `not_applicable`。
11. 模块严格范围检查通过。
12. 不存在 mock 替代生产实现、无效按钮或只登记未实现。

### 模块级

模块验收必须同时满足：模块内无延期故事；菜单、页面、API、后端和数据库形成完整业务链；正常、拒绝、撤销、重复提交、越权和跨货主路径按适用范围通过；字段矩阵、状态机和审计一致；真实 E2E 覆盖主要流程；质量矩阵、OpenAPI、范围和治理检查通过。

使用以下命令验收，延期故事未清零时硬失败：

```bash
python3 scripts/governance/check_quality_matrix.py --complete-module M2
python3 scripts/governance/check_scope_gap_discovery.py --strict --module M2
```

### 发布级

PDA 必须有真机扫码、离线重放、幂等和易用性证据；外部系统必须有真实 dev/staging 请求、回执、重试和审计证据；硬件必须有设备、校准和产物证据；性能必须有约定数据量下的 P95/P99 与并发结果；发布必须有迁移、灰度、监控、回滚和双人审批证据。代码完成不能替代这些证据。

## 维度

每个故事都必须写完整维度：需求、字段、前端、API、后端、数据库、安全、审计、测试、证据、文档、治理。不能用 `partial`、`missing`、`todo` 这类模糊状态。

## 页面分类分区

新增管理端页面必须先判断页面族和展示结构：

- 列表型页面：页面上方使用公共 `QueryPanel`，字段分为核心查询和更多查询，主体使用 `DataGrid`。
- 双栏目录页面：左侧为分类 / 目录 / 树，右侧为明细列表或编辑区，内部搜索不等同于页面上方查询。
- 配置型页面：按配置域分区展示，保留可审计的启停、导入导出和影响预览入口。
- 详情弹窗：按订单信息、商品信息、批号信息、流程信息、审计信息分区，上下排列，避免把所有流程字段混在同一区块。

新增菜单页必须同步 `apps/web-admin/src/pages/page-query-core-fields.json`；无法确认页面类型时登记为待确认原因，并由用户确认后再实现。

## 自我迭代

运行中发现以下问题时，必须回到矩阵判断是否补规则：

- 三个以上页面出现同类布局、字段、按钮或查询遗漏。
- 前端已经构建但后端 API、OpenAPI、数据库或权限未跟上。
- review 或 issue 暴露出脚本没有检查到的共性问题。
- 新增业务概念影响两个以上模块。

迭代顺序固定为：补事实源 → 补脚本 → 跑失败 → 修实现 → 生成展示页 → 复跑治理。

## 范围缺口自发现标准线

`scripts/governance/check_scope_gap_discovery.py` 是质量矩阵之外的范围缺口发现入口：

- 默认模式用于 T1：只对矩阵接线错误硬失败，活跃模块未登记故事和未覆盖菜单页输出缺口报告。
- 严格模式用于模块验收：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H9`，发现型缺口也失败。
- 新增用户故事、管理端菜单页、页面文件或质量矩阵条目时，必须让该脚本能解释“已覆盖、待登记、或明确不在本轮范围”。
- 明确不在本轮范围的故事写入 `governance/quality-matrix.toml` 的 `[[deferred_stories]]`，必须有原因；这只关闭范围缺口，不表示功能已完成。
- issue、review 或验收反馈发现“页面已有但按钮/弹窗/流程没做”时，先定位对应用户故事是否已进入矩阵；没有进入矩阵则先补矩阵或登记待办，再补实现。
- 管理端页面进入 `frontend_pages` 后，必须同时检查 `menuSections`、`defaultMenuTree`、`renderAdminView` 路由可达和 `apps/web-admin/dev-mocks/admin-menu-dev-mock.ts` 已发布菜单种子，避免只登记菜单标题但默认三层菜单、运行时已发布菜单或页面渲染漏接。
- 带 `frontend_interaction` 且声明管理端页面的故事，必须登记 `e2e_checks`。页面级 self-check 可证明菜单、路由和公共组件接线，但不能替代真实浏览器验收。
- `governance/menu-e2e-screenshot-policy.toml` 生效后新增的菜单页，必须登记真实 Playwright `e2e_checks`，并在所属故事写 `e2e_screenshots = [{ page, spec, screenshot }]`；`spec` 与 `screenshot` 还必须进入 `evidence_refs`。截图路径固定使用 `artifacts/screenshot-portal/real-web/<页面证据目录>/*.png`，由 E2E 运行时生成，不把临时 PNG 入库。
- `legacy_pages` 只冻结规则启用前的页面，不得用于豁免新页面；检查器冻结初始债务上限，清单只允许减少。历史页面补齐证据后应从列表删除。`check_scope_gap_discovery.py` 默认对基线外缺证据页面硬失败，避免“菜单可见但流程未验收”。
- 页面级 self-check 至少覆盖菜单入口、默认菜单树、已发布菜单 dev mock、路由渲染、公共 `QueryPanel` / `DataGrid` 使用、真实后端或 dev mock 数据入口。

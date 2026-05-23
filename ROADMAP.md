# wms ROADMAP

> 长期路线（用户视角简版）。详细决策见 [ADR-0007](docs/adr/0007-roadmap-v03-boundary-alignment.md)。
> 模块依赖与并行规则见 [docs/architecture-dependencies.md](docs/architecture-dependencies.md)。

---

## 总目标

完整实现医药冷链 GSP 合规 WMS 的核心业务模块、横向业务能力和横向技术能力。
M11 监管 EDI 已移除：码上放心由 M-TC 承接，药监 EDI 由 ERP/H8 边界承接。

按依赖图分 5 个波次（Wave 1-5）+ 1 个治理波次（Wave 0），波次内可 worktree 并行，波次间严格依赖。

---

## Wave 0：治理骨架（进行中）

**周期**：1 周
**目标**：项目结构、Git、文档、ADR、治理脚本骨架就位，可跑 `just quick-check`。

- [x] 目录骨架
- [x] Git 配置（.gitignore / .editorconfig / .gitattributes）
- [x] docs/governance.md v0.2
- [x] ADR-0001 技术栈
- [x] ADR-0002 仓库结构
- [x] ADR-0003 治理模型
- [x] ADR-0004 v0.2 波次路线（已由 ADR-0007 取代）
- [x] ADR-0006 TDD + 11 层测试
- [x] ADR-0007 v0.3 路线边界对齐
- [x] docs/architecture-dependencies.md 依赖图
- [x] justfile（T1-T4 入口）
- [x] lefthook.yml
- [x] 治理脚本（公共库 + 4 起步脚本 + 2 调度）
- [x] gate-rules.toml + baseline 占位
- [x] README / ROADMAP / TODO / CHANGELOG / ADR 索引
- [ ] 工作区根 README 登记
- [ ] 本地验证 + 首次 commit

**完成标准**：所有 Wave 0 ADR Accepted；`just quick-check` 跑通；`validate_environment.py` 报告环境就绪。

---

## Wave 0.5：原型 + 技术 Spike（进行中）

**周期**：2 周
**目标**：组件库骨架 + P0 原型（Wave 1 涉及的 9 个页面）+ 技术 Spike 验证。

- [x] Design Tokens + wms-theme.css + Storybook 配置
- [x] Layer 2 业务复合组件 Top 5（ScanInput / StepFlow / StatusBadge / FieldTable / OfflineIndicator）
- [x] Layer 2 剩余组件（DualSignPanel / AuditTimeline / KanbanBoard / PrintPreview / TempChart / RuleEditor / ApprovalFlow）
- [x] P0 原型页（9 个：H1 登录/权限/登出/API Key + H2 审计查询/归档/生命周期 + H3 API 文档）
- [x] 技术 Spike 计划落盘（`docs/spikes/` 5 项 + README，状态=起草）
- [ ] 技术 Spike 验证：SPIKE-001 Axum+JWT / 002 H2 append-only / 003 utoipa→OpenAPI→TS / 004 SQLx offline / 005 RN 扫枪 — 状态需进入 accepted/rejected/deferred

**完成标准**：Storybook 可运行；P0 原型 ≥1 次走查 approved；Spike 结论记录到 `docs/spikes/`。

---

## Wave 1：横向底座（H 层）

**周期**：4-6 周（个人）
**目标**：所有业务模块的基础设施就绪。

并行任务：

- W1.A：H1 权限与多租户基础（角色 / 权限码 / 货主隔离 / JWT）
- W1.B：H2 审计追踪基础设施（append-only / 旧值新值 / 操作人时间 IP）
- W1.C：H3 OpenAPI 契约工具链（utoipa 注解 / openapi.json 生成 / openapi-typescript 消费）
- W1.D：治理脚本扩展 — `check_feature_flags.py`（与 ADR-0016 灰度链路同步上线；校验 Feature Flag owner / 创建日期 / 90 天清理期；过期未清理 PR 不予合并）

**外部资质并行启动**：
- "码上放心"账号开通

**完成标准**：任意业务 handler 可挂 H1；任意写操作经 H2；后端注解可生成 OpenAPI，前端 `@wms/api-client` 可消费；**文件版灰度链路（环境变量 / `deploy/feature_flags.toml` 后端）+ 自动回滚链路在 dev / staging 环境验证可用（参 ADR-0016 §灰度发布策略 v3.1.3；Wave 1 不涉真业务上线，仅验证链路就绪；配置中心版验证留到 W2.G）**；`check_feature_flags.py` 进入 T1 治理脚本集。

---

## Wave 2：业务底座 + Schema 先行

**周期**：3-4 周
**目标**：核心 schema 落地，基础 CRUD 可用。

并行任务：

- W2.A：M1.a 基础档案 schema + 基础 CRUD（商品 / 供应商 / 客户 / 仓库 / 库位 / **特殊药品分类字典 M1-010**）
- W2.B：M2 入库 schema 设计（不写业务规则）
- W2.C：M6 报表查询接口骨架
- W2.E：**M-PM 参数对照模块**（v24 新增）：字典 / 规则 / 待映射队列 / 执行 API / 反向追溯，作为 M1 接收 ERP 不规则字段的前置依赖
- W2.G：Feature Flag 存储后端从 W1 文件版迁移到 M1-008 配置中心（含导出 / 批量导入 / 对账 / 切换读取源 / 旧文件归档；具体脚本路径 / 命名 / 审批主体由本任务实施时定义并回写 ADR-0016 v3.2+；参 ADR-0016 §Feature Flag 治理 v3.1.3）

**完成标准**：核心 schema 落地；商品 / 供应商 / 收货单基础 CRUD 可用；M-PM 可处理 ERP 推送的不规则字段；OpenAPI 反映完整 schema；**配置中心版灰度链路（M1-008 后端）在 dev / staging 验证可用**；W1 文件版 flag 已迁移至 M1-008 且对账通过。

---

## Wave 3：核心业务规则铺开

**周期**：8-10 周
**目标**：单货主下"商品-供应商-入库-库存"闭环。

并行任务：

- W3.A：M2 入库业务规则 + handler + PDA 端
- W3.B：M3 库存模型 + 业务规则（FIFO / 近效期 / 库存状态）
- W3.C：M5 冷链数据接入 schema（接收外部冷链系统数据）
- W3.D：M9 3PL 计费"账户/合同"模型

**完成标准**：M2 / M3 关键路径 11 层测试覆盖；GSP 资质有效期校验生效。

---

## Wave 4：完整闭环 + 横向叠加

**周期**：8-10 周
**目标**：单货主下完整业务闭环可上线试运行。

并行任务：

- W4.A：M4 出库（订单 / 拣选 / 复核 / 打印随货同行单）
- W4.B：M5 冷链数据接入（接收外部冷链系统数据 + 温度超标事件联动批次隔离）
- W4.C：M6 报表实现（GSP 法定台账）
- W4.D：M-TC 码上放心上报（追溯码核销事件实时上报国家平台）
- W4.E：**司机/门店用户主动故事补全（v13 P2 决策）** — 司机端 PDA 签收/上报、门店用户端订单查询/电子签收，从被动角色升级为主动 actor 故事

**完成标准**：完整业务闭环（采购入库 → 库存 → 销售出库 → 冷链监控 → 报表）可上线；GSP 法定台账可生成；审计追踪 append-only 不变量验证通过；**首次正式上线（即本波试运行投产）必须使用 ADR-0016 §灰度发布策略链路，不允许全量直发**。

---

## Wave 5：增值模块全面铺开

**周期**：12-16 周
**目标**：增值业务模块生产可用。

并行任务（最多 3 个 worktree 同时）：

- W5.A：M-PK 包装站增强（电子秤复核 / 打印 / 复杂合箱）
- W5.B：M8 连锁专有（自动补货 / 越库 / O2O）
- W5.C：M9 3PL 计费业务规则（仓储费 / 作业费 / 月结账单）
- W5.D：M10 TMS+（路径优化 / 在途温控数据接入 / 周转箱回收）

**完成标准**：多货主隔离生效；码上放心对接通过测试环境验证；至少一个连锁客户场景跑通。

---

## 总周期估计

> **无法给出可靠估时**。以下仅为数量级参考，将在 Wave 1 完成后基于实际节奏修订。

| 阶段 | 数量级参考 |
|------|-----------|
| Wave 0 治理骨架 | 1 周 |
| Wave 1 横向底座 | 数周 |
| Wave 2 业务底座 | 数周 |
| Wave 3 核心业务 | 数月 |
| Wave 4 完整闭环 | 数月 |
| Wave 5 增值模块 | 数月 |

时间不含合规审查、硬件采购联调、监管资质对接等非编码工作。
**不接受"压缩 TDD 节奏"换时间**——本系统错一条数据可能违法，速度必须服从正确性。

---

## 外部依赖追踪

| 外部依赖 | 关联 Wave | 启动时机 | 当前状态 |
|---------|----------|---------|---------|
| ~~药监局接口资质申请~~ | ~~M11~~（v7 移除：由 ERP 负责） | — | 不需要 |
| "码上放心"账号开通 | M-TC（Wave 4） | Wave 2 启动时 | 未启动 |
| "码上放心"正式接口文档 / 鉴权方式 / 错误码 / 频率限制确认 | M-TC（Wave 4） | Wave 2 启动时 | 未确认 |
| 外部冷链监控系统对接（采集/超标判定由外部）| M5（Wave 4） | Wave 3 启动时确认 SOW | 未启动 |
| 蓝牙打印机 / 电子秤 | M-PK（Wave 5）| Wave 4 启动时 | 未启动 |
| 车辆 GPS / 电子地图 API | M10（Wave 5）| Wave 4 启动时 | 未启动 |
| 法规变更跟踪（GSP 修订） | 所有 Wave | 持续 | — |

## v25 后续波次 backlog（特殊药品落地 — 业务方确认承运但本期不实施）

| 项 | 关联 Wave | 启动条件 | 说明 |
|----|---------|---------|------|
| 放射性药品 30 年保留分区实施 | Wave 4+ | 业务方启动放射性药品承运业务前 | H10 分级保留矩阵已设计完成（docs/infra/technical-specs.md），实施需财务 + 运维联合成本评估，建议走 ADR 决策 |
| 放射性 / 血液制品 / 疫苗运营级双人矩阵默认值生效 | Wave 4+ | 业务方启动对应分类承运前 | M1-010 字典 + M-VR 双人策略矩阵设计完成，实施仅需启用预置规则 + 仓库主管确认 |
| 地方法规 ≥ 10 年保留按运营省份细化 | Wave 4+ | 业务方确定运营省份 + 法规专家核对地方法规 | 默认 5 年；按地方法规可单独配置（H10 分级保留矩阵已支持）|

## v26 GSP 字段命名规范化 backlog（v25 审计发现）

> 触发：v25 D 方案治理脚本 `check_gsp_field_traceability.py` 标记 12 项字段当前为 `acceptable_alias`（即多 alias 并存被允许）。如代码层面要求严格统一，可在 v26 启动批量规范化。
> 影响范围：约 30 个故事文件的字段表 / 正文术语调整。
> 详细清单见 `docs/compliance/gsp-field-traceability.md` §5。

| # | canonical | 应规范化原因 | 影响范围 | 工作量 |
|---|----------|-----------|--------|------|
| 1 | 反查链路 | "反查" / "反向追溯" 仅同义词 | M-TC / M6 / H2 | 30 分钟 |
| 2 | 有效期 | "效期"是简写，复合字段名（"批号/效期"）应展开 | 全模块 | 1 小时 |
| 3 | 生产厂家 | "厂家"是简写 | M1 / M2 / M6 | 30 分钟 |
| 4 | 实际到货数量 | "实到数量"是简写 | M2 / M-SA | 30 分钟 |
| 5 | 验收员 user_id | "签字人" / "收货员（验收岗）" 角色映射混乱 | M2 / M6 / 合规 | 1 小时 |
| 6 | 验收结论 | "质量状态"是同义词 | M2 / M6 | 30 分钟 |
| 7 | 订单状态 | "ASN 状态"应通过状态机命名空间区分 | M2 / M4 | 1 小时 |
| 8 | 销毁原因码 | "销毁原因"是简写 | M-SA / M-QL | 30 分钟 |
| 9 | 温湿度记录 | "温度记录"是子集 | M3 / M5 / 合规 | 30 分钟 |
| 10 | 上报记录 | "上报状态"应分离为字段（status）和数据（record） | M-TC | 30 分钟 |
| 11 | 近效期预警 | "近效期"作状态标签使用，命名分离 | M3 / M6 | 30 分钟 |
| 12 | 变动数量 | "库存数量" 在流水账中应统一 | M3 / M6 | 30 分钟 |

合计预估 ~7 小时。**启动条件**：v26 代码实现阶段需严格 schema 命名一致性时启动；本期保留 acceptable_alias 状态。

## 国际化（i18n）backlog（明示推迟到 v25+ / FDA 客户场景）

> 触发：软件设计审计 §4.12 维度 12 子项 a 标记 i18n 完全空白。
> 决策：**当前不做 i18n**。理由如下，本节即为"显式推迟决策"，避免后续被反复问。

| 维度 | 当前状态 | 推迟理由 |
|------|---------|---------|
| 业务语境 | 100% 中国 GSP | 法规/客户/仓库主管/PDA 操作员全中文场景；当前国际化无 ROI |
| 错误码消息 | ADR-0010 已预留 `message_zh + message_en` 双语字段 | 字段已就位，未来扩展英文成本低（仅填 en 列）|
| 字段词典 | 仅中文 canonical | GSP 字段不存在国际监管合规需求，强行翻译可能反而失真 |
| 前端 locale | 仅 zh-CN | i18next / react-intl 引入需额外打包成本 + 翻译流水线 |
| 日志 / 审计 | 仅中文 | GSP 监管检查现场必须中文 |

**启动条件（任一满足才启动 i18n ADR）**：

1. 业务方明确接 FDA / PIC / S / EU GMP 客户（监管要求英文台账）
2. 业务方接东南亚 / 拉美客户（本地法规要求当地语言）
3. WMS 作为 SaaS 出海

**预估工作量（启动后）**：约 4-6 周
- ADR-0017（暂用号位）：i18n 策略 + 翻译流水线 + locale fallback + 时区/日历/数字格式
- 前端 i18next + 词条抽取
- 后端错误码 message_en 全量补齐
- 字段词典英文映射（仅必要字段，GSP 中文字段保留 canonical）
- PDA 端 i18n + RTL 兼容（如阿拉伯语客户）

**反向决策点**：本 ROADMAP 编辑此 backlog 段视为"业务方未启动 i18n 的证据"。任何"现在能不能加点英文"的请求都应回到此处看启动条件。

---

## 节奏铁律（不可违反）

1. 跨 Wave 不允许私自并行（低 Wave 未完成 → 高 Wave 不启动）
2. 完成判据是"生产可用"，不是"代码写完"
3. 每波完成必做 retro（写在 `docs/retros/wave-N-retro.md`）
4. 范围调整必须新建 ADR
5. 每波都必须可上线
6. TDD 在所有 Wave 强制（按 ADR-0006）
7. schema 变更串行
8. 每波 worktree 上限 3 个（含 main）
9. 涉及一线高频操作的 UI 页面，进入实现 Wave 前必须有高保真原型 + ≥1 次业务方走查 approved（ADR-0021）

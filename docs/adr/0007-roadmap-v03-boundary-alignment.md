# ADR-0007: 波次路线 v0.3 边界对齐

- 状态：Accepted
- 日期：2026-05-16
- 决策者：项目发起人
- 关联：ADR-0004、`docs/architecture-dependencies.md`、`ROADMAP.md`、`docs/domain/clarifications.md`

## 背景

ADR-0004 v0.2 建立了依赖驱动的 Wave 0-5 路线，但后续澄清已经改变了监管边界和模块口径：

1. M11 监管 EDI 整体移除，药监 EDI 由 ERP 负责。
2. "码上放心"上报迁移到 M-TC 追溯码模块。
3. H 层从 H1-H3 扩展为 H1-H10，其中 H6-H10 以技术规格形式管理。
4. M-PK 包装站、M-TE 任务引擎、M-RP 补货等横向业务能力已经进入用户故事索引。

如果继续让 ADR-0004 作为当前路线依据，会和 `docs/domain/clarifications.md`、`ROADMAP.md`、`docs/infra/technical-specs.md` 产生冲突。

## 候选方案

1. 直接改写 ADR-0004：简单，但违反 Accepted ADR 不重写历史的规则。
2. 保留 ADR-0004，仅在各文档零散注明例外：改动小，但读者仍会把旧路线当作当前决策。
3. 新增 ADR-0007 取代 ADR-0004：符合 ADR 不可变原则，当前路线有单一入口。

## 决策

采纳方案 3。ADR-0004 标记为 `Superseded by ADR-0007`，当前路线以本文和 `docs/architecture-dependencies.md` 为准。

当前边界：

1. M11 不再作为待实现业务模块；保留 `user-stories-m11-regulatory-edi.md` 作为移除决策占位。
2. M-TC 承接 "码上放心" 上报，外部依赖为 "码上放心" 账号开通。
3. 药监 EDI 不由 WMS 直连；WMS 通过 H8 ERP 防腐层向 ERP 反馈数据。
4. Wave 1 只启动 H1 权限、多租户，H2 审计追踪，H3 OpenAPI 契约三项横向底座。
5. H4-H10、M-TE/M-RP/M-PK/M-VR/M-QL/M-CG/M-SA/M-RC/M-DI/M-BA 等按依赖进入后续 Wave，不再用"11 业务 + 3 横向"旧口径描述总范围。

## 后果

- 正面：监管边界、模块清单、外部依赖和波次计划重新一致。
- 正面：Accepted ADR 历史保持可追溯，不通过重写 ADR-0004 掩盖路线演进。
- 负面：ROADMAP、架构依赖图、站点首页和 AGENTS 索引需要同步维护。
- 风险：模块数量继续演进时容易再次出现口径漂移；需用脚本化检查覆盖故事数量、锚点链接和路线关键词。

## 实施约束

1. `docs/architecture-dependencies.md` 是当前模块依赖唯一真相源。
2. `ROADMAP.md` 只写用户视角摘要，不再保留已移除 M11 的待办项。
3. `docs/domain/clarifications.md` 继续记录业务边界决策，不承载完整路线。
4. 新增或移除业务模块、横向能力、基础设施模块时，必须同步更新本文、依赖图、ROADMAP、AGENTS 索引和 MkDocs 导航。

## 参考

- `docs/domain/clarifications.md`：监管接口边界、M-PK、M-TE、M-RP 等决策记录
- `docs/infra/technical-specs.md`：H6-H10 技术规格
- `docs/domain/user-stories-m11-regulatory-edi.md`：M11 移除占位

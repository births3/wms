# Changelog

本文件记录 wms 项目的版本变更，遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)
和 [Conventional Commits](https://www.conventionalcommits.org/)。

后续版本由 [git-cliff](https://github.com/orhun/git-cliff) 从 Conventional Commits 自动生成。

## [Unreleased]

### Added
- Wave 0 治理骨架：仓库结构、Git 配置、治理总文档、6 份 ADR（0001-0004 + 0006-0007）、依赖图、justfile、lefthook、14 个治理脚本、gate-rules、baseline 占位、README/ROADMAP/TODO。
- 概念审计独立文档（docs/concept-audit.md）：8 镜头扫描结果 + 数据量评估（3 年 164M 行 / 30GB / PG 单机足够）。
- 业务澄清记录（docs/domain/clarifications.md）：47 项业务决策含 v3.1 #47（C1 PDA 离线 / C6 盘点期间出库 / C7 退货批号 ERP 校验）。
- 字段词典 v3.1（docs/compliance/gsp-field-traceability.md）：163 个字段（80 GSP + 40 business + 8 system + 15 config + 10 derived + 10 interface），新增 10 个 P0 GSP canonical（country_of_origin / USCC×6 / shipment_doc_no / operation_type / delivery_time）+ 31 个 legacy 英文 alias 补充。
- 用户故事文档（docs/domain/user-stories-*.md）：34 个文件（154 个故事，含 Wave 0 不实施模块的设计） + 故事文件按主题拆分（m4 → 3 / m3 → 2 / m1 → 2 / m2 → 2）。
- AGENTS.md：风险分级标准（🟢 无风险自主修复 / 🔴 有风险必确认）+ Git 操作引导（红线 + 标准动作）。
- 治理脚本扩充：check_gsp_field_traceability / check_user_story_structure / check_glossary_consistency / check_approval_source_chain / check_config_center_consistency / check_pda_story_completeness / check_baseline_health / check_governance_consistency / check_story_size 等 14 个脚本，T1 层全过。
- governance.md §3 提交规范扩充：§3.2 描述/正文/脚注规则、§3.4 commit 粒度（vs PR）、§3.7 安全敏感信息红线清单与误提交后处理。
- mkdocs 文档站点（site/）+ 业务故事/ADR/合规/治理多视角导航。

### Changed
- ADR-0004 已被 ADR-0007 取代（v0.3 路线边界对齐：5 波次明确化）。
- 所有 27 个故事文件加"测试要求"声明（写操作 L4+L5+L8+L11，读操作 L4+L8）。
- M-QL 跨故事约束 §8 加"审批集中入口"（解决 K15 审批散落 16 文件 32 次）。
- M3-003 跨约束 §9 加"系统触发隔离免二次审批"（K14）。
- M-VR 跨约束 §9 加"校验异常"同名跨模块说明（K13）。
- M-VR-001 §3 出库校验字段加批号校验语义澄清（K11）。
- H5-003 §7 通用打印失败策略 + M2/M4/M-PK 故事引用（C4）。
- H1 跨约束 §7 PDA 离线统一策略（默认 24 小时可配置，C1）。
- M3-005 §6 盘点期间已下发任务继续执行规则（C6）。
- M4-008 §4 退货批号策略（ERP 给批号 + WMS 校验已有 + 异常走 M-QL，C7）。
- 字段词典 §3 字段类别索引：73 → 83；§4.1 性质类别分布：153 → 163；§6.1 总计 + GSP 强制数同步。

### Removed
- M11 监管 EDI 模块整体移除（v7 边界重定）：M11-001 迁到 M-TC-007 / M11-002+003 由 ERP 负责。
- M3-007 跨仓调拨 / M3-008 货主转换（v23 移除：GSP 不合规）。

---

> 当前项目处于 Wave 0 治理骨架阶段，尚无业务功能版本发布。
> 第一个标签版本将在 Wave 1（横向底座）完成时发布为 `v0.1.0-foundation`。

# wms ROADMAP

> 长期路线（用户视角简版）。详细决策见 [ADR-0004](docs/adr/0004-phase-roadmap.md)。
> 模块依赖与并行规则见 [docs/architecture-dependencies.md](docs/architecture-dependencies.md)。

---

## 总目标

完整实现医药冷链 GSP 合规 WMS，11 个业务模块 + 3 个横向能力**全部生产化交付**。

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
- [x] ADR-0004 v0.2 波次路线
- [x] ADR-0006 TDD + 11 层测试
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

## Wave 1：横向底座（H 层）

**周期**：4-6 周（个人）
**目标**：所有业务模块的基础设施就绪。

并行任务：

- W1.A：H1 权限与多租户基础（角色 / 权限码 / 货主隔离 / JWT）
- W1.B：H2 审计追踪基础设施（append-only / 旧值新值 / 操作人时间 IP）
- W1.C：H3 OpenAPI 契约工具链（utoipa 注解 / openapi.json 生成 / openapi-typescript 消费）

**外部资质并行启动**：
- 药监局接口资质申请
- "码上放心"账号开通

**完成标准**：任意业务 handler 可挂 H1；任意写操作经 H2；后端注解可生成 OpenAPI，前端 `@wms/api-client` 可消费。

---

## Wave 2：业务底座 + Schema 先行

**周期**：3-4 周
**目标**：核心 schema 落地，基础 CRUD 可用。

并行任务：

- W2.A：M1.a 基础档案 schema + 基础 CRUD（商品 / 供应商 / 客户 / 仓库 / 库位）
- W2.B：M2 入库 schema 设计（不写业务规则）
- W2.C：M6 报表查询接口骨架

**完成标准**：核心 schema 落地；商品 / 供应商 / 收货单基础 CRUD 可用；OpenAPI 反映完整 schema。

---

## Wave 3：核心业务规则铺开

**周期**：8-10 周
**目标**：单货主下"商品-供应商-入库-库存"闭环。

并行任务：

- W3.A：M2 入库业务规则 + handler + PDA 端
- W3.B：M3 库存模型 + 业务规则（FIFO / 近效期 / 库存类型）
- W3.C：M5 冷链 schema 设计
- W3.D：M9 3PL 计费"账户/合同"模型

**完成标准**：M2 / M3 关键路径 11 层测试覆盖；GSP 资质有效期校验生效。

---

## Wave 4：完整闭环 + 横向叠加

**周期**：8-10 周
**目标**：单货主下完整业务闭环可上线试运行。

并行任务：

- W4.A：M4 出库（订单 / 拣选 / 复核 / 打印随货同行单）
- W4.B：M5 冷链业务规则（温湿度采集 / 超标预警 / 冷链台账）
- W4.C：M6 报表实现（GSP 法定台账）
- W4.D：M11 监管 EDI 适配层骨架（**外部资质并行推进**）

**完成标准**：完整业务闭环（采购入库 → 库存 → 销售出库 → 冷链监控 → 报表）可上线；GSP 法定台账可生成；审计追踪 append-only 不变量验证通过。

---

## Wave 5：增值模块全面铺开

**周期**：12-16 周
**目标**：所有 11 个业务模块生产可用。

并行任务（最多 3 个 worktree 同时）：

- W5.A：M7 零拣包装站（Put-to-Light / 保温箱配置 / 电子秤复核 / 面单打印）
- W5.B：M8 连锁专有（门店经营范围 / 自动补货 / 越库 / O2O）
- W5.C：M9 3PL 计费业务规则（仓储费 / 作业费 / 月结账单）
- W5.D：M10 TMS+（路径优化 / 在途温控 / 周转箱回收）
- W5.E：M11 监管 EDI 业务对接（依赖外部资质就位）

**完成标准**：多货主隔离生效；监管平台对接通过测试环境验证；至少一个连锁客户场景跑通。

---

## 总周期估计（极保守）

| 阶段 | 个人节奏 | 小团队（2-3 人） |
|------|---------|------------------|
| Wave 0 治理骨架 | 1 周 | 1 周 |
| Wave 1 横向底座 | 4-6 周 | 2-3 周 |
| Wave 2 业务底座 | 3-4 周 | 1.5-2 周 |
| Wave 3 核心业务 | 8-10 周 | 4-5 周 |
| Wave 4 完整闭环 | 8-10 周 | 4-5 周 |
| Wave 5 增值模块 | 12-16 周 | 6-8 周 |
| **总计** | **≈10 个月** | **≈5 个月** |

时间不含合规审查、硬件采购联调、监管资质对接等非编码工作。

---

## 外部依赖追踪

| 外部依赖 | 关联 Wave | 启动时机 | 当前状态 |
|---------|----------|---------|---------|
| 药监局接口资质申请 | M11（Wave 4-5）| Wave 1 启动时 | 未启动 |
| "码上放心"账号开通 | M11 | Wave 1 启动时 | 未启动 |
| 冷链温湿度探头 / 网关采购 | M5（Wave 4）| Wave 3 启动时确认 SOW | 未启动 |
| 蓝牙打印机 / 电子秤 | M7（Wave 5）| Wave 4 启动时 | 未启动 |
| 车辆 GPS / 电子地图 API | M10（Wave 5）| Wave 4 启动时 | 未启动 |
| 法规变更跟踪（GSP 修订） | 所有 Wave | 持续 | — |

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

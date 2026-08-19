# ADR-0004: 波次路线（依赖驱动并行 + TDD 节奏）

- 状态：Superseded by ADR-0007
- 日期：2026-05-15
- 决策者：项目发起人
- 关联：`docs/governance.md`、`docs/architecture-dependencies.md`、ADR-0001、ADR-0002、ADR-0003、ADR-0006

> v0.1 的"三阶段"措辞容易被误解为"砍功能 / 半成品 demo"。
> v0.2 改为**波次驱动**：所有 11 个业务模块 + 3 个横向能力**全部生产化交付，按依赖图分波次推进**，每波内可 worktree 并行。
> v0.3 路线已由 ADR-0007 取代：M11 移除，码上放心归 M-TC，药监 EDI 由 ERP/H8 边界承接。

---

## 背景

wms 完整功能图谱有 11 业务模块 + 3 横向能力（参见 `docs/architecture-dependencies.md`），覆盖医药 GSP 全流程。"全部生产化交付"是确定目标，不可妥协。

但同时面对客观规律：

- **依赖锁死**：库存依赖入库 schema、出库依赖库存可用量、计费依赖出库流水……
- **合规模型未稳定**：商品/供应商/批次的字段一旦定下，所有模块都用它
- **个人精力上限**：同时持有 11 个模块的设计上下文会认知崩溃
- **GSP 反馈延迟**：没有可演示版本 → 拿不到合规、监管、真实仓库的反馈
- **正确性优先**：错一条数据可能违法，速度必须服从正确性（详见 ADR-0006）

**核心问题**：如何在"全部模块都做"和"客观依赖 / 精力 / 反馈"之间取得平衡？

---

## 候选方案

### 方案 A：模块顺序串行（垂直切片）

逐模块完整实现：基础档案做完美 → 入库 → 库存 → ……

| 优点 | 缺点 |
|------|------|
| 单模块聚焦深度 | 没有完整业务闭环演示；上下游无法联调；后期模块发现底座设计错时返工巨大 |

### 方案 B：完全并行铺开

11 个模块同时开干。

| 优点 | 缺点 |
|------|------|
| 表面"快" | 依赖错位 → 反复返工；测试组合爆炸；个人认知崩溃；最终所有模块都半成品 |

### 方案 C（采纳）：依赖驱动的波次并行

按 `docs/architecture-dependencies.md` 的依赖图，把所有模块分为 5 个波次（Wave 1-5）+ 1 个治理波次（Wave 0）。

- **波次内**：依赖松的任务用 git worktree 真并行
- **波次间**：严格按依赖顺序，高 Wave 不允许在低 Wave 完成前启动
- **每波都生产可用**：完成时是"完整子集可上线"，不是"半成品 demo"
- **TDD 节奏贯穿所有波次**：所有业务行为 outside-in TDD（详见 ADR-0006）

| 优点 | 缺点 |
|------|------|
| 依赖正向、并行最大化、永远有可演示版本、风险逐步释放 | 需严守纪律避免跨波私自并行；需治理脚本约束 |

---

## 决策

采纳**方案 C**。下文为完整规则。

---

## 1. 波次划分（与依赖图严格一致）

详细依赖与可并行任务清单见 `docs/architecture-dependencies.md` §5。本节仅给出概要。

### Wave 0：治理骨架（前置，本周）

不属于业务模块，但所有代码工作的前置条件。

包含：项目结构、Git 配置、ADR 全集、justfile / lefthook / 治理脚本骨架、依赖图固化。

**完成标准**：本 ADR 编号范围内的 ADR 全部 Accepted；`just quick-check` 可运行。

### Wave 1：横向底座（H 层）

| 任务 | 内容 | 可并行 worktree |
|-----|------|----------------|
| W1.A | H1 权限/多租户基础 | ✅ |
| W1.B | H2 审计追踪基础设施 | ✅ |
| W1.C | H3 OpenAPI 契约工具链（utoipa + gen-api） | ✅ |

**完成标准**：
- 任意业务 handler 可挂上 H1 鉴权
- 任意写操作可经 H2 审计层（append-only）
- 后端 utoipa 注解可生成 `shared/openapi/openapi.json`，前端 `@wms/api-client` 可被消费

### Wave 2：业务底座 + Schema 先行

| 任务 | 内容 | 依赖 |
|-----|------|------|
| W2.A | M1.a 基础档案 schema + 基础 CRUD（商品、供应商、客户、仓库、库位） | H1, H3 |
| W2.B | M2 入库 schema 设计（不写业务规则） | H1, H2 |
| W2.C | M6 报表查询接口骨架 | H2 |

**完成标准**：核心 schema 落地；商品 / 供应商 / 收货单基础 CRUD 可用；OpenAPI 反映完整 schema。

### Wave 3：核心业务规则铺开

| 任务 | 内容 | 依赖 |
|-----|------|------|
| W3.A | M2 入库业务规则 + handler + PDA 端 | W2.A, W2.B |
| W3.B | M3 库存模型 + 业务规则（FIFO / 近效期 / 库存状态） | W2.A, W2.B |
| W3.C | M5 冷链 schema 设计 | 独立 |
| W3.D | M9 3PL 计费"账户/合同"模型 | H1（独立）|

**完成标准**：单货主下，"商品-供应商-入库-库存"完整业务可跑通；M2 / M3 关键路径 11 层测试覆盖。

### Wave 4：完整闭环 + 横向叠加

| 任务 | 内容 | 依赖 |
|-----|------|------|
| W4.A | M4 出库（订单/拣选/复核/打印） | W3.B |
| W4.B | M5 冷链业务规则（温湿度采集 / 超标预警 / 冷链台账） | W3.B, W3.C |
| W4.C | M6 报表实现（消费各业务流水生成法定台账） | W3.A, W3.B, W4.A |
| W4.D | M11 监管 EDI 适配层骨架（**外部资质并行推进，不阻塞代码**） | W3.A, W3.B |

**完成标准**：单货主下，"采购入库 → 库存 → 销售出库 → 冷链监控 → 报表"完整闭环可上线试运行。GSP 法定台账可生成。审计追踪 append-only 不变量验证通过。

### Wave 5：增值模块全面铺开

| 任务 | 内容 | 依赖 |
|-----|------|------|
| W5.A | M7 零拣包装站 | W3.B, W4.A, W4.B |
| W5.B | M8 连锁专有 | W2.A, W4.A |
| W5.C | M9 3PL 计费业务规则 | W3.A, W3.B, W4.A, W3.D |
| W5.D | M10 TMS+ | W4.A, W4.B |
| W5.E | M11 监管 EDI 业务对接（**依赖外部资质就位**） | W3.A, W3.B, W4.A, W4.B |

**完成标准**：所有 11 个业务模块生产可用；多货主隔离生效；监管平台对接通过测试环境验证；至少一个连锁客户场景跑通。

---

## 2. 波次硬规则（不可违反）

1. **跨波不允许私自并行**：低 Wave 未完成时，高 Wave 不得启动
2. **波次完成判据是"生产可用"**：不是"代码写完"，是"能跑、有测试网、可上线"
3. **每波完成必须做 retro**：写一份 `docs/retros/wave-N-retro.md`，识别需要新增/退役的 ADR、依赖图是否需要更新
4. **波次范围调整必须新建 ADR**：不允许默默扩缩
5. **每波都必须可上线**（哪怕只对开发者环境）
6. **TDD 在所有 Wave 强制**（按 ADR-0006）
7. **schema 变更串行**：同一表的 schema 不允许多个 worktree 并行修改
8. **每波 worktree 上限 = 3**（含 main）

---

## 3. 波次内的 worktree 并行策略

### 3.1 命名约定

```
~/workspace/wms                # main 主工作目录
~/workspace/wms-w1a-auth       # Wave 1 worktree A: H1 权限
~/workspace/wms-w1b-audit      # Wave 1 worktree B: H2 审计
~/workspace/wms-w1c-contract   # Wave 1 worktree C: H3 契约
~/workspace/wms-w3b-inv        # Wave 3 worktree B: M3 库存
```

格式：`wms-w<N><letter>-<short-scope>`

### 3.2 并行选择规则（依据依赖图）

只允许下表标记为 ✅ 的并行组合：

- **可并行**：依赖独立、修改不同 schema 或不同表、不在同一关键路径
- **必须串行**：共享 schema、强依赖、同一表的并发修改

具体并行清单见 `docs/architecture-dependencies.md` §5 各 Wave 子表。

### 3.3 worktree 工作流（占位，详见未来 ADR-0007）

第 0 周仅声明意图，具体规则（target/ 共享、node_modules 处理、baseline 协调、跨 worktree 推送顺序）由后续 ADR-0007 落定。在 ADR-0007 出来前的临时规则：

- 每个 worktree 独立 `target/`（暂不优化磁盘）
- 同一时刻只在一个 worktree 跑 `just preflight`（避免 testcontainer 端口冲突）
- baseline 修改必须 rebase 到 main 后再合并

---

## 4. TDD 节奏在波次中的体现

每个业务任务（Wave 2 起）的标准节奏：

```
1. 外层红：写一个 L3 业务流程测试 → 失败
2. 内层 TDD 循环：L1 红 → 绿 → 重构（多次）
3. 外层验证：L3 重新跑 → 应通过
4. 维度补充：按 ADR-0006 §2.3 必备维度判定，补 L2/L4/L5/L8/L10/L11
5. Tier 验证：本地跑 just task-check（T2）→ just preflight（T3）
6. 提交 PR + 自查清单
7. CI 跑 T4 verify
8. Squash merge 到 main
```

每个 Wave 任务（W*.A / W*.B 等）由若干这样的循环构成。完整 W3.A（M2 入库业务规则）预计含 30-50 个红绿循环。

### 4.1 节奏要求

- 每个红绿循环 ≤ 1 小时（超时即拆分）
- 每个 Wave 任务 ≤ 4 周（个人节奏）
- 每个 Wave 整体 ≤ 8 周（个人节奏，含 retro）

### 4.2 测试维度的渐进引入

按 ADR-0006 §4.1，11 层治理脚本随 Wave 落地：

| Wave | 引入的测试治理脚本 |
|------|------------------|
| Wave 0 | （无 11 层脚本）|
| Wave 1 | （无；本身就是基础设施）|
| Wave 2 | check_handler_test_coverage、check_test_org_layout |
| Wave 3 | check_error_path_coverage、check_idempotency_test、check_permission_test_matrix、check_data_consistency_test、check_api_compat |
| Wave 4 | check_observability_test、check_concurrency_test、check_perf_baseline |
| Wave 5 | （脚本已就绪，仅扩展规则集）|

---

## 5. 时间预估（极保守，仅参考，非承诺）

| Wave | 个人节奏 | 小团队（2-3 人）节奏 |
|------|---------|---------------------|
| Wave 0 治理骨架 | 1 周 | 1 周 |
| Wave 1 横向底座 | 4-6 周 | 2-3 周 |
| Wave 2 业务底座 + schema | 3-4 周 | 1.5-2 周 |
| Wave 3 核心业务规则 | 8-10 周 | 4-5 周 |
| Wave 4 完整闭环 + 横向 | 8-10 周 | 4-5 周 |
| Wave 5 增值模块 | 12-16 周 | 6-8 周 |
| **总计** | **36-46 周（≈10 个月）** | **18-23 周（≈5 个月）** |

说明：

- 时间含 TDD 节奏（业务行为先测后码），不含合规审查、硬件采购联调、监管资质对接等非编码工作
- 实际节奏取决于 GSP 现场反馈速度与硬件就位速度
- **不接受"压缩 TDD 节奏"换时间**——本系统错一条数据可能违法，速度必须服从正确性（ADR-0006）

---

## 6. 与治理体系的对应

| Wave | 治理脚本数量目标 | 重点 |
|------|------------------|------|
| Wave 0 | ~10 | 文档 / 流程 / 运行骨架（含 ADR 索引、提交规范、环境检查）|
| Wave 1 | ~15 | 加入代码治理（layer dependency、unwrap、auth coverage、audit coverage）|
| Wave 2 | ~22 | 加入测试治理（handler test coverage、test org layout）+ schema 治理 |
| Wave 3 | ~32 | 加入 11 层质量治理（L4/L5/L8/L9/L11） + GSP 规则追溯 |
| Wave 4 | ~38 | 加入运行治理（perf baseline、observability、cold-chain freshness）|
| Wave 5 | ~42 | 合规专项（audit log integrity、监管 EDI 契约、计费幂等）|

每 Wave 进入前更新 `governance/gate-rules.toml` 与对应 baseline。

---

## 7. 与外部依赖的协调

某些工作不在 git 工作流内但影响 Wave 进度，必须在 Wave 启动时显式登记到 TODO.md：

| 外部依赖 | 关联 Wave | 启动时机 |
|---------|----------|---------|
| 药监局接口资质申请 | M11（Wave 4-5）| **Wave 1 启动时同步启动** |
| "码上放心"账号开通 | M11 | Wave 1 启动时同步启动 |
| 冷链温湿度探头 / 网关采购 | M5（Wave 4）| Wave 3 启动时确认 SOW |
| 蓝牙打印机 / 电子秤 | M7（Wave 5）| Wave 4 启动时确认 |
| 车辆 GPS / 电子地图 API | M10（Wave 5）| Wave 4 启动时确认 |
| 法规变更跟踪（GSP 修订） | 所有 Wave | 持续 |

外部依赖在 ROADMAP.md 单设一节追踪。

---

## 8. 后果

### 正面

- **目标明确**：11 个模块全部生产化交付，无"砍功能"歧义
- **依赖正向**：底座先稳，增值模块依赖稳定底座，不推倒重来
- **永远有可演示版本**：每 Wave 完成都是"生产可用子集"
- **TDD + 11 层兜底**：每 Wave 完成时正确性已被自动化验证
- **波次内可真并行**：worktree + 依赖图让多任务可并行而不混乱
- **风险逐步释放**：核心闭环（W4 完成）稳定后再叠加增值模块

### 负面

- **总周期长**：完整路线 10 个月（个人）/ 5 个月（小团队）
- **TDD 节奏短期慢**：写测试占用时间（但长期省返工时间）
- **波次纪律要求高**：跨波私自并行的诱惑必须抵抗
- **多次"碰"同一模块**：M1 在 W2 / W4 / W5 都会扩展，需保持向后兼容

### 风险

- **跨波蠕变**：Wave 1 时手痒做 Wave 3 的事 → 失控；缓解：硬规则 1 + retro 必查
- **底座设计错误**：H1/H2/H3 错了影响所有 Wave；缓解：Wave 1 完成必做架构评审
- **外部依赖延期**：M11 资质 / 冷链硬件不到位；缓解：第 7 节显式追踪、提前启动
- **TDD 走形**：写测试只为应付治理脚本而非驱动设计；缓解：retro 抽查、PR 自查清单
- **个人精力崩溃**：长周期项目最大风险；缓解：worktree 并行控制在 3 内、每 Wave 必须 retro 自评

---

## 9. 实施约束

- **当前 Wave**：Wave 0 进行中，完成本周后进入 Wave 1 准备期
- **进入 Wave 1 前**必须完成：所有 Wave 0 ADR + `docs/architecture-dependencies.md` + 治理骨架（justfile / lefthook / 8 个起步脚本）
- **进入 Wave 2 前**必须完成：H1/H2/H3 + 架构评审 retro + Wave 1 治理脚本
- **每 Wave 关闭必须产出**：retro 文档 + ADR 索引更新 + baseline 复盘 + ROADMAP 状态更新
- **波次评审在 ADR/Mxx-wave-N-retro.md 中记录**

---

## 10. 与 v0.1 的差异（变更说明）

| 项 | v0.1（已废弃） | v0.2（本版） |
|----|--------------|--------------|
| 措辞 | "MVP / 阶段 1/2/3" | "Wave 0/1/2/3/4/5" |
| 立场 | 暗含"砍功能 / 留待后期" | "全部生产化交付，按依赖排序" |
| 并行 | 不强调 | 显式 worktree 并行 + 依赖图驱动 |
| TDD | 未明确 | 强制 outside-in（引用 ADR-0006）|
| 测试 | 未明确 | 11 层 + 4 Tier（引用 ADR-0006）|
| 阶段数 | 3 | 6（含 Wave 0）|
| 估时 | 24 个月 | 10 个月（含 TDD 节奏，更现实）|

v0.1 文档不删除，标记为 Superseded by 本版。

---

## 11. 参考

- 治理总文档：`docs/governance.md`
- 架构依赖图：`docs/architecture-dependencies.md`
- 长期路线（用户视角简版）：`ROADMAP.md`
- 技术栈决策：`docs/adr/0001-tech-stack.md`
- 仓库结构决策：`docs/adr/0002-monorepo-structure.md`
- 治理模型决策：`docs/adr/0003-governance-model.md`
- TDD 与 11 层测试：`docs/adr/0006-tdd-and-test-layers.md`

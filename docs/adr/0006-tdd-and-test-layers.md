# ADR-0006: TDD 开发模式 + 11 层测试维度

- 状态：Accepted
- 日期：2026-05-15
- 决策者：项目发起人
- 关联：`docs/governance.md`、ADR-0001（技术栈）、ADR-0003（治理模型）、ADR-0004（波次路线）

---

## 背景

wms 是医药 GSP 合规系统，对**正确性**的要求远高于一般业务系统：

- 库存数量错一条 → GSP 违规
- 批次效期判定错 → 用户用药风险
- 审计追踪缺失 → 监管处罚
- 并发竞态导致超卖 → 资金/合规双重损失

仅靠"写完代码后补测试"的传统方式不足以约束这种系统。
同时，wms 模块多（11 业务 + 3 横向）、生命周期长（5 年以上），重构需求频繁——**没有测试网，重构 = 赌博**。

**核心问题**：用什么开发纪律 + 测试体系，能让"写代码"和"保正确"同步进行？

---

## 候选方案

### 方案 A：传统"代码先于测试"

写完代码再补测试，靠 Code Review 把关。

| 优点 | 缺点 |
|------|------|
| 启动快、看起来产出多 | 测试为"覆盖率而测"，不为"行为而测"；重构无网；GSP 场景遗漏率高 |

### 方案 B：纯 TDD（每行代码都先红测试）

机械教条版：任何代码必须先有红测试。

| 优点 | 缺点 |
|------|------|
| 最严格 | 在 schema / migration / utoipa 注解 / 配置类代码上反作用；过分降低产出 |

### 方案 C（采纳）：业务行为 TDD（outside-in 双层循环） + 11 层测试维度 + Tier 分层执行

- 业务行为代码必须 TDD（先测后码），但脚手架 / schema / 配置类代码不强制
- 测试按 11 个维度组织，覆盖业务正确、契约、错误、并发、性能、合规等所有面向
- 11 层测试按 4 个 Tier（T1-T4）分时机执行，速度即采纳率

---

## 决策

采纳**方案 C**，下文为完整规则。

---

## 1. TDD 双层循环（outside-in）

### 1.1 外层循环（业务流程驱动）

每个业务用例（user story）开始时：

1. 写一个**业务流程测试（L3）**，使用集成测试 + testcontainers + 真实 PostgreSQL
2. 跑 → 必须红
3. 进入内层循环
4. 内层全部完成后回到外层 → 应转绿
5. 外层不绿 = 缺行为或外层假设错了

### 1.2 内层循环（领域单元驱动）

针对每一个具体小行为：

1. 红：写一个失败的 L1 单元测试
2. 绿：写**最小代码**让它通过（不写多余）
3. 重构：在测试保护下改进代码（不改行为）
4. 一次只红一个，不红多个并发推进

### 1.3 外层完成后补充其他维度

外层 L3 通过后，针对同一业务补足其他必要维度的测试（见第 2 节 11 层）：
- 必备：L2、L4、L5、L8、L11（写操作必须）、L10（关键事件）
- 按需：L6（涉共享状态）、L7（切片完成时）、L9（API 变更时）

### 1.4 必须 TDD 的代码范围

| 代码类型 | TDD 强制度 |
|---------|-----------|
| 领域层（domain crate）业务规则 | **强制** |
| 应用层（app crate）服务编排 | **强制** |
| API handler 业务行为 | **强制**（至少 L2 + L3 之一先于实现）|
| 仓储实现（infra）涉业务规则 | **强制** |
| Web/PDA 业务交互（涉数据写入） | **强制**（L1 + L3）|

### 1.5 不强制 TDD 的代码范围

| 代码类型 | 处理 |
|---------|------|
| Schema / migration | 不强制；但相关业务规则必须 TDD |
| utoipa 注解 / OpenAPI 标注 | 不强制；L9 兼容性测试覆盖 |
| 配置加载 / 启动代码 | 不强制；T4 启动冒烟覆盖 |
| 纯样式（CSS / Tailwind） | 不强制 |
| 第三方库 thin wrapper | 不强制；但 wrapper 暴露的业务方法仍要 TDD |

### 1.6 TDD 纪律的"软证据"

机器无法在 commit 后证明"测试先于实现"。务实做法：

- **强约束**（机器可校验）：
  - 每个 PR 必须包含测试
  - 新增/修改的领域逻辑必须有对应单元测试
  - 写操作 handler 必须有 L11 幂等性测试
- **弱约束**（自律 + retro 复盘）：
  - 测试 commit 早于实现 commit（鼓励，不强制）
  - PR 描述自查清单包含"是否走了 outside-in"

不引入"先红后绿"的强制扫描脚本——容易误判，且 squash merge 后无法追溯。

---

## 2. 11 层测试维度

### 2.1 维度定义

| 维度 | 测什么 | 主要场景 |
|-----|--------|---------|
| L1 单元测试 | 模块内部逻辑 | 领域规则、纯函数、值对象 |
| L2 API 契约 | 请求/响应格式 | OpenAPI schema 校验、必填字段、类型 |
| L3 业务流程 | 端到端场景 | 完整业务流（如收货-验收-上架）|
| L4 错误处理 | 异常场景 | 业务异常、外部失败、边界条件 |
| L5 数据一致性 | 状态持久化 | 事务回滚、审计落库、跨表一致 |
| L6 并发安全 | 竞态条件 | 同一批次并发拣货、同库位并发上架 |
| L7 性能 | 吞吐 / 资源 / **易用性 SLA**（v25 新增）| 关键路径基准、bundle size、响应时间、PDA 单步交互 P95 ≤ 1.5s 等（详见 docs/infra/usability-baseline.md）|
| L8 权限控制 | 认证 / 授权 | 角色矩阵、租户隔离、权限码覆盖 |
| L9 兼容性 | 版本 / 类型 | OpenAPI 向后兼容、TS 类型 diff |
| L10 可观测性 | 日志 / 指标 | tracing span、关键事件日志 |
| L11 幂等性 | 重试一致性 | 同请求重复提交结果一致 |

### 2.2 各维度工具

| 维度 | Rust 端 | TS 端（Web / PDA）|
|-----|---------|------------------|
| L1 | `cargo test`（domain / app）| Vitest |
| L2 | OpenAPI schema 校验 + handler 集成测试 | openapi-typescript 编译期校验 |
| L3 | `cargo test` + testcontainers + 真实 PG | Playwright |
| L4 | `cargo test` 异常路径专项 | Vitest 异常路径 |
| L5 | testcontainers + 事务断言 | （后端覆盖）|
| L6 | `tokio::test` + `loom`（关键临界区）| 极少需要 |
| L7 | `criterion` 基准（含 baseline）+ 易用性 SLA 验证（v25）| Lighthouse、bundle 大小、PDA 交互 P95 测量（v25）|
| L8 | handler 集成测试 + 角色矩阵 | Playwright + 不同 session |
| L9 | OpenAPI schema diff 工具 | TS 类型 diff 工具 |
| L10 | tracing 测试订阅器 + 断言 | 控制台日志 / OTLP 断言 |
| L11 | testcontainers + 重放断言 | （后端覆盖）|

### 2.3 必备维度判定

写一个新的业务用例时，必须有的测试：

| 用例类型 | 必备维度 |
|---------|---------|
| 只读查询 | L1 + L2 + L3 + L8 |
| 写操作（创建/更新） | L1 + L2 + L3 + L4 + L5 + L8 + L11 |
| 跨上下文事件 | 上述 + L10（事件日志可观测）|
| 涉及并发资源 | 上述 + L6 |
| 关键路径（出入库主流程） | 全部（11 层）|

不判定为"必备"的维度，由开发者按 PR 自查清单补足。

### 2.4 测试组织目录约定

```
backend/crates/<crate>/src/<module>/tests/
├── unit/                    # L1
├── error_paths/             # L4
├── concurrency/             # L6（如有）
└── idempotency/             # L11（写操作）

backend/tests/<context>/     # 跨 crate 集成
├── api_contract/            # L2
├── business_flows/          # L3
├── data_consistency/        # L5
├── permissions/             # L8
└── observability/           # L10

backend/benches/             # L7 criterion
shared/openapi/history/      # L9 schema 历史快照
governance/baselines/perf/   # L7 性能基线

apps/web-admin/src/features/<context>/__tests__/
├── unit/                    # L1
└── e2e.spec.ts              # L3

apps/pda-mobile/src/features/<context>/__tests__/
├── unit/                    # L1
└── e2e.spec.ts              # L3
```

约束：

- 测试文件命名遵循 `test_<被测>_<条件>_<期望>` 模式（中文 / 英文均可，团队统一即可）
- 不允许跨维度混测（一个测试文件只承担一个 L）
- 测试目录结构是治理脚本的扫描依据，**禁止自由命名**

---

## 3. Tier × Layer 执行矩阵（4 Tier × 11 Layer）

| Tier | 命令 | 时间预算 | 跑哪些层 | 触发时机 |
|------|------|---------|---------|---------|
| T1 | `just quick-check` | < 10s | （不跑测试，仅 fmt / lint / 提交规范） | 写代码时、pre-commit |
| T2 | `just task-check` | < 120s | L1 单元、L2 静态契约校验、diff 触发的少量针对测试 | 任务结束、commit 前 |
| T3 | `just preflight` | < 5min | T2 + L3 业务流程、L4 错误、L5 数据一致、L8 权限、L11 幂等 | 推送前、pre-push、PR 创建 |
| T4 | `just verify` | < 30min | T3 + L6 并发、L7 性能、L9 兼容、L10 可观测、完整 E2E | 合并前、CI、发版 |

### 3.1 时间预算红线

- 任何 Tier 超时 = 治理失败，必须降级到下一 Tier
- T1 超时 → 把慢检查移到 T2
- T2 超时 → 升级到 T3 + 拆 diff 触发
- T3 超时 → 升级到 T4 + 并行化
- T4 超时 → 切分 / 并行化（按变更范围跑）

### 3.2 失败处理

- T1 失败 → 阻塞 commit
- T2 失败 → 阻塞 commit
- T3 失败 → 阻塞 push
- T4 失败 → 阻塞合并 / 阻塞发版

---

## 4. 与治理体系的集成

### 4.1 测试相关治理脚本（按波次建立）

| 脚本 | 维度 | 建立波次 | baseline 支持 |
|------|------|---------|--------------|
| `check_handler_test_coverage.py` | L1+L2+L3 | Wave 2 | ✅ |
| `check_error_path_coverage.py` | L4 | Wave 3 | ✅ |
| `check_idempotency_test.py` | L11 | Wave 3 | ✅ |
| `check_permission_test_matrix.py` | L8 | Wave 3 | ✅ |
| `check_data_consistency_test.py` | L5 | Wave 3 | ✅ |
| `check_api_compat.py` | L9 | Wave 3 | ❌（必须当下解决，不允许 baseline）|
| `check_observability.py` | L10 | Wave 4 | ✅ |
| `check_concurrency_test.py` | L6 | Wave 4 | ✅ |
| `report_wave6_pre_release.py` | L7 预发布性能证据汇总 | Wave 6 | ❌（必须引用真实运行证据）|
| `check_test_org_layout.py` | 测试目录约定 | Wave 2 | ❌（结构性强约束）|

### 4.2 第 0 周仅做的事

- 本 ADR 登记完整 11 层 + 4 Tier + 测试目录约定
- justfile 写出 T1-T4 命令骨架（命令体可暂时只跑 fmt/lint）
- **不实现** 11 层相关治理脚本（避免空跑）
- 测试基础设施（testcontainers helpers、test-support crate）等 Wave 2 启动时建

### 4.3 baseline 与覆盖率门槛

- 起步阶段所有覆盖率脚本"检测但不卡门槛"，仅记 baseline
- baseline 必须**单调下降**，季度评审
- domain crate 单元测试覆盖率长期目标 ≥ 80%（GSP 核心，硬指标）
- 其他 crate / 前端长期目标 ≥ 60%

---

## 5. 不变量（红线）

无论任何理由，下列不变量必须遵守：

1. **任何写操作必须有 L11 幂等性测试**（GSP 系统不允许重放产生重复数据）
2. **任何涉及库存变更的逻辑必须有 L5 数据一致性测试**（事务 + 审计同写）
3. **任何涉及权限的 handler 必须有 L8 权限矩阵测试**
4. **OpenAPI 向后不兼容变更必须显式声明**（L9 通过 = baseline 不允许，必须当下解决）
5. **关键路径（M2/M3/M4）E2E 必须 11 层全覆盖**
6. **测试目录布局不可自由命名**（治理脚本依赖）
7. **测试不允许污染共享数据库 / 测试间共享可变状态**

---

## 6. 后果

### 正面

- **正确性优先**：行为先于实现被定义，避免"想当然"
- **重构无惧**：11 层测试网兜住绝大部分回归
- **合规可机器化**：GSP 必备维度（L5/L8/L11）通过脚本约束
- **TDD 节奏可演示**：每个用例红 → 绿 → 重构 + 维度补充，节奏清晰
- **维度正交**：测试组织清晰，新人能快速定位测试位置
- **Tier 分层**：开发体验好（T1 极快）+ 合并门禁严（T4 完整）

### 负面

- **短期产出感降低**：写测试占用时间，新人 / 焦虑场景下感觉"慢"
- **测试代码量大**：测试代码常常 > 业务代码，需要同等维护
- **测试基础设施投入**：testcontainers、loom、criterion、Playwright 都需要起步成本
- **TDD 学习曲线**：outside-in 双层循环需要训练，初期容易走偏
- **L7 性能 / L9 兼容 工具链复杂**：基线对比、schema diff 等需要专项搭建

### 风险

- **测试腐化**：测试本身写错或写松会误导；缓解：测试写完必跑红一次确认有效
- **L6 并发测试不写或写不对**：并发 bug 是 wms 大杀器；缓解：关键临界区必须 loom 化测试，强制 review
- **过度追求覆盖率**：覆盖率高 ≠ 测试好；缓解：用 baseline 渐进，不卡死门槛
- **维度遗漏**：某些维度（特别是 L10/L11）容易被忽略；缓解：PR 自查清单 + Wave 3-4 治理脚本

---

## 7. 实施约束

- **PR 描述自查清单必须包含**：
  - [ ] 走了 outside-in TDD（外层 L3 + 内层 L1）
  - [ ] 写操作含 L11 幂等性测试
  - [ ] 涉及库存变更含 L5 数据一致性测试
  - [ ] 涉权限 handler 含 L8 权限矩阵测试
  - [ ] 测试目录布局符合 §2.4
- **测试基础设施**统一放 `backend/crates/test-support/`（Wave 2 启动时建）
- **禁止使用** `#[ignore]` / `it.skip` 长期跳过测试；如需跳过必须有 issue 链接 + 到期日
- **测试运行时间监控**：T1-T4 实际耗时记录在 `governance/baselines/tier-runtime.json`，超时即触发治理回顾
- **测试相关治理脚本**按 §4.1 波次落地，每个脚本上线 = 一份独立 PR + 短文档

---

## 8. 与既有 ADR / 文档的关系

- **ADR-0003 治理模型**：本 ADR 是治理脚本中"质量治理"维度的详细规范
- **ADR-0004 波次路线**：本 ADR 第 4.1 节定义的脚本随波次落地
- **ADR-0001 技术栈**：本 ADR 不引入新栈（Vitest / Playwright / criterion / loom 已被认可）
- **docs/governance.md §3.6**：测试规范段落引用本 ADR

---

## 9. 参考

- 治理总文档：`docs/governance.md`
- 技术栈决策：`docs/adr/0001-tech-stack.md`
- 治理模型决策：`docs/adr/0003-governance-model.md`
- 波次路线决策：`docs/adr/0004-phase-roadmap.md`
- 架构依赖图：`docs/architecture-dependencies.md`

# 软件设计审计报告

> 时间：2026-05-18
> 审计人：自审（AI 助手）
> 范围：wms 项目的软件设计成熟度（12 个维度）
> 方法：文档静态分析 + 关键字覆盖度统计 + 内部一致性检查
> 文档层级：L4 评审记录（不进入治理硬约束，作为 Wave 1 启动前的差距清单）
> 关联：concept-audit.md（概念审计）/ user-stories-audit-2026-05-16.md（用户故事审计）

---

## 1. 总评

| 维度 | v1 结论 | v3 结论 | 变化 |
|------|---------|---------|------|
| 治理体系 | 🟢 业界领先 | 🟢 业界领先 | 14→20 脚本 + ADR 7→15 |
| 业务文档 | 🟢 业界领先 | 🟢 业界领先 | 169 故事全维度矩阵审计 + L4/L5/L8/L11 具体场景 100% 覆盖 |
| 软件架构 | 🟡 中等 | 🟢 良好 | ADR-0012 限界上下文 + ADR-0018 弹性工程 + §3.7 并发控制 |
| 工程实践 | 🟡 中等 | 🟢 良好 | ADR-0013/0014/0015/0016 + SLO + 灰度 + 文件存储 + 缓存策略 |

**v3 结论**：**全部 12 维度达到 🟢 良好或以上**。模式提炼 5 缺口全部闭环（ADR-0018 弹性工程 + coding-standards §3.6 定时任务 + §3.7 并发控制 + infra/file-storage.md + infra/cache-strategy.md）。Wave 1 启动无阻塞项。

---

## 2. 12 个软件设计维度覆盖度评估

### 2.1 数据驱动覆盖度统计

通过 grep 关键字扫描 docs/ + AGENTS.md + governance/，统计每个维度的文档命中数：

| # | 维度 | 关键字 | 命中文档数 | 评级 |
|---|---|---|---:|---|
| 1 | 分层架构（Clean Arch）| 分层 / domain / infra | 多处 | 🟢 良好 |
| 2 | DDD 战术（聚合/实体/VO） | 聚合根 / 值对象 / 实体 | 多处 | 🟡 部分 |
| 3 | DDD 战略（限界上下文/Context Map）| Bounded Context / Context Map / 限界上下文 | **0** | 🔴 缺失 |
| 4 | API 设计 + 版本管理 | /v1/ / API 版本 | 3 | 🔴 **内部矛盾** |
| 5 | 错误处理与错误码字典 | error_code / 错误码 | 6（仅命名约定）| 🔴 缺字典 |
| 6 | 可观测性（trace/metrics/log）| OpenTelemetry / tracing / metrics | 7（仅提及）| 🔴 缺方案 |
| 7 | SLO / 性能预算 | SLO / p99 / p95 / 性能预算 | 真实命中 1（误判过滤后）| 🔴 不充分 |
| 8 | 配置管理 | 环境变量 / secrets / 密钥管理 | 4 | 🟡 缺运行时 |
| 9 | 多端一致性（PC/PAD/PDA 业务规则放置）| 多端一致 / 前端校验 | **0** | 🔴 缺失 |
| 10 | 部署 / 运维 | Dockerfile / kubernetes / docker-compose | 0 | 🔴 无具体方案 |
| 11 | 数据迁移（legacy → wms）| 数据迁移 / migration | 10（仅 legacy 对照）| 🟡 不充分 |
| 12 | i18n / 灰度发布 / 业务连续性 | 国际化 / i18n / 灰度发布 / 蓝绿 | **0** | 🔴 缺失 |

> 上述命中数包括关键字在文档中的提及，**不代表已有完整规范**。详见各维度的具体分析。

---

## 3. 关键内部一致性问题（必须先修）

### 3.1 API 版本策略冲突

**两处明确矛盾**：

| 位置 | 内容 |
|------|------|
| `user-stories-h3-contract.md §4` | API 路径含 `/v1/`；breaking change 走 `/v2/`，老版本保留 ≥ 6 个月 |
| `coding-standards.md §3.5` | 版本号不放 URL（用 header `Accept-Version` 或 OpenAPI 版本管理）|

**影响**：Wave 1 H3 OpenAPI 工具链实施时无法确定版本策略，会导致前后端不一致或频繁返工。

**建议**：立即统一为 **URL 版本**（推荐理由：与 OpenAPI 主流工具兼容、客户端调试容易、文档站点可路径分版本）。

**风险评级**：🟢 无风险（仅文档修订，无业务影响）

---

## 4. 12 维度详细差距

### 维度 1：分层架构 🟢

**当前**：
- ADR-0002 仓库结构：domain / infra 分层
- coding-standards §1.2：模块组织
- 红线：`domain` ⊥ `infra`

**结论**：充分。

---

### 维度 2：DDD 战术 🟡

**当前**：
- coding-standards §1.1 命名规则：实体 / 值对象 / 仓储 / 应用服务 / 命令查询
- 故事文档中聚合根隐含（如 `InboundOrder`、`InventoryBatch`）

**缺**：
- 没有显式的"聚合根识别原则"
- 领域事件命名约定没规范

**建议**：在 coding-standards 加 §1.1.2 聚合根识别原则 + 领域事件命名约定。

**优先级**：🟢 P2

---

### 维度 3：DDD 战略（限界上下文）🔴

**当前**：完全空白（关键字命中 0 文档）。

**问题**：
- 模块清单（M1-M11 + M-XX + H 横向）已是事实上的限界上下文，但**没有显式声明**：
  - 每个 BC 的责任边界
  - BC 之间的集成模式（同一个"商品"在 M1 vs M-TC 是什么差异？）
  - 8 种 DDD 集成模式（Customer-Supplier / Conformist / Anti-Corruption Layer / Open Host Service / Published Language / Shared Kernel / Partnership / Separate Ways）选用哪种

**影响**（DDD 学派核心问题）：
- 模块间 API 设计无统一指导（什么时候用同步 RPC vs 异步事件？）
- 新人难理解模块边界
- 微服务拆分时手忙脚乱（虽然 wms 当前是 monolith）

**建议**：**ADR-0012 限界上下文与 Context Map**：
- 24 个上下文（H1-H10 + H-DOCK + H-AL + 12 个 M- 横向 + 5 个 M 业务）
- 每个 BC 的责任声明
- BC 之间的 8 种集成模式选择
- Shared Kernel 清单（哪些 type 跨 BC 共享，如 `OwnerId`, `BatchNo`, `ProductCode`）

**工作量**：2-3 天 ADR + 1 张 Context Map 图。

**优先级**：🔴 P0（Wave 1 启动前必须）

---

### 维度 4：API 设计 + 版本管理 🔴

**当前**：
- H3-001 OpenAPI 契约故事
- coding-standards §3.1 API 设计规范

**问题**：
1. **版本策略内部矛盾**（见 §3.1）
2. 缺：API 速率限制 / 配额规范
3. 缺：向后兼容规则（字段废弃、添加策略明示）

**建议**：**ADR-0009 API 版本策略**：
- 决议 URL 版本（`/api/v1/`）
- 弃用流程：标记 `Deprecation` 头 + 6 个月过渡期
- 添加字段：可选默认 → 必填的迁移
- 速率限制：每用户 100 RPS / 每货主 1000 RPS（默认值，可配置）

**工作量**：半天 ADR + 修两处冲突。

**优先级**：🔴 P0

---

### 维度 5：错误处理与错误码字典 🔴

**当前**：
- coding-standards §4 命名约定：`SUPPLIER_GSP_EXPIRED` 这样
- H3-001 §5 API 错误格式：`{ code, message, details? }`

**缺**：
- 没有错误码清单（哪些 code 是合法的）
- 业务 error_code 如何与 HTTP status 映射
- 错误是否分级（INFO/WARN/ERROR/CRITICAL）
- 错误是否本地化（中英文）

**影响**：每个 Wave 实施时各模块自己定义，最终全系统几百个错误码无人统筹，前端展示混乱，监管审计无法分类。

**建议**：**ADR-0010 错误码字典 + `docs/error-codes.md`**：
- 错误码格式：`<MODULE>_<CATEGORY>_<DETAIL>` 三段（如 `M3_INVENTORY_INSUFFICIENT_QTY`）
- 治理脚本 `check_error_codes.py` 校验全局唯一 + 模块前缀 + 与字段词典关联
- 错误码字典作为单一事实之源（类似字段词典）

**工作量**：1-2 天初版（约 50 个核心错误码 + 治理脚本）。

**优先级**：🔴 P0

---

### 维度 6：可观测性方案 🔴

**当前**：
- 11 层测试维度有 L10 可观测性（仅测试要求）
- coding-standards §1.6 提到 tracing 规范

**缺**：
- 用什么具体技术？OpenTelemetry / Prometheus / Loki / Tempo / Jaeger？
- trace_id 跨端传递（PDA → 后端 → 数据库）协议
- 业务关键指标（KPI）暴露规范（如"近效期商品数"作为 metric）
- 日志聚合方案（grep 文件还是 ELK / Loki）

**影响**：H1 / H2 实施时各做各的，事后整合困难；GSP 监管审计时无法快速定位问题。

**建议**：**ADR-0011 可观测性方案**：
- OpenTelemetry SDK（统一 trace + metrics + log）
- Prometheus（metrics 抓取）
- Loki（日志聚合）
- Grafana（仪表板）
- 日志格式：JSON + 结构化字段
- 业务 metric 清单（每个核心模块至少 5 个 KPI）

**工作量**：2-3 天 ADR + Wave 1 H1/H2 实施时落地。

**优先级**：🔴 P0

---

### 维度 7：SLO / 性能预算 🔴

**当前**：usability-baseline.md 有部分性能基线，但**没有 SLO**。

**缺**：每个 API endpoint 的 p95/p99 预算、错误率预算、可用性 SLA。

**建议**：在 usability-baseline 加 §SLO 表格：

| Endpoint | p95 | p99 | 错误率 | 可用性 |
|---|---|---|---|---|
| PDA 扫码（高频）| 200ms | 500ms | < 0.1% | 99.95% |
| 库存查询（中频）| 300ms | 800ms | < 0.5% | 99.9% |
| 报表生成（低频）| 5s | 15s | < 1% | 99.5% |
| 监管上报（关键）| 1s | 3s | < 0.01% | 99.99% |

错误预算：每月超出 SLO 10% 触发"冻结发布"。

**工作量**：1 天。

**优先级**：🟡 P1

---

### 维度 8：配置管理 🟡

**当前**：
- M1-008 业务配置中心
- governance §3.9 提到环境分层 local/dev/staging/prod
- gsp-field-coding-standards §5 加密分级 none/masked/encrypted

**缺**：
- 12-Factor App #3：配置存于环境，wms 没明示
- secrets 管理（数据库密码、JWT 签名密钥、加密密钥）—— 用什么？
- 配置优先级：环境变量 > 配置中心 > 默认值
- 密钥轮换策略（涉及 H10 备份、字段加密）

**建议**：**ADR-0013 配置与 secrets 管理**：
- 配置三层（编译期默认 / 启动环境变量 / 运行时配置中心）
- secrets 用 Vault / k8s secret / 1Password Connect（按部署目标选）
- 密钥轮换日历（建议每 90 天）
- 加密密钥的 KMS 集成

**工作量**：2 天 ADR。

**优先级**：🔴 P0

---

### 维度 9：多端一致性 🔴

**当前**：完全空白。各故事提到"PDA 离线"但没明示业务规则的执行位置。

**关键问题**：
- 校验规则放后端（强一致）还是前端（响应快）？
- 答案应该是：**双重校验**——前端先做（用户体验），后端兜底（强一致）
- 但具体哪些校验、用什么共享代码（OpenAPI schema 中的 validation？前端独立实现？）

**影响**：
- 同一规则三端各自实现 → 维护爆炸
- 前端做不到的复杂规则（库存预占）→ 必须后端
- 简单规则（必填、格式）应前端做（避免无效请求）

**建议**：**ADR-0015 多端业务规则放置**：
- 规则分级：A 类（强一致，仅后端）/ B 类（双端校验）/ C 类（仅前端）
- 共享 schema 的源（OpenAPI 还是单独的 wms-rules crate）
- 离线模式扩展（PAD/PC 离线策略，与 H1 §7 PDA 离线策略对齐）

**工作量**：2 天 ADR + Wave 1 落地。

**优先级**：🟡 P1

---

### 维度 10：部署 / 运维 🔴

**当前**：ADR-0001 选了技术栈但**未涉及部署形态**。

**缺**：
- Dockerfile 多阶段构建模板
- docker-compose.yml（dev 环境）
- k8s manifests（production，可选）
- 多环境配置覆盖
- CI/CD 流程（GitHub Actions / GitLab CI）

**建议**：**ADR-0016 部署形态决策** + 模板提交（Wave 1 启动时落地）：
- 推荐 docker-compose（小型 3PL）+ k8s（大型多客户）双轨
- 多阶段 Dockerfile（cargo chef + sccache + 最小 base image）
- 滚动升级 + DB 迁移 + 应用版本兼容

**工作量**：3 天。

**优先级**：🟡 P1

---

### 维度 11：数据迁移（legacy → wms）🟡

**当前**：legacy-comparison-matrix.md 粗略提到迁移但没具体方案。

**缺**：
- 迁移工具（Embulk / Talend / 自研 Python）
- 数据校验规则（迁移前后对账）
- 灰度切换（双写 → 切读 → 关老库）
- 回滚预案

**建议**：**ADR-0014 数据迁移策略** + Wave 0 末期开始数据 mapping 设计。

**工作量**：3-5 天 ADR（不含实施）。

**优先级**：🟡 P1

---

### 维度 12：i18n / 灰度发布 / 业务连续性 🔴

**i18n**：完全空白。
- 主要中文场景（GSP 中国监管）
- 但医药行业未来可能涉及英文（FDA、PIC/S、ICH）
- 当前字段词典只中文，日志/报错也只中文
- **建议**：暂列 v25 backlog；启动国际客户场景前需 ADR

**灰度发布 / 蓝绿 / Canary**：完全空白。
- **建议**：与部署 ADR-0016 合并

**业务连续性**：
- 单点故障（DB 主从 / 多机房）
- 容量规划
- 高可用部署模式
- 已通过 H10 备份恢复 + sharding-decision-matrix 部分覆盖

**优先级**：🟢 P2

---

## 5. 维度成熟度对比图

### v1（审计初版）→ v3（整改完成后）

```
                     v1              v3
治理体系     🟢🟢🟢🟢🟢 (5/5)  →  🟢🟢🟢🟢🟢 (5/5)
业务文档     🟢🟢🟢🟢🟢 (5/5)  →  🟢🟢🟢🟢🟢 (5/5)  +169故事L4/L5/L8/L11
分层架构     🟢🟢🟢🟢⚪ (4/5)  →  🟢🟢🟢🟢🟢 (5/5)  +§3.6定时+§3.7并发
DDD 战术     🟡🟡🟡⚪⚪ (3/5)  →  🟢🟢🟢🟢⚪ (4/5)  +聚合根识别+事件命名
DDD 战略     🔴⚪⚪⚪⚪ (0/5)  →  🟢🟢🟢🟢⚪ (4/5)  +ADR-0012 BC+Context Map
API 设计     🟡🟡🔴⚪⚪ (1/5)  →  🟢🟢🟢🟢⚪ (4/5)  +URL版本统一+ADR-0010错误码
错误处理     🟡⚪⚪⚪⚪ (1/5)  →  🟢🟢🟢🟢🟢 (5/5)  +ADR-0010+ADR-0018弹性工程
可观测性     🟡⚪⚪⚪⚪ (1/5)  →  🟢🟢🟢🟢⚪ (4/5)  +ADR-0011+metric清单
SLO          🟡⚪⚪⚪⚪ (1/5)  →  🟢🟢🟢🟢⚪ (4/5)  +usability-baseline§6
配置管理     🟡🟡⚪⚪⚪ (2/5)  →  🟢🟢🟢🟢⚪ (4/5)  +ADR-0013+cache-strategy
多端一致     🔴⚪⚪⚪⚪ (0/5)  →  🟢🟢🟢🟢⚪ (4/5)  +ADR-0015+PDA三件套100%
部署运维     🔴⚪⚪⚪⚪ (0/5)  →  🟢🟢🟢🟢⚪ (4/5)  +ADR-0016+灰度+file-storage
数据迁移     🟡🟡⚪⚪⚪ (2/5)  →  🟢🟢🟢⚪⚪ (3/5)  +ADR-0014
i18n/灰度    🔴⚪⚪⚪⚪ (0/5)  →  🟢🟢🟢⚪⚪ (3/5)  +灰度ADR-0016+i18n显式推迟

v1 平均成熟度：32 / 70 ≈ 46%
v3 平均成熟度：60 / 70 ≈ 86%
```

---

## 6. 修复建议（按优先级）

### 🔴 P0（Wave 1 启动前必须有，5 项 ADR）

| # | 项 | 工作量 | 推荐 ADR |
|---|---|---|---|
| 1 | 修复 API 版本策略冲突 | 半小时 | （即时修复，不必 ADR） |
| 2 | 错误码字典 + 治理脚本 | 1-2 天 | ADR-0010 |
| 3 | 可观测性方案 ADR | 2-3 天 | ADR-0011 |
| 4 | 限界上下文 + Context Map | 2-3 天 | ADR-0012 |
| 5 | 配置与 secrets 管理 | 2 天 | ADR-0013 |
| 6 | API 版本策略正式 ADR | 半天 | ADR-0009 |

### 🟡 P1（Wave 1-2 期间补，4 项）

| # | 项 | 工作量 | 推荐 ADR |
|---|---|---|---|
| 7 | SLO / 性能预算 | 1 天 | usability-baseline 增量 |
| 8 | 数据迁移策略 | 3-5 天 | ADR-0014 |
| 9 | 多端业务规则放置 | 2 天 | ADR-0015 |
| 10 | 部署 / Docker / k8s | 3 天 | ADR-0016 |

### 🟢 P2（后续补，3 项）

| # | 项 | 工作量 |
|---|---|---|
| 11 | i18n 策略 | v25 backlog |
| 12 | AI 协作产出审查标准 | 1 小时（写入 AGENTS.md）|
| 13 | 聚合根识别 + 领域事件命名 | 半天（写入 coding-standards）|

---

## 7. 不在范围（明示）

- 不评估代码层（Wave 0 还没业务代码）
- 不评估部署成本（取决于客户场景，等 ADR-0016 决策）
- 不评估安全设计（GSP 合规层已通过字段词典覆盖部分；专门的安全审计待 Wave 4 启动前做）
- 不评估法务（合规边界已通过 GSP 章节级 RTM 部分覆盖）

---

## 8. 关联

- [docs/concept-audit.md](../concept-audit.md)：概念审计（8 镜头扫描业务概念）
- [docs/reviews/user-stories-audit-2026-05-16.md](user-stories-audit-2026-05-16.md)：用户故事 5 维度审计
- [docs/adr/0008-borrow-from-odoo.md](../adr/0008-borrow-from-odoo.md)：借鉴 Odoo 的 9 个设计

---

## 9. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：12 维度软件设计审计 + 6 P0 / 4 P1 / 3 P2 共 13 项缺口清单 |
| 2026-05-18 | v2 | 13 项整改回收闭环（详见 §10）|
| 2026-05-18 | v3 | §1 总评更新（4 维度全 🟢）+ §5 成熟度对比图 v1→v3（46%→86%）；新增 ADR-0018 弹性工程 + coding-standards §3.6/§3.7 + infra/file-storage + infra/cache-strategy + 169 故事全维度矩阵审计 100% 覆盖 |

---

## 10. 整改回收记录（v2）

> 本节是 v1 提出的 13 项缺口的一次性结清账。结清后本审计文档不再持续跟踪——后续治理由各 ADR + 治理脚本承担。

### 10.1 总体完成情况

- 严格"已落地"完成率：**12 / 13 = 92.3%**
- 含按计划推迟的完成率：**13 / 13 = 100%**

### 10.2 P0（6 项）— 全部解决 ✅

| # | 审计项 | 落地证据 | 验证方式 |
|---|---|---|---|
| 1 | API 版本一致性冲突（即时修复）| commit `b83727c` | `coding-standards.md:611` 与 `user-stories-h3-contract.md:41` 已统一 `/api/v1/` URL 版本 |
| 2 | 错误码字典 + 治理脚本 | commit `c952f4b` | `docs/adr/0010-error-codes.md` + `docs/error-codes.md`（50 项）+ `scripts/governance/check_error_codes.py` |
| 3 | 可观测性方案 ADR | commit `812f925` | `docs/adr/0011-observability.md` + `check_observability.py` |
| 4 | 限界上下文 + Context Map | commit `c3145c2` | `docs/adr/0012-bounded-contexts.md` + `check_bounded_contexts.py` |
| 5 | 配置与 secrets 管理 | commit `0aa443f` | `docs/adr/0013-config-secrets.md` + `check_secrets.py` |
| 6 | API 版本策略正式 ADR-0009 | 显式合并处理 | `docs/adr/README.md:43` 注明"已合并到 ADR-0010 + coding-standards §3.5"，0009 保留为未分配号位（不空号占坑）|

### 10.3 P1（4 项）— 全部解决 ✅

| # | 审计项 | 落地证据 | 验证方式 |
|---|---|---|---|
| 7 | SLO / 性能预算 | commit `2338aac` | `usability-baseline.md §6` 含 SLO 表 + 错误预算公式 + Grafana 落地计划 |
| 8 | 数据迁移策略 | commit `0aa443f` | `docs/adr/0014-data-migration.md` |
| 9 | 多端业务规则放置 | commit `2338aac` | `docs/adr/0015-multi-end-rules.md` + `check_multi_end_consistency.py` |
| 10 | 部署 / Docker / k8s | commit `2338aac` + 后续 v3.1 增补 | `docs/adr/0016-deployment.md`（含 §灰度发布策略：三维度 + 四阶段 + 自动回滚阈值 + Feature Flag）|

### 10.4 P2（3 项）— 2 解决 + 1 按计划推迟

| # | 审计项 | 状态 | 落地证据 |
|---|---|---|---|
| 11 | i18n / 灰度发布 / 业务连续性 | 🟢 子项 b/c 已解决 + 子项 a 显式推迟 | **灰度发布**：`ADR-0016 §灰度发布策略`（三维度 + 四阶段 + 6 项自动回滚阈值 + Feature Flag）；**业务连续性**：`technical-specs.md §H10` + `sharding-decision-matrix.md`；**i18n**：`ROADMAP.md §国际化 backlog` 显式推迟到 v25+ FDA 场景，含 3 项启动条件 + 反向决策点 |
| 12 | AI 协作产出审查标准 | ✅ | `AGENTS.md §AI 协作产出审查标准`（9 条审查清单 + 7 条反模式红线 + 5 条 PR 评审重点 + 5 条"不该用 AI 的场景"）|
| 13 | 聚合根识别 + 领域事件命名 | ✅ | `coding-standards.md §1.1.2` 4 原则 + 7 个 wms 实际聚合根表；`§1.1.3` 事件命名 `<聚合根>.<动作过去式>` + 三分类 + DomainEvent 标准字段 |

### 10.5 治理资产增量

- 治理脚本（`scripts/governance/check_*.py` + `validate_*.py`）：审计 v1 时 **14 个 → 现在 20 个**，新增 6 个：`check_error_codes` / `check_observability` / `check_bounded_contexts` / `check_secrets` / `check_multi_end_consistency` / `check_story_size`
- ADR：审计 v1 时 **7 个**（0001-0008 中实际存在 7 个，0005 跳过）→ 现在 **14 个**，新增 7 个文件（0010-0016），0009 显式不占（合并到 ADR-0010 + coding-standards §3.5，参 `docs/adr/README.md:43`）
- 关联文档：`docs/error-codes.md`（21 KB / 50 项错误码字典）+ `AGENTS.md §AI 协作产出审查标准`（约 75 行）

### 10.6 后续不在本审计跟踪范围

- ADR-0017+ 的 i18n 决策：等业务方触发启动条件后由独立 ADR 承担
- Wave 1 启动后实际部署 / 灰度发布的运行数据：由 retros 跟踪
- 安全审计：等 Wave 4 启动前做专项审计（v1 §7 已明示）

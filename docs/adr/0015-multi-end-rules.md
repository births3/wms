# ADR-0015：多端业务规则放置（PC + PAD + PDA）

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0001 / ADR-0010 / ADR-0012 / docs/coding-standards.md §3

---

## 背景

软件设计审计 §4 维度 9 识别多端一致性缺口（完全空白）：

- wms 三端：PC（管理端）+ PAD（管理端的平板形态）+ PDA（仓内手持）
- 各业务故事提到"PDA 离线"但**没明示业务规则的执行位置**：
  - 校验规则放后端（强一致）还是前端（响应快）？
  - 同一规则三端各自实现 → 维护爆炸
  - 离线模式：PDA 已有 H1 §7 离线策略；PAD/PC 离线呢？
- 当前 0 文档覆盖

不解决会导致 Wave 1+ 实施时各端开发者按自己理解写校验，最终：
- 同一规则 3 个版本（前后端 + PDA），改一处别两处不同步
- 用户体验差（前端不校验，后端拒绝时已经填了一堆）
- 离线时无规则（弱网用户写垃圾数据）

---

## 候选方案

### 方案 A（推荐）：规则三级分类 + 共享 schema 源

```
A 类（强一致，仅后端）：库存预占 / 状态机迁移 / 审批源 / 唯一性约束
B 类（双端校验）：必填 / 格式 / 长度 / 范围 / 枚举值
C 类（仅前端）：UI 反馈 / 输入提示 / 自动补全
```

- A 类：后端 100%，前端不实现（前端调 API 处理错误）
- B 类：前端先做（用户体验）+ 后端兜底（强一致）
- C 类：前端独立（不进 API）

**共享 schema 来源**：OpenAPI schema（utoipa 生成）+ 前端 `openapi-typescript` 消费。

### 方案 B：纯后端校验

**否决**：用户填一长串表单后才报错，体验极差；移动端流量浪费。

### 方案 C：纯前端校验 + 后端宽松

**否决**：恶意客户端绕过前端 → 数据污染 → GSP 合规风险。

---

## 决策

**采用方案 A：规则三级分类 + OpenAPI schema 作为共享 schema 源**。

### 三级规则分类

| 级别 | 规则类型 | 执行位置 | 故事中标识 |
|---|---|---|---|
| **A 类** | 强一致约束 | 仅后端（**前端必须不实现**）| `[A]` |
| **B 类** | 通用校验 | 前后端双重 | `[B]` |
| **C 类** | UI 体验 | 仅前端 | `[C]` |

### A 类（仅后端）规则示例

| 规则 | 故事位置 | 不可前端实现的原因 |
|---|---|---|
| 库存预占 | M3-003 / M4-002 | 跨用户/跨请求一致性，需事务 + 锁 |
| 状态机迁移 | M2/M3/M4 各状态机 | 状态依赖最新数据库状态 |
| 审批源校验 | M3-003 + 跨故事约束 | 跨模块依赖 |
| 唯一性约束 | M1-001 商品编码 / M2-001 ASN 号 | 数据库 UNIQUE 索引为唯一保证 |
| 双人复核（M-VR）| US-VR-006 | 服务端校验两个 user_id 不同 + 角色匹配 |
| 货主隔离 | H1 / 全部业务 | 跨货主数据安全 |
| FEFO/FIFO 分配 | M3-002 / M4-002 | 全局批次池决策 |
| 跨模块事件触发 | H2-005 事件总线 | 服务端事件发布 |

### B 类（前后端双重）规则示例

| 规则 | 共享 schema 字段 |
|---|---|
| 必填字段（如 ASN 号 / 批号） | `required: true` |
| 字符串格式（USCC 18 位 / 批号正则）| `pattern: ...` |
| 长度限制（VARCHAR(N)）| `maxLength: N` |
| 范围限制（温度 -50 ~ 50）| `minimum / maximum` |
| 枚举值（储存条件 4 选 1） | `enum: [...]` |
| 日期约束（有效期 > 生产日期）| 用 `@assert` 注解或后端 validator |

**实现路径**：
- 后端 Rust：`utoipa` 生成 OpenAPI schema（含 `format/pattern/required`）
- 前端 TS：`openapi-typescript` 消费 schema → `zod` 自动生成 schema → react-hook-form 集成
- PDA RN：同前端，复用 `@wms/api-client` 包

### C 类（仅前端）规则示例

| 规则 | 实现位置 |
|---|---|
| 输入实时建议（autocomplete）| React component |
| 重复输入防抖 | hooks |
| UI 状态反馈（loading / error 颜色）| component |
| 输入掩码（手机号 138-XXXX-XXXX 显示）| input format |
| 离线缓存提示（"当前离线"）| H1 §7 PDA 离线策略 |

### 离线模式扩展

参 H1 §7 PDA 离线策略，扩展到 PAD/PC：

| 端 | 默认离线行为 | 同步策略 |
|---|---|---|
| **PDA** | 关键作业可离线（已有 H1 §7） | 24 小时上限 + 冲突走"待主管处理" |
| **PAD（管理端）** | 仅查询可离线（不允许写）| 不必同步（重新登录即可）|
| **PC（管理端）** | 不离线 | 浏览器在线性强 |

> **决策**：PAD/PC 不实现写操作离线（避免复杂度爆炸）；只 PDA 因业务必需离线。

### Schema 共享流程

```
Wave 1 W1.C 启动后：
1. 后端 handler 用 utoipa 注解（schema + validation）
2. cargo build 时生成 shared/openapi/openapi.json
3. pnpm build 时 openapi-typescript 生成 packages/api-client/types.ts
4. 前端 / PDA 从 @wms/api-client 导入 schema
5. 前端用 zod-from-openapi 生成 zod schema
6. react-hook-form + zodResolver 自动校验
```

**单一事实之源**：后端 utoipa 注解（不允许前端绕过自定义校验规则）。

---

## 后果

### 正面

- **维护一处生效**：B 类规则改一次（后端），前端编译时自动同步
- **用户体验好**：B 类规则前端先做，无需等 API
- **GSP 合规**：A 类强约束在服务端，恶意客户端绕过无效
- **离线策略明确**：PDA 离线 + PAD/PC 不离线，团队不用纠结

### 负面

- **utoipa + openapi-typescript + zod 工具链复杂**：初学曲线陡
- **应对**：Wave 1 W1.C 一次性建好，后续模块直接复用

### 风险

- **A 类规则误标 B 类**：前端实现了 A 类规则 → 客户端可绕过
- **应对**：治理脚本 `check_multi_end_consistency.py` 扫描故事中 [A]/[B]/[C] 标注，校验 A 类规则不在前端代码中实现

---

## 实施约束

1. **故事中必须标注规则级别**：所有验收标准的校验规则用 `[A]` / `[B]` / `[C]` 前缀标注
2. **A 类前端必须不实现**：治理脚本扫描前端代码，含 A 类规则名 → 报错
3. **B 类必须用 OpenAPI schema**：不允许前端独立实现 zod schema（必须从 @wms/api-client 导入）
4. **C 类不进 OpenAPI**：仅 UI 体验，不暴露给后端
5. **PDA 离线**：参 H1 §7 PDA 离线统一策略
6. **PAD/PC 写操作不允许离线**

---

## 治理脚本

`scripts/governance/check_multi_end_consistency.py`（T1 级）：

- 扫描 `docs/domain/user-stories-*.md` 验收标准中的 `[A]`/`[B]`/`[C]` 标注
- 校验每个写操作故事至少含 1 个 A 类规则
- 校验 A 类规则在故事中描述明确（不能只标号不写内容）
- Wave 1 启动后扩展：扫描前端代码（apps/web-admin / apps/pda-mobile）是否实现了 A 类规则名（如 `transitionStatus` 这种）

---

## 参考

- OpenAPI Specification: https://www.openapis.org/
- utoipa (Rust OpenAPI): https://github.com/juhaku/utoipa
- openapi-typescript: https://github.com/drwpow/openapi-typescript
- zod: https://zod.dev/
- "BFF Pattern" Sam Newman: https://samnewman.io/patterns/architectural/bff/

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：A/B/C 三级规则分类 + OpenAPI schema 作为单一事实之源 + PDA 离线扩展边界 + 治理脚本 |

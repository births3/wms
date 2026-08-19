# ADR-0010：错误码体系与字典

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0001 / ADR-0008 / docs/coding-standards.md §4

---

## 背景

软件设计审计（[reviews/software-design-audit-2026-05-18.md](../reviews/software-design-audit-2026-05-18.md) §4 维度 5）识别错误处理体系缺口：

- 命名约定有了：`SUPPLIER_GSP_EXPIRED` 这样
- API 错误格式有了：`{ code, message, details? }`
- **但没有错误码字典**——哪些 code 是合法的？业务码与 HTTP status 怎么映射？错误是分级的吗？

不解决会导致：
- 各 Wave 模块各自定义错误码，最终全系统几百个无人统筹
- 前端展示与监管审计无法分类
- L4 测试维度（错误路径）无法编写完整测试

---

## 候选方案

### 方案 A（推荐）：数据驱动错误码字典 + 治理脚本

- 单一事实之源：`docs/error-codes.md` §6 YAML 块（类似字段词典 §6 模式）
- 三段式编码：`<MODULE>_<CATEGORY>_<DETAIL>`
- 治理脚本 `check_error_codes.py` 校验唯一性 + 前缀 + 与字段词典关联

### 方案 B：每个模块自治

- 各模块自己在故事中定义错误码
- 后期再合并

**否决理由**：合并阶段成本高 + 命名冲突高发。

### 方案 C：HTTP status 为主

- 仅用 HTTP 4xx/5xx，不引入业务码

**否决理由**：医药 GSP 监管要求**业务异常的精确分类**（如"批号未在 ERP 字典中"vs"批号格式错误"，HTTP 都是 400），无法满足。

---

## 决策

**采用方案 A：数据驱动错误码字典**。

### 错误码格式

```
<MODULE>_<CATEGORY>_<DETAIL>
```

| 段 | 字符 | 示例 |
|---|---|---|
| MODULE | 模块前缀（H1/H2/M1.../M-TC 等） | `M3` / `M_TC` / `H1` |
| CATEGORY | 业务分类 | `INVENTORY` / `BATCH` / `AUTH` / `VALIDATION` |
| DETAIL | 具体错误 | `INSUFFICIENT_QTY` / `EXPIRED` / `FORMAT_INVALID` |

> 注：模块编号中的 `-` 在错误码中替换为 `_`（M-TC → `M_TC`）。

### 错误码示例

| 错误码 | HTTP | 严重度 | 含义 |
|---|---|---|---|
| `M3_INVENTORY_INSUFFICIENT_QTY` | 409 | warning | 库存不足无法满足请求数量 |
| `M3_BATCH_EXPIRED` | 409 | error | 批号已过期，不可出库 |
| `M3_BATCH_RECALLED` | 409 | critical | 批号已召回 |
| `M2_ASN_DUPLICATE` | 409 | warning | ASN 单号重复 |
| `M2_ARRIVAL_TIME_BEFORE_DEPARTURE` | 422 | error | 到货时间早于启运时间 |
| `M_VR_RULE_REJECTED` | 422 | warning | 规则引擎拦截 |
| `H1_AUTH_TOKEN_EXPIRED` | 401 | warning | token 过期 |
| `H1_AUTH_INSUFFICIENT_PERMISSION` | 403 | error | 权限不足 |
| `H1_TENANT_MISMATCH` | 403 | critical | 跨货主访问尝试 |

### 严重度分级（4 级）

| 级别 | 含义 | 处置 |
|---|---|---|
| `info` | 信息提示 | 正常路径 + 用户可知 |
| `warning` | 业务规则拦截 | 用户改输入即可继续 |
| `error` | 业务异常 | 需要人工干预或重试 |
| `critical` | 合规/安全异常 | **必须**触发 H-AL 告警 + 审计 |

### API 错误响应格式（强制）

```json
{
  "code": "M3_INVENTORY_INSUFFICIENT_QTY",
  "message": "库存不足：请求 100，可用 80",
  "severity": "warning",
  "details": {
    "requested_qty": 100,
    "available_qty": 80,
    "batch_no": "BN20260517001"
  },
  "trace_id": "01H7K8...",
  "retry_hint": "no_retry"
}
```

> `retry_hint` 合法值：`no_retry` / `fast` / `standard` / `persistent`，语义定义见 [ADR-0018 §2.2](0018-resilience-engineering.md)。

### 字典存储

`docs/error-codes.md` §6 YAML 块结构：

```yaml
error_codes:
  - code: M3_INVENTORY_INSUFFICIENT_QTY
    module: M3
    category: INVENTORY
    detail: INSUFFICIENT_QTY
    http_status: 409
    severity: warning
    message_zh: "库存不足"
    message_en: "Insufficient stock"
    related_fields: [batch_no, qty]
    related_stories: [US-M3-001, US-M4-002]
    introduced_in: v3.1
```

每条错误码必填 11 项（code/module/category/detail/http_status/severity/message_zh/message_en/related_fields/related_stories/introduced_in）。

### 治理脚本

`scripts/governance/check_error_codes.py`（T1 级）：

- 唯一性：每个 code 全局唯一
- 前缀合法性：MODULE 必须在 architecture-dependencies §1 模块清单内
- 严重度白名单：info/warning/error/critical
- HTTP status 范围：100-599
- related_fields 必须存在于 §6 字段词典
- related_stories 必须存在（故事 ID 唯一性）

---

## 后果

### 正面

- **业务规则透明**：每个错误码对应明确的业务场景，前后端 + 监管审计可分类
- **L4 测试覆盖完整**：每个错误码必有对应测试用例
- **多语言友好**：message_zh + message_en 双语字段，未来 i18n 扩展容易
- **治理脚本兜底**：错误码新增时自动校验唯一性 + 字段关联

### 负面

- **学习成本**：开发者写代码前要查字典，初期 2-3 周适应期
- **字典维护**：随业务增长字典会变大，治理脚本性能需关注（预估 < 500 项可控）

### 风险

- **粒度过细**：模块自定义太多无关紧要的码 → 字典爆炸
- **应对**：每个 PR 加错误码必走 review，治理脚本统计每模块码数（预警 > 30 个/模块）

---

## 实施约束

1. 所有抛出业务异常的代码**必须**用字典中的 code，不允许自由文本
2. 新增错误码必须先改字典再写代码（治理脚本会卡）
3. 错误码废弃**不删除**（标 `deprecated_at`），保留 1 年后归档（因为日志/审计可能引用）
4. critical 级别错误码自动触发 H-AL 告警（强制接入）
5. Wave 1 启动前必须有 ≥ 30 个错误码（核心模块基础覆盖）

---

## 参考

- Stripe API Errors: https://stripe.com/docs/api/errors
- AWS Service Error Codes: https://docs.aws.amazon.com/general/latest/gr/api-error-codes.html
- Google Cloud Error Codes: https://cloud.google.com/apis/design/errors

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：错误码格式 + 严重度分级 + 字典 §6 YAML + 治理脚本约束 |

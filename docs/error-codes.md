# 错误码字典（Error Codes Dictionary）

> 时间：2026-07-21
> 版本：v3.10（当前 175 项）
> 文档层级：L2 规范（必须遵守）
> 关联：[ADR-0010](adr/0010-error-codes.md) / [coding-standards.md §4](coding-standards.md)

---

## 1. 目的

本文档是 wms 项目所有业务错误码的**单一事实之源**。

- 所有抛出业务异常的代码必须使用本字典中的 `code`
- 新增错误码必须先改本字典再写代码
- 治理脚本 `check_error_codes.py` 校验唯一性 + 模块前缀 + 字段关联

---

## 2. 格式

### 2.1 编码规则

```
<MODULE>_<CATEGORY>_<DETAIL>
```

- MODULE：模块前缀（H1/H2/M1.../M_TC 等，`-` 替换为 `_`）
- CATEGORY：业务分类（INVENTORY/BATCH/AUTH/VALIDATION 等）
- DETAIL：具体错误（INSUFFICIENT_QTY/EXPIRED/FORMAT_INVALID 等）

全大写 + 下划线分隔（SCREAMING_SNAKE_CASE）。

### 2.2 严重度（4 级）

| 级别 | 含义 | 处置 |
|---|---|---|
| `info` | 信息提示 | 正常路径 + 用户可知 |
| `warning` | 业务规则拦截 | 用户改输入即可继续 |
| `error` | 业务异常 | 需要人工干预或重试 |
| `critical` | 合规/安全异常 | **必须**触发 H-AL 告警 + 审计 |

### 2.3 API 错误响应格式

```json
{
  "code": "M3_INVENTORY_INSUFFICIENT_QTY",
  "message": "库存不足：请求 100，可用 80",
  "severity": "warning",
  "details": { "requested_qty": 100, "available_qty": 80 },
  "trace_id": "01H7K8...",
  "retry_hint": "client_should_reduce"
}
```

### 2.4 必填字段（11 项）

| 字段 | 类型 | 说明 |
|---|---|---|
| code | string | 错误码（全局唯一）|
| module | string | 模块前缀 |
| category | string | 业务分类 |
| detail | string | 具体错误 |
| http_status | int | HTTP 状态码（100-599）|
| severity | enum | info/warning/error/critical |
| message_zh | string | 中文消息（默认）|
| message_en | string | 英文消息（i18n 预备）|
| related_fields | array | 关联字段词典 §6 canonical |
| related_stories | array | 关联用户故事 ID（US-XXX-NNN）|
| introduced_in | string | 首次引入版本（v3.1 / v25 等）|

---

## 3. 严重度分布概览

| 级别 | 数量 | 主要场景 |
|---|---|---|
| info | 1 | 状态变更通知 / 数据已存在等正常路径 |
| warning | 55 | 业务规则拦截（库存不足 / 资质过期等）|
| error | 109 | 业务异常（数据冲突 / 校验失败）|
| critical | 10 | 合规/安全异常（跨货主访问 / 篡改尝试）|
| **合计** | **175** | — |

---

## 4. 模块前缀分布

| 前缀 | 错误码数 | 主要场景 |
|---|---|---|
| H1 | 24 | 鉴权 / 多租户隔离 / API Key 生命周期 |
| H2 | 5 | 审计 / 事件总线 |
| H4 | 14 | 通知发送 / 审批 / 重发 |
| H6 | 2 | 状态机查询与校验 |
| H5 | 2 | 快递面单 |
| H8 | 1 | ERP 接口表探查 |
| H_DOCK | 11 | 月台预约 |
| H_AL | 28 | 告警引擎 |
| M1 | 12 | 主数据校验 / 配置中心 |
| M2 | 8 | 入库流程 |
| M3 | 8 | 库存与状态 |
| M4 | 10 | 出库与拣选 |
| M_VR | 10 | 规则引擎与双人策略 |
| M_QL | 1 | 质量联系单 |
| M_TC | 2 | 追溯码 |
| M_PM | 1 | 参数对照 |
| M_DI | 12 | 药检平台配置 |
| M_TE | 24 | 任务类型、任务组、优先级规则、释放控制与执行主链 |

---

## 5. 与其他文档的关系

```
docs/error-codes.md（本文档，单一事实之源）
  │
  ├─→ docs/adr/0010-error-codes.md         （决策依据）
  ├─→ docs/adr/0024-auth-model.md §2.6     （AUTH-XXX 短编码定义）
  ├─→ docs/coding-standards.md §4         （命名规范继承）
  ├─→ docs/compliance/gsp-field-traceability.md §6  （字段关联）
  ├─→ docs/domain/user-stories-*.md        （故事关联）
  └─→ scripts/governance/check_error_codes.py（治理校验）
```

### 5.1 AUTH-XXX 短编码速查（ADR-0024 / spike-001 实测）

ADR-0024 §2.6 定义 9 个 AUTH-XXX 短编码，用于前端按 code 切换提示（与 Stripe API 风格一致）。
本表是面向前端的 UX 速查；治理脚本字典（§6 yaml）使用 `H1_AUTH_*` 长编码。

| 短码 | HTTP | 触发 | 对应 H1_* 长编码 |
|---|---|---|---|
| AUTH-001 | 401 | 缺 Authorization 头 | `H1_AUTH_HEADER_MISSING` （Wave 1 W1.A 加 §6 yaml） |
| AUTH-002 | 401 | Authorization 格式错（非 Bearer xxx） | `H1_AUTH_HEADER_MALFORMED` （同上） |
| AUTH-003 | 401 | token 无效 / 已过期 / 解析失败 | `H1_AUTH_TOKEN_EXPIRED` + `H1_AUTH_TOKEN_INVALID`（已在 §6） |
| AUTH-004 | 401 | token 已撤销（blacklist 命中） | `H1_AUTH_TOKEN_REVOKED` （Wave 1 W1.A 加） |
| AUTH-005 | 403 | 权限不足（permissions 缺所需 code） | `H1_AUTH_INSUFFICIENT_PERMISSION`（已在 §6） |
| AUTH-006 | 403 | 跨货主越权 | `H1_TENANT_MISMATCH`（已在 §6） |
| AUTH-007 | 401 | refresh_token 无效或过期 | `H1_AUTH_REFRESH_TOKEN_INVALID` （Wave 1 W1.A 加） |
| AUTH-008 | 401 | 密码错（业务方可决定合并到 AUTH-003 防用户枚举） | `H1_AUTH_PASSWORD_INVALID` （Wave 1 W1.A 加） |
| AUTH-009 | 401 | permissions 已失效（v0.2 新增；token 内 permissions 过时，需重新登录） | `H1_AUTH_PERMISSIONS_REVOKED` （Wave 1 W1.A 加） |

**实施约束**（Wave 1 W1.A）：
- 5 个"Wave 1 W1.A 加"的 H1_* 长编码必须在 W1.A 落地前补到 §6 yaml（治理脚本依赖）
- 后端 IntoResponse 序列化 `code: "AUTH-XXX"` + `message: "中文提示"` 到 HTTP body（spike-001 实测可行）
- 前端 `packages/api-client` 错误处理 switch case 用本表的 9 个短码

修改本表必须同步 ADR-0024 §2.6（双向一致是治理硬约束）。

---

## 6. 错误码字典（治理脚本输入）

> 本节为机器可读的错误码字典，用于 `scripts/governance/check_error_codes.py` 自动核对。
> 修改本节须保持 YAML 格式合法 + 字段完整。

```yaml
error_codes:
  # ========== H1 鉴权与多租户 ==========
  - code: H1_AUTH_HEADER_MISSING
    module: H1
    category: AUTH
    detail: HEADER_MISSING
    http_status: 401
    severity: warning
    message_zh: '缺少 Authorization 头'
    message_en: 'Authorization header missing'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  - code: H1_AUTH_HEADER_MALFORMED
    module: H1
    category: AUTH
    detail: HEADER_MALFORMED
    http_status: 401
    severity: warning
    message_zh: 'Authorization 格式错误'
    message_en: 'Authorization header malformed'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  - code: H1_AUTH_TOKEN_EXPIRED
    module: H1
    category: AUTH
    detail: TOKEN_EXPIRED
    http_status: 401
    severity: warning
    message_zh: 'token 已过期，请重新登录'
    message_en: 'Token expired, please re-login'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  - code: H1_AUTH_TOKEN_INVALID
    module: H1
    category: AUTH
    detail: TOKEN_INVALID
    http_status: 401
    severity: error
    message_zh: 'token 签名无效'
    message_en: 'Token signature invalid'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  - code: H1_AUTH_TOKEN_REVOKED
    module: H1
    category: AUTH
    detail: TOKEN_REVOKED
    http_status: 401
    severity: warning
    message_zh: 'token 已撤销，请重新登录'
    message_en: 'Token revoked, please re-login'
    related_fields: []
    related_stories: [US-H1-001, US-H1-004]
    introduced_in: v3.1

  - code: H1_AUTH_INSUFFICIENT_PERMISSION
    module: H1
    category: AUTH
    detail: INSUFFICIENT_PERMISSION
    http_status: 403
    severity: error
    message_zh: '权限不足'
    message_en: 'Insufficient permission'
    related_fields: []
    related_stories: [US-H1-002, US-H1-003]
    introduced_in: v3.1

  - code: H1_TENANT_MISMATCH
    module: H1
    category: TENANT
    detail: MISMATCH
    http_status: 403
    severity: critical
    message_zh: '跨货主访问被拒绝'
    message_en: 'Cross-tenant access denied'
    related_fields: [tenant_id, owner_id]
    related_stories: [US-H1-002]
    introduced_in: v3.1

  - code: H1_AUTH_REFRESH_TOKEN_INVALID
    module: H1
    category: AUTH
    detail: REFRESH_TOKEN_INVALID
    http_status: 401
    severity: warning
    message_zh: 'refresh_token 无效或过期'
    message_en: 'Refresh token invalid or expired'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  - code: H1_AUTH_PASSWORD_INVALID
    module: H1
    category: AUTH
    detail: PASSWORD_INVALID
    http_status: 401
    severity: warning
    message_zh: '密码错误'
    message_en: 'Password invalid'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  - code: H1_AUTH_PERMISSIONS_REVOKED
    module: H1
    category: AUTH
    detail: PERMISSIONS_REVOKED
    http_status: 401
    severity: warning
    message_zh: 'permissions 已失效，请重新登录'
    message_en: 'Permissions revoked, please re-login'
    related_fields: []
    related_stories: [US-H1-002, US-H1-003]
    introduced_in: v3.1

  - code: H1_LOGIN_LOCKED
    module: H1
    category: AUTH
    detail: ACCOUNT_LOCKED
    http_status: 423
    severity: warning
    message_zh: '账号已被锁定（连续登录失败次数过多）'
    message_en: 'Account locked due to repeated login failures'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  - code: H1_PASSWORD_POLICY_VIOLATION
    module: H1
    category: AUTH
    detail: PASSWORD_WEAK
    http_status: 422
    severity: warning
    message_zh: '密码不符合强度要求'
    message_en: 'Password does not meet policy'
    related_fields: []
    related_stories: [US-H1-001]
    introduced_in: v3.1

  # ========== H1 API Key 生命周期 ==========
  - code: H1_APIKEY_IDEMPOTENCY_REQUIRED
    module: H1
    category: APIKEY
    detail: IDEMPOTENCY_REQUIRED
    http_status: 400
    severity: error
    message_zh: '缺少 Idempotency-Key'
    message_en: 'Idempotency-Key is required'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_INVALID_REQUEST
    module: H1
    category: APIKEY
    detail: INVALID_REQUEST
    http_status: 422
    severity: warning
    message_zh: 'API Key 请求字段非法'
    message_en: 'API Key request is invalid'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_INVALID_SCOPE
    module: H1
    category: APIKEY
    detail: INVALID_SCOPE
    http_status: 422
    severity: warning
    message_zh: 'API Key 作用域非法'
    message_en: 'API Key scope is invalid'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_INVALID_EXPIRY
    module: H1
    category: APIKEY
    detail: INVALID_EXPIRY
    http_status: 422
    severity: warning
    message_zh: 'API Key 过期时间非法'
    message_en: 'API Key expiry is invalid'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_INVALID_GRACE_PERIOD
    module: H1
    category: APIKEY
    detail: INVALID_GRACE_PERIOD
    http_status: 422
    severity: warning
    message_zh: 'API Key 轮换宽限期非法'
    message_en: 'API Key grace period is invalid'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_NOT_FOUND
    module: H1
    category: APIKEY
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: 'API Key 不存在'
    message_en: 'API Key not found'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_IDEMPOTENCY_CONFLICT
    module: H1
    category: APIKEY
    detail: IDEMPOTENCY_CONFLICT
    http_status: 409
    severity: error
    message_zh: '幂等键已被不同请求使用'
    message_en: 'Idempotency-Key conflicts with another request'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_REVOKED
    module: H1
    category: APIKEY
    detail: REVOKED
    http_status: 409
    severity: warning
    message_zh: 'API Key 已吊销，不能轮换'
    message_en: 'Revoked API Key cannot be rotated'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_INVALID_RESPONSIBLE_USER
    module: H1
    category: APIKEY
    detail: INVALID_RESPONSIBLE_USER
    http_status: 422
    severity: warning
    message_zh: '负责人不属于当前货主'
    message_en: 'Responsible user does not belong to the current owner'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_INVALID_WAREHOUSE_SCOPE
    module: H1
    category: APIKEY
    detail: INVALID_WAREHOUSE_SCOPE
    http_status: 422
    severity: warning
    message_zh: '仓库范围不属于当前货主'
    message_en: 'Warehouse scope does not belong to the current owner'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_INTERNAL
    module: H1
    category: APIKEY
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: 'API Key 处理失败'
    message_en: 'API Key processing failed'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  - code: H1_APIKEY_REQUEST_REJECTED
    module: H1
    category: APIKEY
    detail: REQUEST_REJECTED
    http_status: 422
    severity: error
    message_zh: 'API Key 请求被拒绝'
    message_en: 'API Key request was rejected'
    related_fields: []
    related_stories: [US-H1-006]
    introduced_in: v3.1

  # ========== H2 审计与事件总线 ==========
  - code: H2_AUDIT_APPEND_ONLY_VIOLATION
    module: H2
    category: AUDIT
    detail: APPEND_ONLY_VIOLATION
    http_status: 403
    severity: critical
    message_zh: '审计记录禁止 UPDATE/DELETE'
    message_en: 'Audit records are append-only'
    related_fields: []
    related_stories: [US-H2-003]
    introduced_in: v3.1

  - code: H2_AUDIT_QUERY_CURSOR_INVALID
    module: H2
    category: AUDIT
    detail: QUERY_CURSOR_INVALID
    http_status: 400
    severity: error
    message_zh: '审计查询游标格式无效'
    message_en: 'Audit query cursor is invalid'
    related_fields: []
    related_stories: [US-H2-002]
    introduced_in: v3.1

  - code: H2_AUDIT_QUERY_FAILED
    module: H2
    category: AUDIT
    detail: QUERY_FAILED
    http_status: 500
    severity: error
    message_zh: '审计查询失败'
    message_en: 'Audit query failed'
    related_fields: []
    related_stories: [US-H2-002]
    introduced_in: v3.1

  - code: H2_EVENT_DLQ_OVERFLOW
    module: H2
    category: EVENT
    detail: DLQ_OVERFLOW
    http_status: 503
    severity: critical
    message_zh: '事件死信队列积压超阈值'
    message_en: 'Event dead-letter queue overflow'
    related_fields: []
    related_stories: [US-H2-005]
    introduced_in: v3.1

  - code: H2_EVENT_HANDLER_TIMEOUT
    module: H2
    category: EVENT
    detail: HANDLER_TIMEOUT
    http_status: 504
    severity: error
    message_zh: '事件处理超时'
    message_en: 'Event handler timeout'
    related_fields: []
    related_stories: [US-H2-005]
    introduced_in: v3.1

  # ========== H4 企微通知 ==========
  - code: H4_NOTIFY_SEND_FAILED
    module: H4
    category: NOTIFY
    detail: SEND_FAILED
    http_status: 502
    severity: error
    message_zh: '通知发送失败'
    message_en: 'Notification send failed'
    related_fields: []
    related_stories: [US-H4-001]
    introduced_in: v3.1

  - code: H4_NOTIFY_TEMPLATE_NOT_FOUND
    module: H4
    category: NOTIFY
    detail: TEMPLATE_NOT_FOUND
    http_status: 404
    severity: warning
    message_zh: '通知模板不存在'
    message_en: 'Notification template not found'
    related_fields: []
    related_stories: [US-H4-001]
    introduced_in: v3.1

  - code: H4_IDEMPOTENCY_REQUIRED
    module: H4
    category: IDEMPOTENCY
    detail: REQUIRED
    http_status: 400
    severity: error
    message_zh: '缺少 Idempotency-Key'
    message_en: 'Idempotency-Key is required'
    related_fields: []
    related_stories: [US-H4-001, US-H4-002, US-H4-003, US-H4-004]
    introduced_in: v3.1

  - code: H4_EVENT_NOT_FOUND
    module: H4
    category: EVENT
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '通知事件未配置或未启用'
    message_en: 'Notification event is missing or disabled'
    related_fields: []
    related_stories: [US-H4-001, US-H4-002]
    introduced_in: v3.1

  - code: H4_WECHAT_SETTINGS_NOT_FOUND
    module: H4
    category: WECHAT
    detail: SETTINGS_NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '企业微信参数未配置'
    message_en: 'WeCom settings not found'
    related_fields: []
    related_stories: [US-H4-002]
    introduced_in: v3.1

  - code: H4_TEMPLATE_INVALID
    module: H4
    category: TEMPLATE
    detail: INVALID
    http_status: 422
    severity: error
    message_zh: '通知模板变量无法渲染'
    message_en: 'Notification template variables cannot be rendered'
    related_fields: []
    related_stories: [US-H4-001, US-H4-002]
    introduced_in: v3.1

  - code: H4_NO_RECIPIENTS
    module: H4
    category: RECIPIENTS
    detail: EMPTY
    http_status: 422
    severity: error
    message_zh: '通知接收人为空'
    message_en: 'Notification recipients are empty'
    related_fields: []
    related_stories: [US-H4-001, US-H4-002]
    introduced_in: v3.1

  - code: H4_APPROVAL_NOT_FOUND
    module: H4
    category: APPROVAL
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '审批记录不存在'
    message_en: 'Approval record not found'
    related_fields: []
    related_stories: [US-H4-003]
    introduced_in: v3.1

  - code: H4_APPROVAL_STATUS_INVALID
    module: H4
    category: APPROVAL
    detail: STATUS_INVALID
    http_status: 422
    severity: error
    message_zh: '审批结论非法'
    message_en: 'Approval status is invalid'
    related_fields: []
    related_stories: [US-H4-003]
    introduced_in: v3.1

  - code: H4_IDEMPOTENCY_CONFLICT
    module: H4
    category: IDEMPOTENCY
    detail: CONFLICT
    http_status: 409
    severity: error
    message_zh: '幂等键已被不同请求使用'
    message_en: 'Idempotency-Key was used by a different request'
    related_fields: []
    related_stories: [US-H4-001, US-H4-002, US-H4-003, US-H4-004]
    introduced_in: v3.1

  - code: H4_REQUEST_INVALID
    module: H4
    category: REQUEST
    detail: INVALID
    http_status: 422
    severity: error
    message_zh: 'H4 请求非法'
    message_en: 'H4 request is invalid'
    related_fields: []
    related_stories: [US-H4-001, US-H4-002, US-H4-003, US-H4-004]
    introduced_in: v3.1

  - code: H4_NOTIFY_INTERNAL
    module: H4
    category: NOTIFY
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: 'H4 通知处理失败'
    message_en: 'H4 notification processing failed'
    related_fields: []
    related_stories: [US-H4-001, US-H4-002, US-H4-003, US-H4-004]
    introduced_in: v3.1

  - code: H4_RECORD_NOT_FOUND
    module: H4
    category: RECORD
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '通知记录不存在'
    message_en: 'Notification record not found'
    related_fields: []
    related_stories: [US-H4-004]
    introduced_in: v3.1

  - code: H4_RECORD_NOT_RESENDABLE
    module: H4
    category: RECORD
    detail: NOT_RESENDABLE
    http_status: 422
    severity: error
    message_zh: '仅失败或重试中的通知可以重发'
    message_en: 'Only failed or retrying notifications can be resent'
    related_fields: []
    related_stories: [US-H4-004]
    introduced_in: v3.1

  # ========== H6 状态机 ==========
  - code: H6_INVALID_TRANSITION_QUERY
    module: H6
    category: INVALID
    detail: TRANSITION_QUERY
    http_status: 400
    severity: error
    message_zh: '缺少或无法解析状态转换查询参数'
    message_en: 'Transition query parameters are missing or invalid'
    related_fields: []
    related_stories: [US-H6-001]
    introduced_in: v3.1

  - code: H6_STATE_MACHINE_NOT_FOUND
    module: H6
    category: STATE_MACHINE
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '状态机定义不存在'
    message_en: 'State machine definition not found'
    related_fields: []
    related_stories: [US-H6-001]
    introduced_in: v3.1

  # ========== H5 快递面单 ==========
  - code: H5_PRINTER_OFFLINE
    module: H5
    category: PRINTER
    detail: OFFLINE
    http_status: 503
    severity: warning
    message_zh: '打印机离线'
    message_en: 'Printer offline'
    related_fields: []
    related_stories: [US-H5-003]
    introduced_in: v3.1

  - code: H5_CARRIER_API_FAILED
    module: H5
    category: CARRIER
    detail: API_FAILED
    http_status: 502
    severity: error
    message_zh: '快递商接口调用失败'
    message_en: 'Carrier API call failed'
    related_fields: []
    related_stories: [US-H5-004]
    introduced_in: v3.1

  # ========== H-DOCK 月台预约 ==========
  - code: H_DOCK_APPOINTMENT_CONFLICT
    module: H_DOCK
    category: APPOINTMENT
    detail: TIME_CONFLICT
    http_status: 409
    severity: warning
    message_zh: '月台预约时间窗冲突'
    message_en: 'Dock appointment time conflict'
    related_fields: []
    related_stories: [US-DOCK-003]
    introduced_in: v3.1

  - code: H_DOCK_VEHICLE_TYPE_MISMATCH
    module: H_DOCK
    category: APPOINTMENT
    detail: VEHICLE_TYPE_MISMATCH
    http_status: 422
    severity: warning
    message_zh: '车辆类型与月台温区不匹配'
    message_en: 'Vehicle type does not match dock temperature zone'
    related_fields: []
    related_stories: [US-DOCK-002, US-DOCK-005]
    introduced_in: v3.1

  - code: H_DOCK_NO_AVAILABLE_DOCK
    module: H_DOCK
    category: APPOINTMENT
    detail: NO_AVAILABLE_DOCK
    http_status: 409
    severity: warning
    message_zh: '所选时段无可用月台'
    message_en: 'No available dock for the selected time window'
    related_fields: []
    related_stories: [US-DOCK-002]
    introduced_in: v3.1

  - code: H_DOCK_IDEMPOTENCY_REQUIRED
    module: H_DOCK
    category: IDEMPOTENCY
    detail: REQUIRED
    http_status: 400
    severity: error
    message_zh: '缺少 Idempotency-Key'
    message_en: 'Idempotency-Key is required'
    related_fields: []
    related_stories: [US-DOCK-002]
    introduced_in: v3.2

  - code: H_DOCK_DOCK_NOT_FOUND
    module: H_DOCK
    category: APPOINTMENT
    detail: DOCK_NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '月台或仓库不存在'
    message_en: 'Dock or warehouse was not found'
    related_fields: [仓库 ID, 月台号]
    related_stories: [US-DOCK-002]
    introduced_in: v3.2

  - code: H_DOCK_IDEMPOTENCY_CONFLICT
    module: H_DOCK
    category: IDEMPOTENCY
    detail: CONFLICT
    http_status: 409
    severity: error
    message_zh: '幂等键已被不同请求使用'
    message_en: 'Idempotency-Key was used by a different request'
    related_fields: []
    related_stories: [US-DOCK-002]
    introduced_in: v3.2

  - code: H_DOCK_APPOINTMENT_INVALID
    module: H_DOCK
    category: APPOINTMENT
    detail: INVALID
    http_status: 422
    severity: error
    message_zh: '预约字段或时间窗非法'
    message_en: 'Dock appointment fields or time window are invalid'
    related_fields: []
    related_stories: [US-DOCK-002]
    introduced_in: v3.2

  - code: H_DOCK_PERSISTENCE_FAILED
    module: H_DOCK
    category: STORAGE
    detail: PERSISTENCE_FAILED
    http_status: 500
    severity: error
    message_zh: '月台预约持久化失败'
    message_en: 'Dock appointment persistence failed'
    related_fields: []
    related_stories: [US-DOCK-002]
    introduced_in: v3.2

  - code: H_DOCK_APPOINTMENT_NOT_FOUND
    module: H_DOCK
    category: APPOINTMENT
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '预约不存在'
    message_en: 'Dock appointment was not found'
    related_fields: []
    related_stories: [US-DOCK-005]
    introduced_in: v3.3

  - code: H_DOCK_ARRIVAL_CHECK_FAILED
    module: H_DOCK
    category: APPOINTMENT
    detail: ARRIVAL_CHECK_FAILED
    http_status: 409
    severity: error
    message_zh: '预约到达核对不一致'
    message_en: 'Dock appointment arrival check failed'
    related_fields: []
    related_stories: [US-DOCK-005]
    introduced_in: v3.3

  - code: H_DOCK_APPOINTMENT_NOT_ARRIVABLE
    module: H_DOCK
    category: APPOINTMENT
    detail: NOT_ARRIVABLE
    http_status: 409
    severity: error
    message_zh: '预约当前状态不允许到达核对'
    message_en: 'Dock appointment cannot be marked as arrived in its current state'
    related_fields: []
    related_stories: [US-DOCK-005]
    introduced_in: v3.3

  # ========== H-AL 告警引擎 ==========
  - code: HAL_IDEMPOTENCY_REQUIRED
    module: H_AL
    category: IDEMPOTENCY
    detail: REQUIRED
    http_status: 400
    severity: warning
    message_zh: '缺少告警定义变更幂等键'
    message_en: 'Alert definition change idempotency key is required'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_NOT_FOUND
    module: H_AL
    category: ALERT
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '告警定义不存在'
    message_en: 'Alert definition was not found'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_DUPLICATE
    module: H_AL
    category: ALERT
    detail: DUPLICATE
    http_status: 409
    severity: warning
    message_zh: '告警编码或名称已存在'
    message_en: 'Alert code or name already exists'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_STALE
    module: H_AL
    category: ALERT
    detail: STALE
    http_status: 409
    severity: warning
    message_zh: '告警定义版本已过期'
    message_en: 'Alert definition version is stale'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_IN_USE
    module: H_AL
    category: ALERT
    detail: IN_USE
    http_status: 409
    severity: warning
    message_zh: '告警定义已有触发记录，不能删除'
    message_en: 'Alert definition has trigger history and cannot be deleted'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_GSP_REQUIRED
    module: H_AL
    category: ALERT
    detail: GSP_REQUIRED
    http_status: 422
    severity: critical
    message_zh: 'GSP 强制告警不能停用或删除'
    message_en: 'GSP-mandatory alert cannot be disabled or deleted'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_DISABLE_NOT_ALLOWED
    module: H_AL
    category: ALERT
    detail: DISABLE_NOT_ALLOWED
    http_status: 422
    severity: warning
    message_zh: '该告警定义不允许停用'
    message_en: 'This alert definition cannot be disabled'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_CONDITION_INVALID
    module: H_AL
    category: CONDITION
    detail: INVALID
    http_status: 422
    severity: warning
    message_zh: '告警触发条件必须是有效 JSON'
    message_en: 'Alert condition must be valid JSON'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_INVALID
    module: H_AL
    category: ALERT
    detail: INVALID
    http_status: 422
    severity: warning
    message_zh: '告警定义字段或变更结构非法'
    message_en: 'Alert definition fields or change shape are invalid'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_APPROVAL_NOT_CONFIGURED
    module: H_AL
    category: APPROVAL
    detail: NOT_CONFIGURED
    http_status: 422
    severity: error
    message_zh: '未配置告警定义变更审批类型'
    message_en: 'Alert definition change approval type is not configured'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_IDEMPOTENCY_CONFLICT
    module: H_AL
    category: IDEMPOTENCY
    detail: CONFLICT
    http_status: 409
    severity: warning
    message_zh: '幂等键已被不同的告警变更请求使用'
    message_en: 'Idempotency key was used by a different alert change request'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ALERT_INTERNAL
    module: H_AL
    category: ALERT
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: '告警处理失败'
    message_en: 'Alert processing failed'
    related_fields: []
    related_stories: [US-AL-001, US-AL-002]
    introduced_in: v3.9

  - code: HAL_CHANNEL_NOT_FOUND
    module: H_AL
    category: CHANNEL
    detail: NOT_FOUND
    http_status: 422
    severity: error
    message_zh: '告警通知通道不存在或未启用'
    message_en: 'Alert notification channel was not found or is disabled'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.9

  - code: HAL_ESCALATION_RULE_NOT_FOUND
    module: H_AL
    category: ESCALATION
    detail: RULE_NOT_FOUND
    http_status: 422
    severity: warning
    message_zh: '告警升级规则不存在或未启用'
    message_en: 'Alert escalation rule was not found or is disabled'
    related_fields: []
    related_stories: [US-AL-001, US-AL-003]
    introduced_in: v3.9

  - code: HAL_ALERT_INSTANCE_NOT_FOUND
    module: H_AL
    category: ALERT
    detail: INSTANCE_NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '告警实例不存在'
    message_en: 'Alert instance was not found'
    related_fields: []
    related_stories: [US-AL-002]
    introduced_in: v3.9

  - code: HAL_ALERT_STATUS_INVALID
    module: H_AL
    category: ALERT
    detail: STATUS_INVALID
    http_status: 409
    severity: warning
    message_zh: '当前告警状态不允许执行该操作'
    message_en: 'The current alert status does not allow this action'
    related_fields: []
    related_stories: [US-AL-002]
    introduced_in: v3.9

  - code: HAL_ALERT_REASON_REQUIRED
    module: H_AL
    category: ALERT
    detail: REASON_REQUIRED
    http_status: 422
    severity: warning
    message_zh: '关闭或忽略告警必须填写原因'
    message_en: 'Closing or ignoring an alert requires a reason'
    related_fields: []
    related_stories: [US-AL-002]
    introduced_in: v3.9

  - code: HAL_ESCALATION_LEVEL_LIMIT
    module: H_AL
    category: ESCALATION
    detail: LEVEL_LIMIT
    http_status: 422
    severity: warning
    message_zh: '告警升级规则最多允许三级'
    message_en: 'An alert escalation rule supports at most three levels'
    related_fields: []
    related_stories: [US-AL-003]
    introduced_in: v3.9

  - code: HAL_ESCALATION_INVALID
    module: H_AL
    category: ESCALATION
    detail: INVALID
    http_status: 422
    severity: warning
    message_zh: '告警升级规则格式或阈值非法'
    message_en: 'Alert escalation rule shape or threshold is invalid'
    related_fields: []
    related_stories: [US-AL-003]
    introduced_in: v3.9

  - code: HAL_ESCALATION_INTERNAL
    module: H_AL
    category: ESCALATION
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: '告警升级处理失败'
    message_en: 'Alert escalation processing failed'
    related_fields: []
    related_stories: [US-AL-003]
    introduced_in: v3.9

  - code: HAL_EXPORT_NOT_FOUND
    module: H_AL
    category: EXPORT
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '告警报表导出任务或下载链接不存在'
    message_en: 'Alert report export job or download link was not found'
    related_fields: []
    related_stories: [US-AL-004]
    introduced_in: v3.9

  - code: HAL_QUERY_RANGE_TOO_LARGE
    module: H_AL
    category: QUERY
    detail: RANGE_TOO_LARGE
    http_status: 422
    severity: warning
    message_zh: '告警统计查询时间范围过大'
    message_en: 'Alert statistics query range is too large'
    related_fields: []
    related_stories: [US-AL-004]
    introduced_in: v3.9

  - code: HAL_WAREHOUSE_SCOPE_REQUIRED
    module: H_AL
    category: AUTHORIZATION
    detail: WAREHOUSE_SCOPE_REQUIRED
    http_status: 422
    severity: warning
    message_zh: '必须选择已授权的仓库范围'
    message_en: 'An authorized warehouse scope must be selected'
    related_fields: []
    related_stories: [US-AL-004]
    introduced_in: v3.9

  - code: HAL_WAREHOUSE_SCOPE_DENIED
    module: H_AL
    category: AUTHORIZATION
    detail: WAREHOUSE_SCOPE_DENIED
    http_status: 403
    severity: critical
    message_zh: '无权查询该仓库的告警数据'
    message_en: 'The user cannot query alert data for this warehouse'
    related_fields: []
    related_stories: [US-AL-004]
    introduced_in: v3.9

  - code: HAL_EXPORT_FORMAT_INVALID
    module: H_AL
    category: EXPORT
    detail: FORMAT_INVALID
    http_status: 422
    severity: warning
    message_zh: '告警报表仅支持 Excel 或 PDF 格式'
    message_en: 'Alert reports support only Excel or PDF format'
    related_fields: []
    related_stories: [US-AL-004]
    introduced_in: v3.9

  - code: HAL_DASHBOARD_INTERNAL
    module: H_AL
    category: DASHBOARD
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: '告警看板或报表处理失败'
    message_en: 'Alert dashboard or report processing failed'
    related_fields: []
    related_stories: [US-AL-004]
    introduced_in: v3.9

  - code: H_AL_GSP_ALERT_DISABLE_DENIED
    module: H_AL
    category: ALERT
    detail: GSP_DISABLE_DENIED
    http_status: 403
    severity: critical
    message_zh: 'GSP 强制告警不可停用'
    message_en: 'GSP-mandatory alert cannot be disabled'
    related_fields: []
    related_stories: [US-AL-001]
    introduced_in: v3.1

  - code: H_AL_ESCALATION_NO_HANDLER
    module: H_AL
    category: ALERT
    detail: ESCALATION_NO_HANDLER
    http_status: 422
    severity: error
    message_zh: '告警升级链无可用接收人'
    message_en: 'No available handler in escalation chain'
    related_fields: []
    related_stories: [US-AL-003]
    introduced_in: v3.1

  # ========== M1 主数据 ==========
  - code: M1_PRODUCT_CODE_DUPLICATE
    module: M1
    category: PRODUCT
    detail: CODE_DUPLICATE
    http_status: 409
    severity: warning
    message_zh: '商品编码已存在'
    message_en: 'Product code already exists'
    related_fields: [商品编码]
    related_stories: [US-M1-001]
    introduced_in: v3.1

  - code: M1_LOCATION_HAS_STOCK
    module: M1
    category: LOCATION
    detail: HAS_STOCK
    http_status: 422
    severity: warning
    message_zh: '库位仍有库存，不能停用'
    message_en: 'Location with stock cannot be disabled'
    related_fields: []
    related_stories: [US-M1-004]
    introduced_in: v25

  - code: M1_LOCATION_CAPACITY_INVALID
    module: M1
    category: LOCATION
    detail: CAPACITY_INVALID
    http_status: 422
    severity: warning
    message_zh: '库位已用容积不能超过最大容积'
    message_en: 'Used location volume cannot exceed maximum volume'
    related_fields: []
    related_stories: [US-M1-004]
    introduced_in: v25

  - code: M1_LOCATION_OWNER_INVALID
    module: M1
    category: LOCATION
    detail: OWNER_INVALID
    http_status: 422
    severity: warning
    message_zh: '库位绑定货主不存在'
    message_en: 'Bound location owner does not exist'
    related_fields: []
    related_stories: [US-M1-004]
    introduced_in: v25

  - code: M1_WAREHOUSE_TYPE_INVALID
    module: M1
    category: WAREHOUSE
    detail: TYPE_INVALID
    http_status: 422
    severity: warning
    message_zh: '仓库类型必须是物理仓、逻辑仓或虚拟仓'
    message_en: 'Warehouse type must be physical, logical, or virtual'
    related_fields: []
    related_stories: [US-M1-004]
    introduced_in: v25

  - code: M1_PRODUCT_APPROVAL_NO_INVALID
    module: M1
    category: PRODUCT
    detail: APPROVAL_NO_INVALID
    http_status: 422
    severity: error
    message_zh: '批准文号格式错误'
    message_en: 'Approval number format invalid'
    related_fields: [批准文号]
    related_stories: [US-M1-001]
    introduced_in: v3.1

  - code: M1_SUPPLIER_GSP_EXPIRED
    module: M1
    category: SUPPLIER
    detail: GSP_EXPIRED
    http_status: 422
    severity: error
    message_zh: '供应商 GSP 资质已过期'
    message_en: 'Supplier GSP certificate expired'
    related_fields: [GSP 证]
    related_stories: [US-M1-002]
    introduced_in: v3.1

  - code: M1_USCC_FORMAT_INVALID
    module: M1
    category: USCC
    detail: FORMAT_INVALID
    http_status: 422
    severity: error
    message_zh: '统一社会信用代码格式或校验码错误'
    message_en: 'USCC format or checksum invalid'
    related_fields: [delivery_org_uscc, shipper_org_uscc, receiver_org_uscc]
    related_stories: [US-M1-002, US-M1-005]
    introduced_in: v3.1

  - code: M1_INVOICE_DEFAULT_NOT_UNIQUE
    module: M1
    category: INVOICE
    detail: DEFAULT_NOT_UNIQUE
    http_status: 422
    severity: warning
    message_zh: '同客商默认开票信息必须有且仅有 1 个'
    message_en: 'Default invoice info must be unique per customer'
    related_fields: []
    related_stories: [US-M1-005]
    introduced_in: v3.1

  - code: M1_CONFIG_FLAG_MISSING
    module: M1
    category: CONFIG
    detail: FLAG_MISSING
    http_status: 404
    severity: error
    message_zh: 'Feature Flag 不存在'
    message_en: 'Feature Flag missing'
    related_fields: []
    related_stories: [US-M1-008]
    introduced_in: W6.C

  - code: M1_CONFIG_FLAG_DISABLED
    module: M1
    category: CONFIG
    detail: FLAG_DISABLED
    http_status: 404
    severity: error
    message_zh: 'Feature Flag 未启用'
    message_en: 'Feature Flag disabled'
    related_fields: []
    related_stories: [US-M1-008]
    introduced_in: W6.C

  - code: M1_CONFIG_FLAG_SOURCE_INVALID
    module: M1
    category: CONFIG
    detail: FLAG_SOURCE_INVALID
    http_status: 422
    severity: error
    message_zh: 'Feature Flag 读取源无效'
    message_en: 'Feature Flag source invalid'
    related_fields: []
    related_stories: [US-M1-008]
    introduced_in: W6.C

  # ========== M2 入库 ==========
  - code: M2_ASN_DUPLICATE
    module: M2
    category: ASN
    detail: DUPLICATE
    http_status: 409
    severity: warning
    message_zh: 'ASN 单号重复'
    message_en: 'ASN number duplicate'
    related_fields: [ASN 号]
    related_stories: [US-M2-001]
    introduced_in: v3.1

  - code: M2_ARRIVAL_TIME_BEFORE_DEPARTURE
    module: M2
    category: VALIDATION
    detail: ARRIVAL_BEFORE_DEPARTURE
    http_status: 422
    severity: error
    message_zh: '到货时间早于启运时间'
    message_en: 'Arrival time before departure time'
    related_fields: [到货时间, 启运时间]
    related_stories: [US-M2-002]
    introduced_in: v3.1

  - code: M2_TEMPERATURE_OUT_OF_RANGE
    module: M2
    category: COLD_CHAIN
    detail: TEMP_OUT_OF_RANGE
    http_status: 422
    severity: warning
    message_zh: '收货温度超出商品储存条件范围'
    message_en: 'Receiving temperature out of storage condition range'
    related_fields: [到货温度]
    related_stories: [US-M2-002, US-M2-006]
    introduced_in: v3.1

  - code: M2_DUAL_PERSON_REQUIRED
    module: M2
    category: VALIDATION
    detail: DUAL_PERSON_REQUIRED
    http_status: 422
    severity: error
    message_zh: '该商品需双人验收'
    message_en: 'Dual-person validation required'
    related_fields: [验收员 user_id]
    related_stories: [US-M2-003, US-M2-004]
    introduced_in: v3.1

  - code: M2_DUAL_PERSON_SAME_USER
    module: M2
    category: VALIDATION
    detail: DUAL_PERSON_SAME_USER
    http_status: 422
    severity: critical
    message_zh: '双人验收的两人不能为同一用户'
    message_en: 'Dual-person validators cannot be the same user'
    related_fields: [验收员 user_id]
    related_stories: [US-M2-004]
    introduced_in: v3.1

  - code: M2_DUAL_PERSON_APPROVAL_REQUIRED
    module: M2
    category: APPROVAL
    detail: DUAL_PERSON_APPROVAL_REQUIRED
    http_status: 422
    severity: error
    message_zh: 'M-VR 策略要求先完成主管审批'
    message_en: 'Supervisor approval required by M-VR dual-person policy'
    related_fields: [双人策略命中规则 ID]
    related_stories: [US-M2-004, US-VR-006]
    introduced_in: v3.2

  - code: M2_VERIFIER_UNAUTHORIZED
    module: M2
    category: AUTHORIZATION
    detail: VERIFIER_UNAUTHORIZED
    http_status: 422
    severity: error
    message_zh: '签字人不是当前货主的有效验收岗用户'
    message_en: 'Signer is not an active receiving verifier for the owner'
    related_fields: [验收员 user_id, 货主]
    related_stories: [US-M2-004]
    introduced_in: v3.2

  - code: M2_QUANTITY_MISMATCH
    module: M2
    category: VALIDATION
    detail: QUANTITY_MISMATCH
    http_status: 422
    severity: warning
    message_zh: '实到数量 + 缺货 + 拒收 ≠ 预报数量'
    message_en: 'Actual + missing + rejected != expected'
    related_fields: [预报数量, 实际到货数量, 缺货数量, 拒收数量]
    related_stories: [US-M2-002]
    introduced_in: v3.1

  # ========== M3 库存 ==========
  - code: M3_INVENTORY_INSUFFICIENT_QTY
    module: M3
    category: INVENTORY
    detail: INSUFFICIENT_QTY
    http_status: 409
    severity: warning
    message_zh: '库存不足'
    message_en: 'Insufficient stock'
    related_fields: [可用数量]
    related_stories: [US-M3-001, US-M4-002]
    introduced_in: v3.1

  - code: M3_BATCH_EXPIRED
    module: M3
    category: BATCH
    detail: EXPIRED
    http_status: 409
    severity: error
    message_zh: '批号已过期，不可出库'
    message_en: 'Batch expired, cannot be shipped'
    related_fields: [批号, 有效期]
    related_stories: [US-M3-002]
    introduced_in: v3.1

  - code: M3_BATCH_RECALLED
    module: M3
    category: BATCH
    detail: RECALLED
    http_status: 409
    severity: critical
    message_zh: '批号已召回'
    message_en: 'Batch recalled'
    related_fields: [批号, 召回标记]
    related_stories: [US-M3-002]
    introduced_in: v3.1

  - code: M3_STATUS_NOT_QUALIFIED
    module: M3
    category: STATUS
    detail: NOT_QUALIFIED
    http_status: 409
    severity: error
    message_zh: '库存状态非合格，不可出库'
    message_en: 'Inventory not in qualified status'
    related_fields: [库存状态]
    related_stories: [US-M3-003]
    introduced_in: v3.1

  - code: M3_BATCH_NOT_FOUND
    module: M3
    category: BATCH
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: 'ERP 指定批号在 WMS 不存在'
    message_en: 'Batch specified by ERP not found in WMS'
    related_fields: [批号]
    related_stories: [US-M4-008]
    introduced_in: v3.1

  - code: M3_LOCATION_LOCKED
    module: M3
    category: LOCATION
    detail: LOCKED
    http_status: 409
    severity: warning
    message_zh: '库位已锁定（盘点中）'
    message_en: 'Location locked (stocktake in progress)'
    related_fields: [库位]
    related_stories: [US-M3-005]
    introduced_in: v3.1

  - code: M3_APPROVAL_SOURCE_REQUIRED
    module: M3
    category: STATUS
    detail: APPROVAL_SOURCE_REQUIRED
    http_status: 422
    severity: error
    message_zh: '状态变更必须填写审批源'
    message_en: 'Approval source required for status change'
    related_fields: [审批源]
    related_stories: [US-M3-003]
    introduced_in: v3.1

  - code: M3_NEAR_EXPIRY_WARNING
    module: M3
    category: BATCH
    detail: NEAR_EXPIRY
    http_status: 200
    severity: info
    message_zh: '批号即将过期'
    message_en: 'Batch is near expiry'
    related_fields: [批号, 有效期, expire_warning_days]
    related_stories: [US-M3-002]
    introduced_in: v3.1

  # ========== M4 出库 ==========
  - code: M4_ORDER_BATCH_NOT_SPECIFIED
    module: M4
    category: ORDER
    detail: BATCH_NOT_SPECIFIED
    http_status: 422
    severity: error
    message_zh: '出库订单未指定批号（应由 ERP 指定）'
    message_en: 'Outbound order missing batch (should be specified by ERP)'
    related_fields: [批号]
    related_stories: [US-M4-001]
    introduced_in: v3.1

  - code: M4_PICKER_VERIFIER_SAME_USER
    module: M4
    category: VALIDATION
    detail: PICKER_VERIFIER_SAME
    http_status: 422
    severity: critical
    message_zh: '拣选人不能与复核人为同一用户'
    message_en: 'Picker and verifier cannot be the same user'
    related_fields: [拣选人 user_id, 复核人 user_id]
    related_stories: [US-M4-003, US-M4-004]
    introduced_in: v3.1

  - code: M4_DUAL_PERSON_REQUIRED
    module: M4
    category: VALIDATION
    detail: DUAL_PERSON_REQUIRED
    http_status: 422
    severity: error
    message_zh: 'M-VR 策略要求第二复核员'
    message_en: 'Second reviewer required by M-VR policy'
    related_fields: [验收员 user_id, 双人策略命中规则 ID]
    related_stories: [US-M4-004, US-VR-006]
    introduced_in: v3.2

  - code: M4_SECOND_REVIEWER_UNAUTHORIZED
    module: M4
    category: AUTHORIZATION
    detail: SECOND_REVIEWER_UNAUTHORIZED
    http_status: 422
    severity: error
    message_zh: '第二复核员不是当前货主的有效保管员'
    message_en: 'Second reviewer is not an active custodian for the owner'
    related_fields: [验收员 user_id, 货主]
    related_stories: [US-M4-004, US-VR-006]
    introduced_in: v3.2

  - code: M4_DUAL_PERSON_APPROVAL_REQUIRED
    module: M4
    category: APPROVAL
    detail: DUAL_PERSON_APPROVAL_REQUIRED
    http_status: 422
    severity: error
    message_zh: 'M-VR 策略要求先完成主管审批'
    message_en: 'Supervisor approval required by M-VR dual-person policy'
    related_fields: [双人策略命中规则 ID]
    related_stories: [US-M4-004, US-VR-006]
    introduced_in: v3.2

  - code: M4_WAVE_RELEASED_TWICE
    module: M4
    category: WAVE
    detail: ALREADY_RELEASED
    http_status: 409
    severity: warning
    message_zh: '波次已下发，不可重复释放'
    message_en: 'Wave already released'
    related_fields: [拣货单号]
    related_stories: [US-M4-002]
    introduced_in: v3.1

  - code: M4_PARTIAL_PICKING_NOT_ALLOWED
    module: M4
    category: PICKING
    detail: PARTIAL_NOT_ALLOWED
    http_status: 422
    severity: warning
    message_zh: '本订单不允许部分拣选'
    message_en: 'Partial picking not allowed for this order'
    related_fields: []
    related_stories: [US-M4-003]
    introduced_in: v3.1

  - code: M4_RETURN_REASON_INVALID
    module: M4
    category: RETURN
    detail: REASON_INVALID
    http_status: 422
    severity: warning
    message_zh: '退货原因不在配置清单中'
    message_en: 'Return reason not in configured list'
    related_fields: []
    related_stories: [US-M4-008]
    introduced_in: v3.1

  - code: M4_OFFLINE_CONFLICT
    module: M4
    category: SYNC
    detail: OFFLINE_CONFLICT
    http_status: 409
    severity: warning
    message_zh: '离线提交检测到冲突，已转人工处理'
    message_en: 'Offline submission conflict detected, escalated to manager'
    related_fields: []
    related_stories: [US-M4-003]
    introduced_in: v3.1

  - code: M4_DELIVERY_ADDRESS_MISSING
    module: M4
    category: VALIDATION
    detail: ADDRESS_MISSING
    http_status: 422
    severity: error
    message_zh: '配送地址缺失'
    message_en: 'Delivery address missing'
    related_fields: [配送地址]
    related_stories: [US-M4-001, US-M4-006]
    introduced_in: v3.1

  # ========== M-VR 校验规则 ==========
  - code: M_VR_RULE_REJECTED
    module: M_VR
    category: VALIDATION
    detail: RULE_REJECTED
    http_status: 422
    severity: warning
    message_zh: '规则引擎拦截'
    message_en: 'Validation rule rejected'
    related_fields: []
    related_stories: [US-VR-002]
    introduced_in: v3.1

  - code: M_VR_FORCE_PASS_REQUIRED
    module: M_VR
    category: VALIDATION
    detail: FORCE_PASS_REQUIRED
    http_status: 403
    severity: error
    message_zh: '强制通过需要主管审批'
    message_en: 'Force pass requires supervisor approval'
    related_fields: []
    related_stories: [US-VR-002]
    introduced_in: v3.1

  - code: M_VR_DUAL_PERSON_POLICY_INVALID
    module: M_VR
    category: VALIDATION
    detail: DUAL_PERSON_POLICY_INVALID
    http_status: 422
    severity: error
    message_zh: '双人策略流程、节点或规则参数非法'
    message_en: 'Invalid process, node, or dual-person policy rule'
    related_fields: [特殊药品分类, 双人策略命中规则 ID]
    related_stories: [US-VR-006]
    introduced_in: v3.2

  - code: M_VR_DUAL_PERSON_IDEMPOTENCY_REQUIRED
    module: M_VR
    category: DUAL_PERSON
    detail: IDEMPOTENCY_REQUIRED
    http_status: 400
    severity: error
    message_zh: '双人策略写操作缺少幂等键'
    message_en: 'Dual-person policy write requires an idempotency key'
    related_fields: []
    related_stories: [US-VR-006]
    introduced_in: v3.7

  - code: M_VR_DUAL_PERSON_CROSS_OWNER
    module: M_VR
    category: DUAL_PERSON
    detail: CROSS_OWNER
    http_status: 403
    severity: critical
    message_zh: '跨货主访问双人策略被拒绝'
    message_en: 'Cross-owner dual-person policy access was denied'
    related_fields: []
    related_stories: [US-VR-006]
    introduced_in: v3.7

  - code: M_VR_DUAL_PERSON_SAME_PERSON
    module: M_VR
    category: DUAL_PERSON
    detail: SAME_PERSON
    http_status: 422
    severity: error
    message_zh: '双人策略变更的操作人和确认人不能相同'
    message_en: 'Dual-person policy operator and confirmer must differ'
    related_fields: []
    related_stories: [US-VR-006]
    introduced_in: v3.7

  - code: M_VR_DUAL_PERSON_UNQUALIFIED
    module: M_VR
    category: DUAL_PERSON
    detail: UNQUALIFIED
    http_status: 422
    severity: error
    message_zh: '双人策略变更确认人不具备对应资格'
    message_en: 'Dual-person policy confirmer lacks the required qualification'
    related_fields: []
    related_stories: [US-VR-006]
    introduced_in: v3.7

  - code: M_VR_DUAL_PERSON_REFERENCE_NOT_FOUND
    module: M_VR
    category: DUAL_PERSON
    detail: REFERENCE_NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '双人策略关联的商品、仓库或特殊药品分类不存在'
    message_en: 'A product, warehouse, or special-drug category referenced by the policy was not found'
    related_fields: [特殊药品分类]
    related_stories: [US-VR-006]
    introduced_in: v3.7

  - code: M_VR_DUAL_PERSON_IDEMPOTENCY_CONFLICT
    module: M_VR
    category: DUAL_PERSON
    detail: IDEMPOTENCY_CONFLICT
    http_status: 409
    severity: error
    message_zh: '双人策略幂等键已被不同请求使用'
    message_en: 'Dual-person policy idempotency key was used by a different request'
    related_fields: []
    related_stories: [US-VR-006]
    introduced_in: v3.7

  - code: M_VR_DUAL_PERSON_INTERNAL
    module: M_VR
    category: DUAL_PERSON
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: '双人策略处理失败'
    message_en: 'Dual-person policy processing failed'
    related_fields: []
    related_stories: [US-VR-006]
    introduced_in: v3.7

  # ========== M-QL 质量联系单 ==========
  - code: M_QL_TIMEOUT_EXCEEDED
    module: M_QL
    category: APPROVAL
    detail: TIMEOUT_EXCEEDED
    http_status: 200
    severity: warning
    message_zh: '质量联系单审批超时'
    message_en: 'Quality liaison timeout exceeded'
    related_fields: []
    related_stories: [US-QL-003]
    introduced_in: v3.1

  # ========== M-TC 追溯码 ==========
  - code: M_TC_CODE_FORMAT_INVALID
    module: M_TC
    category: TRACE_CODE
    detail: FORMAT_INVALID
    http_status: 422
    severity: error
    message_zh: '追溯码格式错误'
    message_en: 'Trace code format invalid'
    related_fields: [追溯码]
    related_stories: [US-TC-003]
    introduced_in: v3.1

  - code: M_TC_REGULATORY_REPORT_FAILED
    module: M_TC
    category: REGULATORY
    detail: REPORT_FAILED
    http_status: 502
    severity: error
    message_zh: '"码上放心"上报失败'
    message_en: 'Regulatory platform report failed'
    related_fields: [regulatory_report_id]
    related_stories: [US-TC-007]
    introduced_in: v3.1

  # ========== M-PM 参数对照 ==========
  - code: M_PM_FIELD_NOT_MAPPED
    module: M_PM
    category: MAPPING
    detail: NOT_MAPPED
    http_status: 422
    severity: warning
    message_zh: 'ERP 字段未在 M-PM 字典中映射'
    message_en: 'ERP field not mapped in M-PM dictionary'
    related_fields: []
    related_stories: [US-MPM-003]
    introduced_in: v3.1

  # ========== M-DI 药检平台配置 ==========
  - code: M_DI_PLATFORM_IDEMPOTENCY_REQUIRED
    module: M_DI
    category: PLATFORM
    detail: IDEMPOTENCY_REQUIRED
    http_status: 400
    severity: error
    message_zh: '缺少 Idempotency-Key'
    message_en: 'Idempotency-Key is required'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_NOT_FOUND
    module: M_DI
    category: PLATFORM
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '药检平台不存在'
    message_en: 'Drug inspection platform was not found'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_IDEMPOTENCY_CONFLICT
    module: M_DI
    category: PLATFORM
    detail: IDEMPOTENCY_CONFLICT
    http_status: 409
    severity: error
    message_zh: '幂等键已被不同请求使用'
    message_en: 'Idempotency-Key was used by a different request'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_PERSISTENCE_FAILED
    module: M_DI
    category: PLATFORM
    detail: PERSISTENCE_FAILED
    http_status: 500
    severity: error
    message_zh: '药检平台配置持久化或审计失败'
    message_en: 'Drug inspection platform persistence or audit failed'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_FIELD_REQUIRED
    module: M_DI
    category: PLATFORM
    detail: FIELD_REQUIRED
    http_status: 422
    severity: error
    message_zh: '药检平台配置必填字段缺失'
    message_en: 'A required drug inspection platform field is missing'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_FIELD_TOO_LONG
    module: M_DI
    category: PLATFORM
    detail: FIELD_TOO_LONG
    http_status: 422
    severity: error
    message_zh: '药检平台配置字段超长'
    message_en: 'A drug inspection platform field is too long'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_API_URL_INVALID
    module: M_DI
    category: PLATFORM
    detail: API_URL_INVALID
    http_status: 422
    severity: error
    message_zh: 'API 地址必须是带主机的 HTTP 或 HTTPS 地址'
    message_en: 'The API URL must be an HTTP or HTTPS URL with a host'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_AUTH_METHOD_INVALID
    module: M_DI
    category: PLATFORM
    detail: AUTH_METHOD_INVALID
    http_status: 422
    severity: error
    message_zh: '认证方式必须是 API Key 或账号密码'
    message_en: 'Authentication must use an API key or username and password'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_CREDENTIAL_REF_INVALID
    module: M_DI
    category: PLATFORM
    detail: CREDENTIAL_REF_INVALID
    http_status: 422
    severity: error
    message_zh: '认证凭证必须使用 Vault 引用'
    message_en: 'Credentials must use a Vault reference'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_CREDENTIAL_COMBINATION_INVALID
    module: M_DI
    category: PLATFORM
    detail: CREDENTIAL_COMBINATION_INVALID
    http_status: 422
    severity: error
    message_zh: '认证参数与认证方式不匹配'
    message_en: 'Credentials do not match the selected authentication method'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_TIMEOUT_INVALID
    module: M_DI
    category: PLATFORM
    detail: TIMEOUT_INVALID
    http_status: 422
    severity: error
    message_zh: '超时必须在 1 到 300 秒之间'
    message_en: 'Timeout must be between 1 and 300 seconds'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  - code: M_DI_PLATFORM_STATUS_INVALID
    module: M_DI
    category: PLATFORM
    detail: STATUS_INVALID
    http_status: 422
    severity: error
    message_zh: '平台状态必须是 connected、testing 或 disabled'
    message_en: 'Platform status must be connected, testing, or disabled'
    related_fields: []
    related_stories: [US-DI-001]
    introduced_in: v3.4

  # ========== M-TE 任务类型配置 ==========
  - code: M_TE_TASK_TYPE_IDEMPOTENCY_REQUIRED
    module: M_TE
    category: TASK_TYPE
    detail: IDEMPOTENCY_REQUIRED
    http_status: 400
    severity: error
    message_zh: '缺少 Idempotency-Key'
    message_en: 'Idempotency-Key is required'
    related_fields: []
    related_stories: [US-TE-001]
    introduced_in: v3.4

  - code: M_TE_TASK_TYPE_INVALID
    module: M_TE
    category: TASK_TYPE
    detail: INVALID
    http_status: 422
    severity: error
    message_zh: '任务类型配置非法'
    message_en: 'Task type configuration is invalid'
    related_fields: [任务优先级]
    related_stories: [US-TE-001]
    introduced_in: v3.4

  - code: M_TE_TASK_TYPE_NOT_FOUND
    module: M_TE
    category: TASK_TYPE
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '任务类型不存在'
    message_en: 'Task type was not found'
    related_fields: []
    related_stories: [US-TE-001, US-TE-003]
    introduced_in: v3.4

  - code: M_TE_TASK_TYPE_IDEMPOTENCY_CONFLICT
    module: M_TE
    category: TASK_TYPE
    detail: IDEMPOTENCY_CONFLICT
    http_status: 409
    severity: error
    message_zh: '幂等键已被不同请求使用'
    message_en: 'Idempotency-Key was used by a different request'
    related_fields: []
    related_stories: [US-TE-001]
    introduced_in: v3.4

  - code: M_TE_TASK_TYPE_INTERNAL
    module: M_TE
    category: TASK_TYPE
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: '任务类型处理失败'
    message_en: 'Task type processing failed'
    related_fields: []
    related_stories: [US-TE-001]
    introduced_in: v3.4

  # ========== M-TE 任务组、创建、分派与执行 ==========
  - code: M_TE_IDEMPOTENCY_REQUIRED
    module: M_TE
    category: TASK
    detail: IDEMPOTENCY_REQUIRED
    http_status: 400
    severity: error
    message_zh: '缺少 Idempotency-Key'
    message_en: 'Idempotency-Key is required'
    related_fields: []
    related_stories: [US-TE-002, US-TE-003, US-TE-005, US-TE-008]
    introduced_in: v3.6

  - code: M_TE_TASK_INVALID
    module: M_TE
    category: TASK
    detail: INVALID
    http_status: 422
    severity: error
    message_zh: '任务数据非法'
    message_en: 'Warehouse task data is invalid'
    related_fields: []
    related_stories: [US-TE-002, US-TE-003, US-TE-005, US-TE-008]
    introduced_in: v3.6

  - code: M_TE_RULE_INVALID
    module: M_TE
    category: PRIORITY_RULE
    detail: INVALID
    http_status: 422
    severity: error
    message_zh: '优先级规则参数非法'
    message_en: 'Task priority rule parameters are invalid'
    related_fields: []
    related_stories: [US-TE-004]
    introduced_in: v3.7

  - code: M_TE_RELEASE_CONDITION_NOT_MET
    module: M_TE
    category: RELEASE
    detail: CONDITION_NOT_MET
    http_status: 422
    severity: error
    message_zh: '任务释放条件未满足'
    message_en: 'Task release condition is not met'
    related_fields: []
    related_stories: [US-TE-006]
    introduced_in: v3.8

  - code: M_TE_TASK_GROUP_NOT_FOUND
    module: M_TE
    category: TASK_GROUP
    detail: NOT_FOUND
    http_status: 422
    severity: error
    message_zh: '任务组不存在、不适用或未启用'
    message_en: 'Task group was not found, applicable, or enabled'
    related_fields: []
    related_stories: [US-TE-002, US-TE-003]
    introduced_in: v3.6

  - code: M_TE_WAREHOUSE_NOT_FOUND
    module: M_TE
    category: TASK_GROUP
    detail: WAREHOUSE_NOT_FOUND
    http_status: 422
    severity: error
    message_zh: '仓库不存在或未启用'
    message_en: 'Warehouse was not found or enabled'
    related_fields: []
    related_stories: [US-TE-002, US-TE-003]
    introduced_in: v3.6

  - code: M_TE_ZONE_NOT_FOUND
    module: M_TE
    category: TASK_GROUP
    detail: ZONE_NOT_FOUND
    http_status: 422
    severity: error
    message_zh: '任务组库区不存在或不属于指定仓库'
    message_en: 'Task group zone was not found in the selected warehouse'
    related_fields: []
    related_stories: [US-TE-002]
    introduced_in: v3.6

  - code: M_TE_USER_NOT_FOUND
    module: M_TE
    category: TASK_GROUP
    detail: USER_NOT_FOUND
    http_status: 422
    severity: error
    message_zh: '任务组成员不存在或未启用'
    message_en: 'Task group member was not found or active'
    related_fields: []
    related_stories: [US-TE-002]
    introduced_in: v3.6

  - code: M_TE_WORKER_NOT_QUALIFIED
    module: M_TE
    category: ASSIGNMENT
    detail: WORKER_NOT_QUALIFIED
    http_status: 422
    severity: error
    message_zh: '人员不具备该任务组资格'
    message_en: 'Worker is not qualified for the task group'
    related_fields: []
    related_stories: [US-TE-002, US-TE-005]
    introduced_in: v3.6

  - code: M_TE_CERT_EXPIRED
    module: M_TE
    category: ASSIGNMENT
    detail: CERT_EXPIRED
    http_status: 422
    severity: error
    message_zh: '人员任务资格已过期'
    message_en: 'Worker task qualification has expired'
    related_fields: []
    related_stories: [US-TE-002, US-TE-005]
    introduced_in: v3.6

  - code: M_TE_WORKER_AT_CAPACITY
    module: M_TE
    category: ASSIGNMENT
    detail: WORKER_AT_CAPACITY
    http_status: 409
    severity: error
    message_zh: '人员同时在手任务已达上限'
    message_en: 'Worker has reached the active task capacity limit'
    related_fields: []
    related_stories: [US-TE-005, US-TE-006]
    introduced_in: v3.6

  - code: M_TE_NO_AVAILABLE_WORKER
    module: M_TE
    category: ASSIGNMENT
    detail: NO_AVAILABLE_WORKER
    http_status: 422
    severity: error
    message_zh: '任务组内没有可用人员'
    message_en: 'No available worker exists in the task group'
    related_fields: []
    related_stories: [US-TE-005]
    introduced_in: v3.6

  - code: M_TE_TASK_NOT_FOUND
    module: M_TE
    category: TASK
    detail: NOT_FOUND
    http_status: 404
    severity: error
    message_zh: '任务不存在'
    message_en: 'Warehouse task was not found'
    related_fields: []
    related_stories: [US-TE-005, US-TE-008]
    introduced_in: v3.6

  - code: M_TE_NOT_ASSIGNEE
    module: M_TE
    category: EXECUTION
    detail: NOT_ASSIGNEE
    http_status: 403
    severity: error
    message_zh: '仅任务当前执行人可执行此操作'
    message_en: 'Only the current task assignee may execute this operation'
    related_fields: []
    related_stories: [US-TE-008]
    introduced_in: v3.6

  - code: M_TE_INVALID_TRANSITION
    module: M_TE
    category: EXECUTION
    detail: INVALID_TRANSITION
    http_status: 409
    severity: error
    message_zh: '当前任务状态不允许此操作'
    message_en: 'Current task status does not allow this transition'
    related_fields: []
    related_stories: [US-TE-005, US-TE-008]
    introduced_in: v3.6

  - code: M_TE_QUANTITY_DIFFERENCE_REQUIRES_EXCEPTION
    module: M_TE
    category: EXECUTION
    detail: QUANTITY_DIFFERENCE_REQUIRES_EXCEPTION
    http_status: 422
    severity: error
    message_zh: '实际数量与计划数量不一致时必须上报异常'
    message_en: 'A quantity difference must be reported as an exception'
    related_fields: []
    related_stories: [US-TE-008]
    introduced_in: v3.6

  - code: M_TE_SOURCE_TASK_CONFLICT
    module: M_TE
    category: TASK
    detail: SOURCE_TASK_CONFLICT
    http_status: 409
    severity: error
    message_zh: '同一业务触发源已存在参数不同的任务'
    message_en: 'The business source already has a task with different parameters'
    related_fields: []
    related_stories: [US-TE-003]
    introduced_in: v3.6

  - code: M_TE_IDEMPOTENCY_CONFLICT
    module: M_TE
    category: TASK
    detail: IDEMPOTENCY_CONFLICT
    http_status: 409
    severity: error
    message_zh: '幂等键已被不同请求使用'
    message_en: 'Idempotency-Key was used by a different request'
    related_fields: []
    related_stories: [US-TE-002, US-TE-003, US-TE-005, US-TE-008]
    introduced_in: v3.6

  - code: M_TE_EXECUTION_INTERNAL
    module: M_TE
    category: EXECUTION
    detail: INTERNAL
    http_status: 500
    severity: error
    message_zh: '任务引擎处理失败'
    message_en: 'Task engine processing failed'
    related_fields: []
    related_stories: [US-TE-002, US-TE-003, US-TE-005, US-TE-008]
    introduced_in: v3.6

  # ========== H8 ERP 防腐层 ==========
  - code: H8_PROBE_CREDENTIAL_NOT_CONFIGURED
    module: H8
    category: PROBE
    detail: CREDENTIAL_NOT_CONFIGURED
    http_status: 409
    severity: error
    message_zh: '接口表探查凭据未配置'
    message_en: 'Interface-table probe credentials are not configured'
    related_fields: []
    related_stories: [US-H8-004]
    introduced_in: v3.10
```

---

## 7. 维护规则

1. **新增**：必须先改本字典 §6 再写代码（治理脚本会卡）
2. **废弃**：不删除，标 `deprecated_at: <date>` + 在 message_zh 加"（已废弃）"前缀；保留 ≥ 1 年
3. **修改 message**：可改，但不破坏 code（即不改 code 本身）
4. **改 severity**：必须经 ADR（涉及合规）

---

## 8. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：50 个错误码（H1×6 + H2×3 + H4×2 + H5×2 + H_DOCK×3 + H_AL×2 + M1×5 + M2×6 + M3×8 + M4×7 + M_VR×2 + M_QL×1 + M_TC×2 + M_PM×1）|
| 2026-06-07 | v2 | W6.C 配置中心 Feature Flag smoke 新增 M1_CONFIG_FLAG_* 3 项；脚本统计当前合计 59 项 |
| 2026-07-10 | v3 | 补齐当前 H4 handler 的 12 个业务错误码，并登记 H6 状态机查询与不存在错误码；脚本统计当前合计 75 项 |
| 2026-07-13 | v3.2 | 新增 M2_VERIFIER_UNAUTHORIZED，约束双人验收签字人必须是同货主有效验收岗；脚本统计当前合计 92 项 |
| 2026-07-13 | v3.3 | 登记 H-DOCK 实到对账的预约不存在、到达核对失败和不可到达状态错误码；脚本统计当前合计 100 项 |
| 2026-07-14 | v3.4 | 登记 M-DI 药检平台配置与 M-TE 任务类型配置 API 错误码；脚本统计当前合计 117 项 |
| 2026-07-15 | v3.6 | 登记 M-TE 任务组、创建、分派与执行主链 15 个错误码；脚本统计当前合计 137 项 |
| 2026-07-15 | v3.7 | 补齐 M-VR 双人策略实际返回的 7 个错误码，并统一 M-TE 内部错误命名；脚本统计当前合计 144 项 |
| 2026-07-15 | v3.8 | 登记 M-TE 任务释放条件未满足错误码；脚本统计当前合计 148 项 |
| 2026-07-21 | v3.10 | 登记 US-H8-004 接口表探查凭据未配置错误码；脚本统计当前合计 175 项 |

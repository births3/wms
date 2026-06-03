# 错误码字典（Error Codes Dictionary）

> 时间：2026-05-18
> 版本：v3.1（初版 ~50 项）
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

## 3. 严重度分布概览（v3.1 初版）

| 级别 | 数量 | 主要场景 |
|---|---|---|
| info | 5 | 状态变更通知 / 数据已存在等正常路径 |
| warning | 25 | 业务规则拦截（库存不足 / 资质过期等）|
| error | 15 | 业务异常（数据冲突 / 校验失败）|
| critical | 5 | 合规/安全异常（跨货主访问 / 篡改尝试）|
| **合计** | **50** | — |

---

## 4. 模块前缀分布

| 前缀 | 错误码数 | 主要场景 |
|---|---|---|
| H1 | 6 | 鉴权 / 多租户隔离 |
| H2 | 3 | 审计 / 事件总线 |
| H4 | 2 | 通知发送 |
| H5 | 2 | 快递面单 |
| H_DOCK | 3 | 月台预约 |
| H_AL | 2 | 告警引擎 |
| M1 | 5 | 主数据校验 |
| M2 | 6 | 入库流程 |
| M3 | 8 | 库存与状态 |
| M4 | 7 | 出库与拣选 |
| M_VR | 2 | 规则引擎 |
| M_QL | 1 | 质量联系单 |
| M_TC | 2 | 追溯码 |
| M_PM | 1 | 参数对照 |

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

  # ========== H-AL 告警引擎 ==========
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
    message_zh: '统一社会信用代码格式错误（应为 18 位）'
    message_en: 'USCC format invalid (must be 18 chars)'
    related_fields: [delivery_org_uscc, shipper_org_uscc, receiver_org_uscc]
    related_stories: [US-M1-005]
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

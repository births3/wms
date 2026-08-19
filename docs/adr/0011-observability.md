# ADR-0011：可观测性方案（OpenTelemetry + Prometheus + Loki + Grafana）

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0001 / ADR-0006 / ADR-0010 / docs/coding-standards.md §1.6

---

## 背景

软件设计审计 §4 维度 6 识别可观测性缺口：

- L10 测试维度（11 层之一）有了，但**没有具体方案**
- coding-standards §1.6 提到 tracing 但没标准化
- H1 / H2 / H4 等基础能力实施时各做各的，事后整合困难
- GSP 监管审计要求快速定位问题（合规审计员问"昨天 14:00 那笔出库的具体路径"）

不解决会导致：
- 各 Wave 实施时技术栈分裂
- 跨端跨服务请求无法追踪（trace_id 不传）
- 业务 KPI 散落在各业务模块日志中无法聚合
- 关键告警（H-AL）缺数据源

---

## 候选方案

### 方案 A（推荐）：OpenTelemetry + Prometheus + Loki + Grafana

- **OpenTelemetry SDK**：统一 trace + metrics + log，跨语言（Rust / TS / RN）
- **Prometheus**：metrics 抓取与存储
- **Loki**：日志聚合
- **Tempo / Jaeger**：trace 后端（可选，初期不必）
- **Grafana**：统一仪表板

### 方案 B：自研日志 + 单独的 metrics

- 不引入 OTel
- 用 tracing crate（Rust）+ pino（前端）+ 自研 metrics

**否决理由**：
- 跨端 trace_id 传递无标准
- 后期对接监管审计（如药监 EDI 出问题）无统一查询入口
- 长期维护成本高

### 方案 C：商业 APM（如 Datadog / New Relic）

**否决理由**：
- 数据出境涉及 GSP 合规（医药数据不能出境）
- 成本高（按事件数计费）
- 商业绑定

---

## 决策

**采用方案 A：OpenTelemetry + Prometheus + Loki + Grafana**。

### 技术栈定型

| 层 | 技术 | 版本约束 |
|---|---|---|
| **trace** | OpenTelemetry SDK | OTel 1.x（稳定）|
| **metrics 抓取** | Prometheus | 2.40+ |
| **logs 聚合** | Loki | 2.9+ |
| **trace 后端** | Tempo（推荐）/ Jaeger | 可延后 |
| **仪表板** | Grafana | 10.x+ |
| **Rust 集成** | `tracing` + `tracing-opentelemetry` + `opentelemetry-otlp` | — |
| **TS 集成** | `@opentelemetry/api` + `@opentelemetry/sdk-node` | — |
| **RN 集成** | `@opentelemetry/sdk-trace-web` | — |

### 日志格式（强制 JSON 结构化）

```json
{
  "ts": "2026-05-18T10:30:45.123Z",
  "level": "info",
  "trace_id": "01H7K8...",
  "span_id": "8f9a...",
  "service": "wms-api",
  "module": "M3",
  "operation": "inventory.update_status",
  "tenant_id": 1,
  "user_id": 10001,
  "event": "status_changed",
  "old_value": "qualified",
  "new_value": "isolated",
  "approval_source": "M-RC-2026051800123",
  "duration_ms": 45,
  "result": "success"
}
```

**必填字段**：`ts / level / trace_id / span_id / service / module / operation / tenant_id`

**条件必填**：`user_id`（认证后），`event`（业务事件），`approval_source`（写操作）

### Trace 上下文传递

- HTTP：W3C Trace Context（`traceparent` / `tracestate` header）
- 跨端：PDA 扫码 → 后端 → DB 全链路 trace_id 一致
- 异步：H2-005 事件总线携带 trace_id 到订阅者

### Metrics 命名约定

```
wms_<module>_<entity>_<action>_<metric_type>
```

| 类型 | 后缀 | 示例 |
|---|---|---|
| Counter | `_total` | `wms_m3_inventory_update_total` |
| Gauge | （无） | `wms_m3_inventory_near_expiry_count` |
| Histogram | `_seconds` / `_bytes` | `wms_h3_api_request_duration_seconds` |

**Labels**（不允许高基数）：
- 允许：`module`, `operation`, `result`, `severity`, `tenant_id`（≤ 100）
- 禁止：`user_id`（高基数），`order_id`（无限基数）

### 关键 KPI 清单（每个核心模块至少 5 个）

每个核心模块的故事文件**必须声明本模块需暴露的 metric**（在跨故事约束加 §10 KPI 清单）。治理脚本 `check_observability.py` 校验。

**示例（M3 库存）**：

| KPI | 类型 | 用途 |
|---|---|---|
| `wms_m3_inventory_total` | Gauge | 实时库存量（按货主+商品+状态聚合）|
| `wms_m3_inventory_status_change_total` | Counter | 状态变更次数 |
| `wms_m3_inventory_near_expiry_count` | Gauge | 近效期商品数 |
| `wms_m3_inventory_recalled_count` | Gauge | 召回标记数 |
| `wms_m3_inventory_query_duration_seconds` | Histogram | 库存查询性能 |

**示例（M2 入库）**：

| KPI | 类型 | 用途 |
|---|---|---|
| `wms_m2_asn_created_total` | Counter | ASN 创建数 |
| `wms_m2_receipt_duration_seconds` | Histogram | 收货全流程耗时 |
| `wms_m2_temperature_exceed_total` | Counter | 温度超标拒收数 |
| `wms_m2_dual_person_required_total` | Counter | 双人验收触发数 |
| `wms_m2_quantity_mismatch_total` | Counter | 数量异常率 |

### SLO 与告警

每个 SLO 对应一条告警规则（Prometheus alert rule）：

| SLO | 告警阈值 | 接收人 |
|---|---|---|
| API p99 < 500ms | p99 > 1s 持续 5min | 系统管理员 |
| 错误率 < 0.1% | 5min 错误率 > 1% | 仓库主管 + 系统管理员 |
| 库存查询 p95 < 300ms | p95 > 500ms 持续 5min | 系统管理员 |
| 监管上报成功率 > 99% | 5min 失败率 > 5% | 系统管理员 + 货主 |
| critical 错误码 5min 内出现 | 直接告警 | H-AL 集成 |

### 成本预算（初期，单仓 < 10 货主）

- Prometheus：单机即可，预算 50GB/30 天
- Loki：单机，预算 200GB/30 天（按需归档到对象存储）
- Tempo：单机，预算 100GB/30 天
- Grafana：单机
- 总计：约 1 台中等规格机器（4C/16G/500GB SSD）

---

## 后果

### 正面

- **统一可观测性**：trace + metrics + log 同源，故障定位 < 5 分钟
- **跨端追踪**：PDA 扫码到 DB 写入全链路 trace_id 一致，业务问题端到端复现
- **GSP 审计支持**：每条 API 请求都有 trace_id，监管审计可即查
- **业务 KPI 暴露**：业务模块强制声明 KPI，避免实施后无指标
- **告警自动化**：H-AL 告警引擎数据源直接来自 Prometheus

### 负面

- **学习成本**：OpenTelemetry SDK 各语言用法略不同，初期 2 周适应
- **运维成本**：增加 Grafana / Prometheus / Loki 三个组件维护
- **存储成本**：日志和 metrics 占磁盘，需配置归档策略

### 风险

- **PII 泄露**：日志中可能误带敏感字段（如手机号）
- **应对**：定义"日志脱敏字段清单"+ Loki 查询权限分级 + critical 字段加密
- **trace 性能开销**：高频接口加 trace 可能 5-10% 延迟
- **应对**：sampling rate 默认 10%（critical 路径 100%）

---

## 实施约束

1. 所有写操作的 handler 必须有 `tracing::instrument` 注解
2. 业务 metric 在每个模块的故事文件 §跨故事约束 §10 声明
3. critical 错误码（ADR-0010）必须自动触发 H-AL 告警
4. PII 日志字段（手机号 / 身份证 / 银行账号）必须脱敏，参照 [../compliance/gsp-field-coding-standards.md](../compliance/gsp-field-coding-standards.md) §5 加密分级
5. trace_id 跨端传递必须用 W3C 标准（不自创）
6. 单机部署预算先满足，多副本/异地容灾在 Wave 5 启动时再 ADR

---

## 治理脚本

`scripts/governance/check_observability.py`（T1 级，与 ADR 同步落地）：
- 扫描 `docs/domain/user-stories-*.md` 中的 §10 KPI 声明
- 校验 KPI 命名约定（`wms_<module>_<entity>_<action>_<metric_type>`）
- 校验每个核心写操作模块至少声明 5 个 KPI

---

## 参考

- OpenTelemetry: https://opentelemetry.io/
- W3C Trace Context: https://www.w3.org/TR/trace-context/
- Prometheus naming: https://prometheus.io/docs/practices/naming/
- Grafana Loki: https://grafana.com/oss/loki/

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：技术栈定型 + 日志格式 + Metrics 命名 + KPI 清单 + SLO 告警 |

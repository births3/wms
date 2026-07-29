# Runbook：H8 ERP 防腐层双通道（本地）

> 对应：`docs/infra/technical-specs.md` H8
> **通道 B**：接口表 + Worker
> **通道 A**：WMS OpenAPI 入站 + ERP HTTP 回调出站
>
> “双通道”表示本地可分别验证两种通道，不表示生产双写；生产按 US-H8-001 选择唯一
> 路由，主备切换沿用同一 Idempotency-Key。

## 1. 启动接口库（通道 B）与容器 ERP 厂商（通道 A）

```bash
cd deploy
# 通道 B：MSSQL 接口表
docker compose -f docker-compose.h8-erp-if.yml up -d
./h8-erp-if/wait-and-init.sh

# 通道 A：容器化外部 ERP 厂商（独立进程/端口/回执存储）
export H8_ERP_VENDOR_BEARER_TOKEN='replace-with-local-test-token'
docker compose -f docker-compose.h8-erp-vendor.yml up -d --build
export ERP_CALLBACK_BASE=http://127.0.0.1:18092
curl -fsS \
  -H "Authorization: Bearer $H8_ERP_VENDOR_BEARER_TOKEN" \
  "$ERP_CALLBACK_BASE/healthz"
```

首个正式版本前只维护当前 MSSQL schema 基线。若本地持久卷来自旧 migration，
必须按 ADR-0038 删除并重建该开发卷后再初始化，不能把初始化脚本当作旧结构升级器。

应用：`01` 入站三表、`03` 出站+销退、`04` 商品变更回写。
端口 **14333**，SA 默认 `Wms_Erp_If_Dev_2026!`（仅本地）。
ERP 厂商容器端口 **18092**，厂商标识 `container-erp-vendor-a`；回执 `GET /receipts/{source_outbox_id}`。

## 2. WMS 与环境变量

```bash
export WMS_API_BASE=http://127.0.0.1:18090
export WMS_API_TOKEN='...'   # login 需 owner_code
export WMS_DB_URL='postgres://...'   # 出站读 outbox
export H8_CONNECTOR_ID='...' # 当前接口库对应的 H8 连接 UUID；一个 Worker 只绑定一个连接
export WMS_H8_SECRET_ALIASES='{"vault://h8/worker-db":"本机注入的接口库密码"}'
# 可选：H8_WORKER_ID（默认主机名+PID）/ H8_WORKER_VERSION / H8_HEARTBEAT_TTL_SEC

# API 对 REST 连接测试采用精确 endpoint 白名单；必须写主机/IP + 端口
export WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS='erp.example.internal:443,10.20.30.40:443,[fd00::10]:443'
# 仅本机开发需要；默认/生产不得设置
export WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP='true'

# API 侧完整报文短期保留；生产必须由 Secret 管理器注入，不写入仓库或日志
export WMS_ENCRYPTION_MASTER_KEY='至少 32 字节的主密钥'
export WMS_ENCRYPTION_KEY_VERSION='v2' # 可选，默认 v1
# 轮换期按版本提供尚未到期的旧密钥；值由 Secret 管理器注入，禁止写入文件或日志
export WMS_ENCRYPTION_PREVIOUS_MASTER_KEYS='{"v1":"至少 32 字节的旧主密钥"}'

# 出站生产路由来自 H8 ERP 连接配置：channel_mode + api_base_url。
# Worker 不接受 --transport，也不读取全局首条 active 连接；每条 outbox 按
# 货主、可用 warehouse_id、outbound 和 message_type 调 route-resolve。
# rest_primary_table_fallback：REST 连续失败（H8_HTTP_MAX_ATTEMPTS，默认 2）后
# 以同一 Idempotency-Key 写入 if_out_message，不双写。
```

非本机 REST 连接测试只允许 HTTPS，目标 `host:port` 必须精确出现在
`WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS`；HTTPS URL 未写端口时按 `443` 匹配。只写主机、
通配符、同主机不同端口均拒绝；IPv6 必须写成 `[address]:port`。明确列入的企业内网 IP
可以使用；未指定、回环、链路本地、组播和非规范 IP 表示一律拒绝。本机开发仅例外允许
`http://localhost` / `http://127.0.0.1`，仍须把实际端口（HTTP URL 未写时为 `80`）
加入 endpoint 白名单，并显式设置 `WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP=true`。该开关
默认关闭，生产不得启用。白名单中的 DNS 名由部署运维负责保证解析目标可信；连接探测
不接受 URL 用户信息、查询串、重定向，也不读取本机 `.curlrc`。

权限：`mpm.execute`、`m2.write`、`m4.write`、商品写和 `h8.erp_connector.write`；迁移 `202607230002_mpm_persistent_mapping.sql` 会登记 `mpm.execute`，缺权限时重登刷新 JWT。
扩展 outbox：`202607180005_h8_erp_outbox_extensions.sql`。

## 3. 入站类型（通道 B 接口表 → WMS API）

| type | 表 | API |
|------|-----|-----|
| asn | if_in_asn | POST receiving-orders |
| outbound_order | if_in_outbound_order | POST outbound/orders |
| product_master | if_in_product_master | POST products |
| return_order | if_in_return_order | POST receiving-orders sales_return（必填 batch_no、supplier_id） |
| product_change | if_in_product_change | POST `/api/v1/integration/erp-messages/inbound/product_change`；携带 `liaison_id + asn_id` 时再 POST 档案同步回执 |

`product_change.field_name=physical_dimensions` 时，`new_value` 必须是包含
`length_mm`、`width_mm`、`height_mm` 三个正数的 JSON 对象；三个尺寸作为一个原子变更，
不接受单字段更新。其他受支持字段继续使用标量 `new_value`。接口表 Worker 与 REST 入站
都进入同一 H8 防腐层，不再直接调用 M1 商品 PATCH。

档案补录回执为 `POST /api/v1/quality-liaisons/{id}/archive-sync-callback`。Worker 只在
M1 商品幂等更新成功后调用；服务端校验档案出站已成功、审批载荷、商品实际值、
货主/仓库和 ASN 状态，同一事务将 M-QL 改为 `landed`、ASN 恢复 `inspecting`。
重试复用 `{product_change_idempotency_key}:archive-closeout`，不重复转移或写审计。

## 4. 出站类型（WMS outbox → B 或 A）

| outbox 表 | event_type | 回调 path（通道 A） |
|-----------|------------|---------------------|
| receiving_putaway_erp_feedback_outbox | inbound_putaway_completed | /inbound-complete |
| inventory_status_erp_feedback_outbox | inventory_status_changed | /inventory-status |
| stock_adjustment_erp_feedback_outbox | stock_loss/surplus_completed | /stock-adjustment |
| archive_revision_erp_feedback_outbox | archive_revision | /archive-revision |
| reconciliation_erp_feedback_outbox | reconciliation_diff | /reconciliation-diff |
| shipment_confirm_erp_feedback_outbox | shipment_confirm | /shipment-confirm |
| inventory_snapshot_erp_feedback_outbox | inventory_snapshot | /inventory-snapshot |

档案补录 outbox：`max_attempts=5`、失败退避 5 分钟、`deadline_at` 默认 24h，超时/超次 → `dead`。

通道 B 写入 `if_out_message` 后，ERP 可确认：

```bash
python3 scripts/h8_erp_interface_sync/ack_if_out.py --all
```

通道 A 的 ERP 业务回执调用
`POST /api/v1/integration/erp-messages/{message_id}/receipt`，使用具备
`inbound:push` scope 的 `X-WMS-API-Key`，并携带与原出站消息一致的
`Idempotency-Key`、`schema_version` 和 `correlation_id`。`result=ok` 进入
`acked` 且不得携带 `error_summary`；`result=rejected` 必须带 `error_summary`
并进入 `dead`。生成的 curl 示例必须保留 API Key 与幂等头，禁止误写为 JWT Bearer。

技术送达后 H8 按 ADR-0018 L2 设置 `next_retry_at`。Worker 每轮先处理已到期回执：
服务端把同一消息推进为重试或第 5 次超时 `dead`；可重试时只把原幂等键指向的
原 outbox 从 `succeeded` 恢复为 `failed`，随后由既有出站管线重新发送，禁止复制消息。

## 5. Worker

Worker 启动后向「H8 ERP 消息 / Worker 状态」上报实例、版本、方向、当前认领数和心跳。
启动时只从环境读取 WMS 控制面地址、令牌和 `H8_CONNECTOR_ID`，随后先读取当前连接，
再读取 `/api/v1/config/erp-connectors/{id}/versions/{config_version}` 不可变快照。
接口库主机、端口、库名、用户名和密码 alias 全部来自该快照；密码仅通过
`WMS_H8_SECRET_ALIASES` / `WMS_SECRETS_MAP` 解析。生产 Worker 禁止使用
`H8_MSSQL_*`、Docker 容器名或全局 MSSQL 默认值覆盖连接配置；历史版本的在途消息按其
既有 `connector_config_version` 排空，新版本由新 Worker 实例承接。

Worker 主机必须安装 `sqlcmd`，并使用快照中的 `tcp:host,port` 直连 MSSQL；Worker
不执行 `docker exec`。本运行手册中的 Docker、SA 密码和初始化变量仅用于开发接口库
建库/证据准备，不是生产 Worker 的传输配置。

每批认领前读取当前连接 + 方向的暂停控制；暂停时不触碰 MSSQL 待处理行，在途批次继续完成。
恢复或暂停到期后继续认领。心跳失败只告警，不中断已经在途的业务处理。

完整报文默认只计算摘要。管理员在 Worker 状态页按连接启用后，API 使用 PostgreSQL
`pgcrypto` 加密保存 1–30 天（默认 7 天）；查看明文需要写权限并产生 H2 审计。API 每小时
清除到期密文，保留消息、尝试、摘要和审计。部署迁移账号必须有创建 `pgcrypto` 扩展的权限；
密钥缺失或错误时禁止启用/解密，不得把数据库加密错误返回前端。

```bash
# 双向 + 接口表出站
python3 scripts/h8_erp_interface_sync/sync_worker.py --once

# 连接配置为 REST 后仅跑出站
python3 scripts/h8_erp_interface_sync/channel_a_callback_mock.py --port 18091 &
# 在 H8 ERP 连接中配置 api_base_url=http://127.0.0.1:18091
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction out

# 入站指定类型
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction in --types product_change

# H-SCH 每 5 分钟按“一个 Worker/连接/货主令牌”触发一次；规则决定是否到期
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --reconcile-due
```

`--reconcile-due` 先以本轮唯一 Idempotency-Key 调用 `POST /api/v1/reconciliation/claims`。
`WMS_API_TOKEN` 必须来自持有 service-only `rc.reconciliation.ingest` 的货主级服务账号；
仓库主管的 `rc.reconciliation.execute` 不能认领窗口、续租、上报 Worker 失败或提交快照。
服务端只认领当前 JWT 货主，并要求存在 active、outbound、owner-wide
（`warehouse_ids=[]`）的 `inventory_snapshot` REST 路由；仓级路由不得与货主全量库存混算，
未到配置周期或已有 active claim 时返回 `claim=null`。

claim 返回 `id`、`claim_token`、deterministic `window_key`、`attempt_no` 和有限
`lease_expires_at`。Worker 每个 ERP 分页前及最终提交前调用
`POST /api/v1/reconciliation/claims/{id}/renew`，租约只允许 30–900 秒；本实现使用 120 秒，
等于单次 ERP 请求超时边界的两倍。租约过期后其他 Worker 可认领相同窗口的新 attempt，旧
token 不再允许续租或提交，从而避免多 Worker 同时拉取同一窗口。

持有租约后 Worker 调用自研 ERP
`GET {api_base_url}/inventory-snapshots?owner_id=...&cursor=...`，所有分页必须保持同一
`snapshot_at`，再携带 claim id/token 一次提交 `POST /api/v1/reconciliation/runs`。
对账批次成功与 claim completed 在同一事务完成；同窗口同请求重放返回原结果，不同请求返回
幂等冲突。ERP 拉取或 WMS 提交失败时，Worker 调用
`POST /api/v1/reconciliation/claims/{id}/failed`，仅上报受控的 `pull|submit` 阶段和稳定
错误码，不上传外部响应正文：`pull` 只能配 `erp_pull_failed`，`submit` 只能配
`snapshot_submit_failed`；`lease + lease_expired` 仅由服务端租约接管流程写入。
数据库同时约束阶段/错误码配对及 active/completed/failed/expired 状态形态，绕过 API 的非法
组合也会失败。服务端追加 H2 审计并创建 H4 运维通知。失败上报自身不可用时，
Worker 保留并抛出原始异常，租约到期后由下一 Worker 接管。
连接配置必须存在 `bearer_secret_alias`，Worker 从 `WMS_H8_SECRET_ALIASES` /
`WMS_SECRETS_MAP` 解析该货主令牌；所有环境都禁止跨货主复用 `ERP_API_TOKEN`
等全局令牌。开发与测试环境也必须为连接填写显式 alias，并在本地 secret map
中提供对应值；alias 缺失或无法解析时按受控 503 保留消息等待重试。

H-SCH 不持有跨货主超级令牌，也不自行计算周期；每个货主连接实例使用自己的
`WMS_API_TOKEN`。建议调度器每 5 分钟调用一次，实际执行频率由
`reconciliation_rules.interval_hours` 控制。

以 WMS 为准生成的 `reconciliation_erp_feedback_outbox` 最多自动尝试 5 次；耗尽后标记
dead 并创建 H4 运维通知，不再无限重试。

单测：`cd scripts/h8_erp_interface_sync && python3 -m unittest test_h8_sync_worker test_exchange_lifecycle test_outbound_routing test_outbound_receipts test_reconciliation_pull -v`

### ASN 入站闭环样本

初始化脚本提供独立的 `DEMO-ASN-FLOW-001`，引用 E2E 库中真实存在的货主、仓库、
供应商和商品，不占用 US-H8-004 只读探查固定样本。启动 WMS API 并取得令牌后执行：

```bash
export H8_BATCH_SIZE=1
python3 scripts/h8_erp_interface_sync/sync_worker.py --once --direction in --types asn
```

验收必须同时确认：接口行由 `pending` 经 Worker 认领后变为 `success`；
`wms_resource_id` 指向唯一的 M2 `receiving_orders`；H2 存在 receive、convert、
business_api、receipt 审计；通过消息重放 API 对同一消息填写原因并确认后，Worker
应自动以原 Idempotency-Key 将既有终态接口行恢复为 `pending`、接管消息并重放，
不得人工修改接口表；M2 单据数量仍为 1 且资源 ID 不变。证据见
`docs/retros/h8-asn-inbound-flow-evidence.json`。

### ASN 死信与人工重放切片

用不存在的供应商引用准备独立 ASN，并设置 `H8_MAX_RETRY=1` 后运行同一 Worker：接口表与
H8 消息必须同时进入 `dead`，H8 留下 `h8_exchange_final_failure`、`h8_message_dead`
和 dead 尝试记录。随后由外部 ERP 更正供应商引用但不修改 `sync_status`，管理员调用既有
重放 API（原因 + 二次确认）；Worker 必须以原 Idempotency-Key 自动把原接口行恢复为
`pending`、认领并完成业务调用。最终必须同时断言：

- H8 使用同一消息 ID、连接 ID、连接编码、配置版本和通道，状态为 `succeeded`；
- MSSQL 原行状态为 `success`，`retry_count` 保留且资源 ID 指向 M2 单据；
- M2 同一 `external_ref` 只有一张收货单；
- H2 具备失败、死信、重放、认领和成功回执的完整关联动作。

定向自动化命令：

```bash
cd scripts/h8_erp_interface_sync
python3 -m unittest test_inbound_terminal_state test_h8_sync_worker test_exchange_lifecycle -v
cargo test --manifest-path ../../backend/Cargo.toml -p wms-api \
  inbound_lifecycle_persists_failure_retry_and_success_status --lib -- --nocapture
```

本地真实运行记录见 `docs/retros/h8-asn-manual-replay-evidence.json`；它只证明 V2 软件切片，
不替代客户 ERP dev/staging 的 V4 故障恢复证据。

## 6. 验收

| ID | 检查 |
|----|------|
| B 入站 | asn/outbound/product/return/product_change → success |
| B 出站 | outbox → if_out_message pending → ack → acked |
| A 出站 | mock `/_dump` 收到 archive/recon/shipment 等 path |
| 档案重试 | 超 max/deadline 变 dead |

主备降级 L11 使用真实 Docker MSSQL 运行，并显式开启证据写入：

```bash
sudo -n bash deploy/h8-erp-if/wait-and-init.sh
sudo -n env PYTHONPATH=scripts/h8_erp_interface_sync \
  python3 scripts/h8_erp_interface_sync/run_failover_l11_evidence.py --record
```

通过条件是同一业务键连续两次从 REST 降级到接口表后，
`interface_row_count` 仍为 `1`，且证据中的 `cleanup` 为
`deleted acceptance row`。证据写入
`docs/retros/h8-failover-l11-evidence.json`；该 V2 证据不替代客户正式 ERP S4。

### 本机记录（2026-07-18 补全）

- 通道 A：archive_revision / reconciliation_diff / shipment_confirm → HTTP mock succeeded
- 通道 B：inventory_snapshot → if_out；ack_if_out → acked
- 入站 product_change：manufacturer 回写 `H8药厂-已补录`
- M-RC：自研 ERP 库存 HTTP 契约、到期窗口、真实 PostgreSQL 对账和浏览器证据已通过

## 7. 仍非本切片

- 真实 ERP 与质量矩阵 S4 关闭
- 内置 `process_*_outbox` 本地直接 succeeded 与 H8 worker **并存时的运维约定**：启用 H8 出站时勿并行跑会抢 outbox 的本地闭环 job

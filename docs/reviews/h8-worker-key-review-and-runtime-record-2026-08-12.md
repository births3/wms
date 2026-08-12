# H8 Worker Key 审查与运行记录

日期：2026-08-12（Asia/Shanghai）
范围：`zbpf7_test` SQL Server 接口表同步、H8 Rust Worker 控制面认证、API Key 创建、运行恢复和本轮 review-fix-review。
相关提交：`8749537f`、`416b1ccc`；本记录对应审查基线为 `a0ed2b91`。

> 本记录不保存登录密码、JWT、API Key 明文、Redis 密码、SQL Server 密码或 secret alias
> 的真实值。API Key 明文只在创建响应中出现一次，本次仅注入运行时环境，没有写入仓库、命令输出或日志。

## 1. 结论

本次“重启同步”已经恢复并持续运行，但 23,012 行历史权限失败数据仍在分批处理中，当前不能
把“Worker 已启动”表述为“全部同步完成”：

- WMS API 健康检查返回 HTTP 200。
- 新建的 H8 Worker 专用 H1 API Key 能访问当前连接控制面，探测返回 HTTP 200，且
  `last_used_at` 已更新。
- `h8-rust-worker-test` 已重启到新进程，当前观察到连续 `inbound=100 outbound=0`；重启后接口
  审计持续出现入站请求，成功请求为 HTTP 200。
- 已从 SQL Server 精确重置 23,012 行旧权限失败数据：商品 1 行、客户 23,010 行、供应商
  1 行。商品 422 映射缺失、客户其他非权限失败和 `INVALID_DATA` 没有被重置。
- 重启后抽样窗口已确认客户数据从待处理状态向成功状态推进；完整数量需要等 Worker 处理完
  余量后再以 SQL Server 状态和 WMS 落库数量复核。
- 当前验证对象为货主 `PY_OWNER`、连接 `H8-IF-E2E`、接口库 `zbpf7_test`；Worker 使用
  `H8_CONNECTOR_ID` 固定绑定该连接。
- 未启动历史 Python Worker，也未让 Python Worker 与 Rust Worker 并行竞争接口表。

这次故障包含三层原因：旧 Worker 的 JWT 已过期且运行进程没有随代码修改重启；新 Worker Key
进入业务入站路由时，鉴权中间件此前丢弃了 `h8:worker` 权限，入站生命周期又只接受连接上
原 ERP Key，导致历史数据被记录为 403；此外，API 的 Redis 鉴权配置不一致曾使 Key 创建
按 fail-closed 返回 `AUTH-010`。SQL Server 本身仍可访问，商品的 422 记录则是独立的参数映射
缺失问题。

## 2. 发生了什么

### 2.1 原始症状

旧 Worker 使用会过期的 JWT/`WMS_API_TOKEN` 访问控制面，返回 `401 AUTH-003`。进程虽然
还在运行，但它加载的是旧二进制和旧环境变量；仅重新生成代码或修改仓库文件不会改变已经
运行的进程。

### 2.2 认证设计调整

H8 Worker 已改为长期 API Key 方案：

| 项目 | 当前约定 |
|---|---|
| Worker 控制面认证 | `WMS_H8_WORKER_API_KEY` + `X-WMS-API-Key` |
| 禁止继续使用 | `WMS_API_TOKEN` / Worker Bearer JWT |
| 控制面 scope | `h8:worker` |
| 控制面权限映射 | `h8.erp_connector.read` + `h8.erp_worker.write` |
| 业务入站 scope | 按当前连接消息类型追加最小 scope |
| Key 仓库范围 | `warehouse_ids=[]`，由货主和 `H8_CONNECTOR_ID` 限制 |
| 明文保存 | 仅创建/轮换响应一次；不进入数据库明文、代码、文档和日志 |

`h8:worker` 不包含 `h8.erp_connector.write`，因此 Worker 不能创建、修改、测试、启停
或删除 ERP 连接。通道 A 的 REST 出站业务回执另使用 `outbound:receipt`，不能误用
`inbound:push`。

## 3. 时间线

| 时间 | 处置与证据 |
|---|---|
| 18:xx 前 | 复现旧 Worker 控制面 401，确认旧 JWT 已失效；停止旧 Worker，避免旧认证继续重试。 |
| 随后 | 使用新 Rust Worker 二进制启动但不提供新 Key，进程按设计快速失败并报 `H8_WORKER_CONTROL_KEY_REQUIRED`，证明缺少长期凭据不会降级为旧 JWT。 |
| 第一次创建 Key | 通过官方 H1 API Key 接口创建失败，返回 `503 AUTH-010`。API `/healthz` 仍为 200。 |
| 排查 | 确认 API 进程使用的 `WMS_REDIS_URL` 未带本地 Redis 所需认证信息；Redis 撤销存储不可用，高风险 API Key 写入按设计 fail-closed。 |
| 18:17 左右 | 仅在 API tmux 运行环境补齐已存在的 Redis 认证配置并重启 API；未修改仓库配置、未打印密码。 |
| 之后 | 通过登录态调用官方 H1 API Key 创建接口，使用 `Idempotency-Key`，未直接写数据库。 |
| 18:17:42 UTC+8 | 新 Key 到期时间为 `2027-02-08 18:17:42`；明文只在创建响应中取得一次，未写入本记录。 |
| 18:18:29 UTC+8 | 新 Key 的 `last_used_at` 更新，控制面探测 HTTP 200。 |
| 之后 | 将新 Key 仅注入 `h8-rust-worker-test` tmux 会话并重启 Worker；Worker 持续输出 `inbound=100 outbound=0`。 |
| 20:xx | 发现 Worker 控制面请求正常但接口表没有入站调用；核对 SQL Server 后确认可重试数据被标记为 `handelflag=4/5`，Worker 重启不会自动重放这些终态记录。 |
| 20:xx | 修复 Worker Key 在业务入站路径上的权限保留和连接 Key 绑定判断；API 重启后健康检查 HTTP 200，针对 3 行抽样数据验证入站 HTTP 200。 |
| 20:19 左右 | 仅重置已确认的 403/AUTH-005 数据 23,012 行，随后重启 Worker；接口审计持续出现 HTTP 200 入站请求，客户数据开始从待处理状态推进。 |

## 4. 本次生成的 Key 与连接范围

以下仅记录不可用于认证的元数据：

| 项目 | 值 |
|---|---|
| Key ID | `78043a1f-db87-44c2-9540-74fa32d4b7a8` |
| 调用方 | `h8-rust-worker` |
| 用途 | `zbpf7_test H8 接口表同步控制面` |
| 货主 | `PY_OWNER` |
| 连接 | `H8-IF-E2E` / `00000000-0000-0000-0000-000000008801` |
| 仓库范围 | 空（货主级） |
| Scope | `h8:worker`、`inbound:push`、`inventory:seed`、`master-data:write`、`order:command`、`outbound:push` |
| 负责人 | `PY_OWNER/admin` |
| 有效期 | 到期时间已记录在 H1 生命周期元数据，见上方；不记录 secret |

当前连接的既有 ERP-facing `api_key_id` 没有被本次 Worker 控制面 Key 替换。两者职责不同：
Worker Key 用于 WMS 控制面认领/推进消息，连接自身的 Key 继续用于既有外部通道绑定。后续
轮换或吊销前必须先确认实际调用方，不能按 Key 名称猜测。

## 5. Review → 修复 → Review

### 5.1 第一轮审查发现与修复

初版长期认证实现的主要风险是最小权限边界：Worker 需要运行控制面写权限，但不应获得
ERP 连接管理写权限。修复后形成以下边界：

- 新增 `h8:worker` 受控 scope，只映射 `h8.erp_connector.read` 和 `h8.erp_worker.write`。
- 心跳、认领决策、认领和生命周期推进使用 Worker 写权限。
- 连接创建/修改/测试/启停/删除、消息重放/解密/清理/归档等管理动作仍保留管理员权限。
- Worker 控制面客户端统一发送 `X-WMS-API-Key`，不再按路径回退到 Bearer JWT。

### 5.2 当前轮审查发现与修复

基于 `a0ed2b91...HEAD` 审查代码、运行手册和 H8 契约，发现一处文档错误：

- 运行手册把 REST 出站业务回执的 scope 写成 `inbound:push`。
- H8 用户故事和后端鉴权实现的正确契约是独立的 `outbound:receipt`，只映射
  `h8.erp_receipt.write`。
- 已修正 [H8 接口表同步运行手册](../runbooks/h8-erp-interface-table-sync.md)，并补充
  `AUTH-010`、Redis 配置、进程重启和验证要求。

### 5.3 重启同步审查发现与修复

本次实际重启同步又发现了一个比“Key 是否有效”更具体的业务路径缺口：

- `api_key_auth` 过去只把当前 URL 对应的业务 scope 转成权限，虽然 Key 具有 `h8:worker`，
  进入商品/客户/供应商入站 URL 后仍没有 `h8.erp_worker.write`。
- H8 入站生命周期过去只接受连接配置上的 ERP-facing `api_key_id`，独立的 Worker Key 即使
  认证成功也会被拒绝为 `AUTH-005`。
- 已在共享鉴权中保留 `h8:worker` 对应的 Worker 写权限，同时仍要求当前 URL 的业务 scope；
  已在入站生命周期允许具有 Worker 写权限的专用 Key，不替换连接原有 ERP Key。
- 追加了两个回归测试，分别覆盖 scope 合并和“专用 Worker Key 不替换连接绑定 Key”的行为。

### 5.4 第二轮审查结果

本轮修复后重新检查以下一致性：

- `api_key_auth.rs`、H8 用户故事、H1 API Key 切片、运行手册的 scope 定义一致。
- Worker 只依赖 `WMS_H8_WORKER_API_KEY`，所有控制面请求统一使用 API Key。
- Worker Key 仍必须同时具备当前入站业务 scope；`h8:worker` 只补充控制面推进权限，未扩大
  为 ERP 连接管理写权限。
- 连接原有 `api_key_id` 仍保留给既有 ERP-facing 通道，Worker Key 只作为接口表同步控制面
  调用方。
- 运行手册没有把 API `/healthz` 当作 Redis/Key 链路的充分证据。
- 文档没有包含任何真实密码、Token、API Key 明文或 secret alias 值。

最终代码和文档审查结果：无未关闭的高风险或阻断项。

## 6. 实际运行方式

本次是本机验证，不是生产发布：

1. API 使用本地 Rust debug 二进制，在 tmux 会话 `wms-api-18184-h8-test` 中运行，监听本机
   `18184`；修改环境变量后通过 tmux 重启使配置生效。
2. SQL Server 通过既有 SSH 外网映射通道转发到本机 `127.0.0.1:19631`；本记录不保存
   外网 SSH 地址、账号或密钥。
3. Worker 使用本地 Rust debug 二进制，在 `h8-rust-worker-test` tmux 会话中直接运行，
   环境变量只存在于该会话；这不是进程监督器，断会话或机器重启不会自动拉起。
4. staging/生产应使用 `docker-compose.staging.yml` 的 `h8-erp-worker-staging`，其
   `restart: unless-stopped` 和 healthcheck 才是可持续运行方式；不要把 tmux 当作生产部署。
5. Worker 只会认领 `handelflag=0`、到期可重试的 `handelflag=2` 和约定的重试状态；历史
   `handelflag=4/5` 不会因为进程重启自动回放。因此重启同步前必须先按错误原因统计，再只
   重置确认可安全重试的记录。

本轮 SQL Server 统计快照如下（货主 `PY_OWNER`，时间为本机观察时点；状态会随 Worker 继续
运行变化）：

| 接口表 | 已重置并重放 | 保留的主要失败 | 重启后观察 |
|---|---:|---|---|
| `GoodsInfo` | 1 | 10,696 行 422 映射缺失、2 行 `INVALID_DATA` | 持续有商品入站请求；映射缺失仍返回 500/业务拒绝 |
| `CustomerInfo` | 23,010 | 36 行非权限类拒绝、2 行 `INVALID_DATA` | 已出现 `flag=0/2/5`，说明批量同步正在推进 |
| `SupplierInfo` | 1 | 2 行 `INVALID_DATA` | 权限失败行已进入成功处理状态 |

接口审计抽样窗口记录了 326 次入站请求，其中 322 次 HTTP 200；HTTP 500 请求集中在商品
映射缺失记录，不是 Worker Key 403。完整同步完成仍以源表状态、H8 消息生命周期和 WMS 业务
表落库三者一致为准。

## 7. 验证清单

| 层级 | 验证 | 结果 |
|---|---|---|
| 编译 | `cargo build --manifest-path backend/Cargo.toml -p wms-api -p h8-erp-worker` | 通过 |
| 回归测试 | Worker scope 合并、入站专用 Key 绑定两个定向测试 | 通过 |
| 代码检查 | `git diff --check` | 通过 |
| 治理 | `just gov-t1` | 58/59；唯一失败为现有非同步页面尺寸门禁，H8 相关治理项通过 |
| 运行 | API `/healthz` | HTTP 200 |
| 运行 | 新 Key 访问连接控制面 | HTTP 200 |
| 运行 | Key 使用审计/`last_used_at` | 已更新 |
| 运行 | Worker 控制面循环 | 重启后持续 `inbound=100 outbound=0` |
| 运行 | 入站审计 | 重启后持续产生请求，成功请求为 HTTP 200 |

## 8. 下次重新同步 SOP

1. **先确认依赖和身份**：确认接口库映射仍在、`H8_CONNECTOR_ID` 指向当前 active 连接、
   API 进程的 `WMS_REDIS_URL` 可认证访问；不要先删表或直接改 API Key 表。
2. **创建或轮换 Worker Key**：通过 H1 API Key 管理接口创建/轮换，保留 `h8:worker` 和
   当前接口表消息类型所需的最小业务 scope，`warehouse_ids=[]`，记录 Key ID 和到期时间，
   不记录明文。
3. **重启实际进程**：把明文只注入 Worker 进程环境；API 或 Worker 环境变更后必须重启对应
   进程。旧 Worker 未停止前不要启动新 Worker，避免重复认领。
4. **按错误原因统计源表**：区分 `handelflag` 和 `error_code`。只对已确认的 403/AUTH-005
   权限失败重置为待处理；422 参数映射缺失、`INVALID_DATA` 和业务拒绝必须先修映射或数据。
5. **做小批量探针**：先重置 1–3 行，观察接口审计为 HTTP 200，并确认源表状态和 WMS 落库；
   探针通过后再批量重置同一错误类型。
6. **重启并观察**：重启实际 Worker，检查日志、接口审计和源表状态持续推进；仅 `/healthz`
   通过不能证明同步链路通过。
7. **收口**：确认消息数量、H8 生命周期、业务落库和审计记录；确认新 Key 生效后，再按
   H1 生命周期流程吊销旧 Key。旧 Key 的实际使用方未确认前，不自动吊销。

## 9. 未关闭事项与风险

- 本次新 Key 有有效期，必须在到期前按 H1 轮换；应补入部署平台的到期提醒和轮换责任人。
- 旧的 H8 相关 Key 未因本次创建自动吊销。需要管理员核对调用方、完成平滑轮换后再吊销，
  避免误伤 ERP-facing 通道。
- 本轮重放已启动但不是瞬间完成；客户接口表仍需 Worker 继续处理剩余批次，完成后再做三方
  数量复核。
- 商品接口仍有历史参数映射缺失记录，不能通过重启解决；需补齐商品参数映射后单独重放。
- 本次本机 Worker 仍是 tmux 手工进程；正式环境必须使用 compose/systemd 等已有监督方式，
  并补充部署后的 Worker 心跳和重启证据。
- 历史 Python 脚本本次未删除，也未运行；待确认无回滚或审计依赖后再单独清理，不能与本次
  认证修复混合删除。

## 10. 关联文档

- [H8 接口表同步运行手册](../runbooks/h8-erp-interface-table-sync.md)
- [H1 撤销存储故障运行手册](../runbooks/auth-revocation-degradation.md)
- [H1-006 API Key 生命周期实现与验收切片](../h1-006-api-key-lifecycle-slice.md)
- [H8 ERP 集成用户故事](../domain/user-stories-h8-erp-integration.md)

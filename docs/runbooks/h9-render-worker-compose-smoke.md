# AR-09 Render Worker Compose 隔离烟测

## 目的与边界

本烟测验证 staging Compose 中 `wms-api-staging` 不再等待 H9 Render Worker
`healthy` 才启动。Render Worker 是打印子能力；它不可用时，非打印核心接口仍应可用，分类
PDF 准备接口必须 fail-closed，返回稳定的 `H9_CATEGORY_PDF_RENDER_FAILED`，不能返回一个
看似成功的任务或结果。

本烟测不替代 Windows Print Agent、真实打印机或 Wave 6 S4 证据。Render Worker 与 Print
Agent 必须分别验收。

## 前置输入

无需注入 staging 令牌、订单或实例 URL。脚本使用隔离 PostgreSQL、随机 JWT 和
`WMS_E2E_SEED=1` 启动 `wms-api-e2e`，通过真实 API 创建打印组套、截单并解析组套实例；
不会连接或修改默认 staging 项目。可选的 `WMS_H9_SMOKE_CORE_URL` 仅用于替换非打印核心
接口路径，默认是 `/api/v1/inventory/batches`。

## 执行

```bash
just h9-render-worker-compose-smoke
```

脚本会生成一次性数据库/JWT/H-FILE/Worker 凭据、随机宿主端口和临时 Compose override，
使用唯一的 `COMPOSE_PROJECT_NAME` 启动隔离项目。退出时始终执行 `docker compose down -v`
并删除临时凭据；不会触碰默认 staging 项目或持久卷。API 镜像同时包含仅供该隔离烟测使用的
`wms-api-e2e` 入口，种子数据和实例均随一次性卷销毁。

脚本默认把不含凭据的 evidence manifest 和响应副本写入
`artifacts/h9-render-worker-compose-smoke/<COMPOSE_PROJECT_NAME>/`；可用
`WMS_H9_SMOKE_EVIDENCE_DIR` 指定其他目录。manifest 记录项目名、四个宿主端口、API/Worker
健康检查、鉴权核心接口、Worker 停止时的 HTTP/错误码、持久化失败状态、错误/正确令牌结果、
恢复重试结果和 `down -v --remove-orphans` 清理退出码。数据库密码、JWT、H-FILE 密钥和
Worker 令牌不会写入 evidence。

## 验收断言

脚本按以下顺序失败即退出：

1. 停止状态的 Worker 不应阻塞 API 启动；`/healthz`、`/readyz` 和已认证的非打印核心接口
   返回成功。
2. Worker 未启动时，对同一个 `Idempotency-Key` 调用打印准备接口返回 HTTP 502，JSON
   `code` 必须是 `H9_CATEGORY_PDF_RENDER_FAILED`；随后查询分类 PDF 列表，确认持久化失败状态
   和每个输出的 `processing_status=failed`，不产生可执行的假任务或假 PDF。
3. Worker 启动后 `/healthz` 成功；错误 Bearer token 返回 401，正确 token 的 `/render`
   返回 `application/pdf` 且以 `%PDF-` 开头。
4. 使用原 `Idempotency-Key` 重试同一打印请求返回已完成结果，证明旧失败请求可安全重试且
   不改变实例/输出身份；再次查询列表确认 `preparation_status=completed` 且输出均为
   `processing_status=ready`。

脚本自身的 Compose 配置必须使用非空测试配置；证据应保存项目名、端口、API/Worker
健康检查、失败响应码、恢复重试响应和清理日志。该隔离烟测通过只证明 Render Worker 与核心
API 的 Compose 解耦；仍不得把它写成真实 staging、Print Agent 或物理打印机 S4 通过。

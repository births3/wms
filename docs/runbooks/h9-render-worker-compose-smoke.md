# AR-09 Render Worker Compose 隔离烟测

## 目的与边界

本烟测验证 staging Compose 中 `wms-api-staging` 不再等待 H9 Render Worker
`healthy` 才启动。Render Worker 是打印子能力；它不可用时，非打印核心接口仍应可用，分类
PDF 准备接口必须 fail-closed，返回稳定的 `H9_CATEGORY_PDF_RENDER_FAILED`，不能返回一个
看似成功的任务或结果。

本烟测不替代 Windows Print Agent、真实打印机或 Wave 6 S4 证据。Render Worker 与 Print
Agent 必须分别验收。

## 前置输入

必须在现场注入真实 staging 的已签发令牌和已准备的组套实例，不把令牌、订单数据或 URL
提交到仓库：

```bash
export WMS_H9_SMOKE_AUTHORIZATION='Bearer <staging-token>'
export WMS_H9_SMOKE_PRINT_URL='/api/v1/print-orchestration/suite-instances/<instance-id>/category-pdfs/prepare'
export WMS_H9_SMOKE_CORE_URL='/api/v1/inventory/batches'
# 可选：若准备接口不是标准的 /prepare 后缀，显式提供同一实例的查询 URL
export WMS_H9_SMOKE_LIST_URL='/api/v1/print-orchestration/suite-instances/<instance-id>/category-pdfs'
```

`WMS_H9_SMOKE_AUTHORIZATION` 至少需要非打印核心查询权限和
`h9.print_pdf.prepare`；两个 URL 变量可以填以 `/` 开头的隔离 API 路径，也可以填现场可达的
完整 URL。`WMS_H9_SMOKE_PRINT_URL` 必须指向现场已播种、源单据就绪且使用 `rendered` 分类的
真实实例，禁止填占位 URL。未设置 `WMS_H9_SMOKE_LIST_URL` 时，脚本从准备 URL 去掉
`/prepare` 得到分类 PDF 查询 URL。

## 执行

```bash
just h9-render-worker-compose-smoke
```

脚本会生成一次性数据库/JWT/H-FILE/Worker 凭据、随机宿主端口和临时 Compose override，
使用唯一的 `COMPOSE_PROJECT_NAME` 启动隔离项目。退出时始终执行 `docker compose down -v`
并删除临时凭据；不会触碰默认 staging 项目或持久卷。

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

脚本自身的 Compose 配置必须使用非空测试配置；真实证据还应保存项目名、端口、API/Worker
健康检查、失败响应码、恢复重试响应和清理日志。未实际执行该脚本时，不得把 AR-09 标记为
真实 staging 通过。

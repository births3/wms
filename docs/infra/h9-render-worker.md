# H9 独立浏览器 Render Worker

## 1. 定位

H9 `rendered` 分类由独立 Render Worker 使用冻结的 hiprint 模板和组套实例数据生成 PDF。
WMS API 负责业务编排、幂等、失败状态、H-FILE 留存和 H2 审计；Worker 不持有数据库、
对象存储、队列或业务状态。

```text
H9 service
  -> 内部 HTTP + Bearer token
  -> hiprint 0.4.0 + Chromium
  -> application/pdf
  -> H-FILE store_pdf
```

`external_file` 分类不经过 Worker，继续引用已经摄取并校验的权威 PDF。

## 2. 运行契约

| 项目 | 约束 |
|------|------|
| 健康检查 | `GET /healthz` |
| 渲染 | `POST /render`，请求为 `{ template, data }`，响应为 `application/pdf` |
| 认证 | `Authorization: Bearer <WMS_H9_RENDER_TOKEN>` |
| 默认限制 | 请求体 2 MiB、PDF 20 MiB、并发 2、单次渲染 30 秒 |
| 浏览器网络 | 只允许 Worker 内部空白页及 `data:` / `blob:`；HTTP、HTTPS、file 和脚本 URL 均拒绝 |
| 日志 | 不记录模板正文、业务数据、PDF 或令牌 |

生产 API 使用 `WMS_H9_RENDER_WORKER_URL` 和 `WMS_H9_RENDER_TOKEN`。配置缺失、超时、
非 PDF 响应或浏览器异常均 fail-closed：分类产物标记失败，组套实例保持
`waiting_documents`，不会创建可执行打印任务。

## 3. 部署

- 开发 H2：`deploy/docker-compose.dev-h2.yml`，默认只为本机 API 暴露 `18090`。
- staging：`deploy/docker-compose.staging.yml`，Worker 不暴露宿主端口，仅由
  `wms-api-staging` 通过 Compose 内网访问。
- 容器以非 root 用户运行、根文件系统只读、移除 Linux capabilities，并把 Chromium
  临时文件限制在 512 MiB `tmpfs`。
- Worker 令牌必须独立生成，不与 JWT、H-FILE、数据库或 Print Agent 凭据复用；轮换时同步
  更新 API 与 Worker 后重启两者。

## 4. 验证

```bash
pnpm --dir apps/h9-render-worker test
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --lib print_orchestration::render_worker::tests
pnpm --dir prototypes exec playwright test \
  --config=playwright-web-admin-m1-real-config.ts \
  e2e/web-admin-h9-delivery-note-aggregation-real.spec.ts \
  --grep 'US-H9-008/009'
```

真实浏览器 E2E 使用一次性 PostgreSQL、真实 API、独立 Worker 和进程内 H-FILE，下载
Worker 生成的 PDF 后检查 PDF 图片对象，并从该 PDF 的实际页面图像生成截图证据。该证据只
证明开发软件链路，不替代 staging 对象存储、正式 KMS 或真实打印机 S4 证据。

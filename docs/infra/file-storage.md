# 文件存储 / 附件方案

> 定位：基础设施层文档；本方案已登记为正式横向能力 **H-FILE**（见 [ADR-0031](../adr/0031-file-attachment-capability.md) + architecture-dependencies.md §1.1）
> 关联：模式提炼报告 §5.2 缺口 #3（5 个故事文件需要文件存储）；ADR-0001 技术栈；ADR-0013 配置
> 文档层级：L2 规范

---

## 1. 背景

5 个业务故事文件涉及文件存储需求：

| 模块 | 场景 | 文件类型 |
|------|------|---------|
| M-QL | 质量联系单附件（照片/PDF） | 图片 / PDF |
| M-DI | 药检报告扫描件 | PDF / 图片 |
| M6 | 报表导出（Excel/PDF） | Excel / PDF |
| H7 | 批量导入模板 + 导入文件暂存 | Excel / CSV |
| M1 | 资质证照扫描件 | 图片 / PDF |

---

## 2. 选型

### 方案 A（推荐）：MinIO（S3 兼容）自建

| 维度 | 说明 |
|------|------|
| 协议 | S3 兼容 API |
| 部署 | docker-compose 单节点（小型）/ 分布式（大型） |
| 优势 | 私有化部署（医药数据不出网）+ S3 SDK 通用 + 未来可无缝迁移到云 S3 |
| Rust SDK | `aws-sdk-s3`（MinIO 兼容） |

### 方案 B：云 S3 / OSS

**否决当前阶段**：医药 GSP 数据敏感，多数客户要求私有化部署；但 ADR 不排斥——大型客户可选云 S3，接口一致。

### 方案 C：本地文件系统

**否决**：多副本部署时无法共享；备份/清理困难。

---

## 3. 决策

**采用方案 A：MinIO（S3 兼容）**，接口层用 `aws-sdk-s3`，部署层按客户选 MinIO 或云 S3。

### 3.1 Bucket 规划

| Bucket | 用途 | 访问控制 | 生命周期 |
|--------|------|---------|---------|
| `wms-attachments` | 业务附件与 H9 分类 PDF | 私有（后端按权限代理读取） | 元数据逐文件声明 `gsp_5_year` 或 `short_cache` |
| `wms-exports` | 报表导出临时文件 | 私有 | 7 天自动清理 |
| `wms-imports` | 导入文件暂存 | 私有 | 24h 自动清理 |
| `wms-backups` | H10 数据库备份 | 私有 + 版本控制 | 按 H10 分级保留策略 |

### 3.2 附件关联模型

当前首个正式版本前的已实现切片以
`backend/migrations/202607280001_h_file_h9_category_pdfs.sql` 为唯一建表事实：

- `attachments` 保存 `owner_id`、业务实体、bucket / object key、文件名、MIME、大小、
  SHA-256、文件版本、状态、留存策略、保留/缓存截止时间及确认时间。
- 业务表只保存 `(owner_id, attachment_id)` 外键和覆盖关系，不复制对象存储元数据。
- H9 发票/药检单覆盖关系保存于 `h9_document_file_bindings`；临时访问 URL
  不进入业务事实。
- 当前生产代码只开放 PDF 切片；图片、Excel、CSV 必须在对应故事补齐类型校验、
  扫描、API 与质量矩阵后再扩展，不能绕过 H-FILE 直接落盘。

### 3.3 上传/下载流程

```
H9 分类 PDF 写入：
  H9 service → 独立 hiprint/Chromium Render Worker → H-FILE store_pdf
  H-FILE → 校验 PDF/50MB → 写 pending 元数据 → SSE-S3 PUT
  H-FILE → 确认 ready + SHA-256/大小/留存策略 → H2 上传审计

H9 分类 PDF 读取：
  管理端 → H9 下载/应急打印 API（稳定 attachment_id）
  API → H1 独立权限 → H-FILE GET → 复核大小/SHA-256/PDF
  API → H2 下载/应急打印审计 → 流式返回 PDF
```

M-DI 独立客户平台不得为下载回查 WMS。平台先按客户账号、地址、订单、商品和批号完成本地授权，再签发 15 分钟有效的只读下载 URL。批量下载由异步任务生成 ZIP 和缺失清单；任务凭证只能写自身导出前缀，ZIP 保留 7 天。

临时上传/下载 URL 使用自身会话令牌鉴权，不继承 WMS Bearer 登录态；令牌非法返回
`401 ErrorResponse`，上传授权过期返回 `410 ErrorResponse`。URL 及令牌不得写入审计 diff。

H9 不向浏览器暴露 MinIO object key、长期 URL 或通用预签名端点。其他业务如需大文件
直传，必须另立故事定义确认、扫描和权限边界。
Worker 的进程边界、令牌、网络阻断和部署方式见
[H9 独立浏览器 Render Worker](h9-render-worker.md)。

### 3.4 约束

| 规则 | 说明 |
|------|------|
| 大小限制 | 通用附件单文件 ≤ 50 MB；M-DI 的 JPG/PNG 单文件 ≤ 5 MB、解码后 ≤ 50 MP、任一边 ≤ 12000 px，输入 PDF ≤ 50 MB；客户副本 PDF 软上限 50 MB（审核人说明理由可放行）、绝对上限 100 MB |
| 类型白名单 | `image/jpeg`, `image/png`, `application/pdf`, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`, `text/csv` |
| 病毒扫描 | Wave 3+ 可选 ClamAV 扫描（当前不强制） |
| 加密 | `WMS_HFILE_SSE_MODE` 必须显式为 `none` 或 `aes256`；`aes256` 只在已配置 KMS 的对象存储启用 |
| 审计 | WMS 上传/下载/删除写 H2 审计；独立客户平台的查看、下载、批量导出和授权变更写平台只追加审计 |
| 清理 | `wms-exports` / `wms-imports` 由定时任务清理（coding-standards §3.6） |
| 多货主隔离 | storage key 前缀含 `owner_id`：`{owner_id}/{module}/{entity_type}/{entity_id}/{uuid}.pdf` |

---

## 4. 部署

- 开发 H2 与 staging docker-compose 均提供固定版本 MinIO 和幂等 bucket 初始化；
  `wms-attachments` 保持私有。
- 环境变量：`WMS_HFILE_ENDPOINT` / `WMS_HFILE_ACCESS_KEY` /
  `WMS_HFILE_SECRET_KEY` / `WMS_HFILE_REGION` / `WMS_HFILE_BUCKET` /
  `WMS_HFILE_SSE_MODE`。
- 生产大型客户可替换为云 S3 / OSS，仅改 endpoint + credentials

开发 H2 和当前 staging 单节点 MinIO 未配置 KMS，因此显式使用
`WMS_HFILE_SSE_MODE=none`；这只用于软件链路验收。正式环境必须单独完成 KMS、
`aes256` 写入和读取回归后才能关闭对象存储加密验收项，不能用开发截图替代。

### 4.1 分层验证

| 层级 | 对象存储适配器 | 验证口径 |
|------|----------------|----------|
| Rust 单元 / PostgreSQL 集成 | 显式注入内存对象存储 | PDF、哈希、留存、幂等、失败重试、权限与审计 |
| 管理端真实数据 E2E | E2E 进程内存对象存储，权威 PDF 经 H-FILE 端口写入 | 真实 PostgreSQL + 真实 API + 浏览器交互与截图，不冒充 MinIO/硬件证据 |
| 开发 H2 | `docker-compose.dev-h2.yml` 单节点 MinIO | 本地 S3 兼容联调 |
| staging | `docker-compose.staging.yml` 单节点 MinIO | 预发布对象存储与部署证据；当前开发故事不据此宣称正式环境已验收 |

---

## 5. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-07-25 | v2 | 增加 M-DI 客户平台本地授权、批量导出隔离和药检单专用文件限制 |
| 2026-05-18 | v1 | 初版：MinIO（S3 兼容）+ 4 Bucket + 附件关联模型 + presigned URL 流程 |
| 2026-07-28 | v2 | US-H9-009：落地 H-FILE PDF 切片、统一附件元数据、SSE-S3、后端代理读取、分层验证与开发/staging MinIO |

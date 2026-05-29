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
| `wms-attachments` | 业务附件（资质/药检/质量联系单） | 私有（签名 URL 读取） | 永久（GSP 要求 ≥ 5 年） |
| `wms-exports` | 报表导出临时文件 | 私有 | 7 天自动清理 |
| `wms-imports` | 导入文件暂存 | 私有 | 24h 自动清理 |
| `wms-backups` | H10 数据库备份 | 私有 + 版本控制 | 按 H10 分级保留策略 |

### 3.2 附件关联模型

```sql
CREATE TABLE attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    module TEXT NOT NULL,           -- M-QL / M-DI / M1 / M6 / H7
    entity_type TEXT NOT NULL,      -- quality_liaison / drug_inspection / supplier_cert ...
    entity_id UUID NOT NULL,        -- 关联业务实体 ID
    file_name TEXT NOT NULL,        -- 原始文件名
    content_type TEXT NOT NULL,     -- MIME type
    size_bytes BIGINT NOT NULL,
    storage_key TEXT NOT NULL,      -- S3 object key（bucket/path/uuid.ext）
    uploaded_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_attachments_entity ON attachments(tenant_id, entity_type, entity_id);
```

### 3.3 上传/下载流程

```
上传：
  前端 → POST /api/v1/attachments/presign → 后端生成 presigned PUT URL（5 min TTL）
  前端 → PUT presigned URL → MinIO（直传，不经后端）
  前端 → POST /api/v1/attachments/confirm { storage_key, entity_type, entity_id }
  后端 → 校验文件存在 + 写 attachments 表 + H2 审计

下载：
  前端 → GET /api/v1/attachments/:id/url → 后端生成 presigned GET URL（15 min TTL）
  前端 → GET presigned URL → MinIO（直取）
```

### 3.4 约束

| 规则 | 说明 |
|------|------|
| 大小限制 | 单文件 ≤ 50 MB；超过走分片上传 |
| 类型白名单 | `image/jpeg`, `image/png`, `application/pdf`, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`, `text/csv` |
| 病毒扫描 | Wave 3+ 可选 ClamAV 扫描（当前不强制） |
| 加密 | MinIO 服务端加密（SSE-S3）；传输 HTTPS |
| 审计 | 上传/下载/删除均写 H2 审计追踪 |
| 清理 | `wms-exports` / `wms-imports` 由定时任务清理（coding-standards §3.6） |
| 多租户隔离 | storage_key 前缀含 `tenant_id`：`{tenant_id}/{module}/{entity_id}/{uuid}.{ext}` |

---

## 4. 部署

- docker-compose 加 `minio` 服务（参 ADR-0016 §容器架构）
- 环境变量：`WMS_S3_ENDPOINT` / `WMS_S3_ACCESS_KEY` / `WMS_S3_SECRET_KEY`（走 ADR-0013 secrets）
- 生产大型客户可替换为云 S3 / OSS，仅改 endpoint + credentials

---

## 5. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：MinIO（S3 兼容）+ 4 Bucket + 附件关联模型 + presigned URL 流程 |

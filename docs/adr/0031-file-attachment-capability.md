# ADR-0031：H-FILE 统一附件/文件能力（提升登记）

- 状态：Accepted
- 决策日期：2026-05-29
- 决策人：项目主人
- 关联：infra/file-storage.md（已有基础）/ ADR-0013 配置与 secrets / ADR-0016 部署 / H2 审计 / H7 导入导出 / ADR-0030（横向能力抽象方法论）

---

## 背景

附件/文件需求关键词命中 15 个模块，**经逐模块精读复核：9 个为真附件需求，6 个为误报**（误报中"上传"实为 PDA 离线数据同步、"PDF"实为报表导出/打印，均与文件附件存储无关）。

**9 个真附件需求模块**：M1（资质证照图片/PDF）、M2-verify（实物包装照片 + 稳定性报告 + 电子药检单附件）、M2-asn（稳定性报告）、M3（异常照片）、M-DI（药检报告 PDF/图片）、M-QL（拍照证据）、M5（设备验证报告 PDF/图片）、H-Driver（签收/异常现场照片 hash 存证）、M4-pick（第三方快递员签字图片）。

> 9 个独立模块远超"三次法则"，足以支撑 H-FILE 作为横向能力。

**现状**：`docs/infra/file-storage.md` 已完整定义存储方案（MinIO/S3 + attachments 表 + presigned URL 流程 + 大小/类型/加密/审计约束），但**没有 H 层横向能力编号**——能力实质已设计，却悬空在 infra 文档层，未登记为正式横向能力。

**问题**：缺编号导致两件事——(1) 各业务模块不清楚"附件能力是统一基础设施"，可能各自直连存储；(2) 依赖图横向能力表不完整，治理无法约束"附件必须经 H-FILE"。

---

## 决策

将 `infra/file-storage.md` 的文件存储能力**提升登记为正式横向能力 H-FILE（file-attachment）**，并确立接入契约。

### 契约（所有模块附件需求必须遵守）

| 契约项 | 要求 | 来源 |
|--------|------|------|
| 统一入口 | 所有附件经 H-FILE 的 presign/confirm 接口，禁止业务模块直连存储 | file-storage §3.3 |
| 关联模型 | 统一 attachments 表（module/entity_type/entity_id 关联） | file-storage §3.2 |
| 大小/类型 | 单文件 ≤ 50MB + 类型白名单 | file-storage §3.4 |
| 多租户隔离 | storage_key 前缀含 tenant_id | file-storage §3.4 |
| 审计 | 上传/下载/删除写 H2 审计 | file-storage §3.4 |
| 留存 | GSP 附件 ≥ 5 年（wms-attachments bucket 永久） | file-storage §3.1 |
| 加密 | 服务端加密 SSE-S3 + 传输 HTTPS | file-storage §3.4 |

### 本 ADR 不改变技术选型
存储方案（MinIO/S3）、表结构、流程已由 file-storage.md 定稿，本 ADR 仅做**能力登记 + 契约确立**，不重新设计。

---

## 后果

### 正面
- 横向能力表完整，治理可约束"附件统一收口"。
- 成本最低的一个候选——能力已设计，仅登记 + 补编号。

### 负面/成本
- 需在依赖图横向能力表新增 H-FILE 行；file-storage.md 头部补回指 H-FILE。

### 中立
- 病毒扫描（ClamAV）维持 file-storage §3.4 现状（Wave 3+ 可选，不强制）。

---

## 替代方案（已否决）

| 方案 | 否决理由 |
|------|---------|
| 维持 infra 文档、不设 H 编号 | 能力悬空，治理无法约束，业务模块可能各自直连存储 |
| 并入 H7 导入导出 | H7 是"数据导入导出"，H-FILE 是"附件存储"，关注点不同 |

---

## 待确认事项

1. 已经决策人确认，状态 Proposed → Accepted（2026-05-29）。
2. 已登记进 architecture-dependencies.md §1.1（编号 H-FILE，v3.3）+ file-storage.md 头部回指 H-FILE。
3. 散落复核已完成：15 命中 → 9 真需求 + 6 误报（见背景段）。9 个真需求模块支撑能力成立。

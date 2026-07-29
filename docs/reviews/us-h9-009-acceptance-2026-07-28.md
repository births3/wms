# US-H9-009 验收记录

- 故事：`US-H9-009 分类 PDF 渲染与留存`
- 验收基线：当前工作区（提交后补充本地 commit）
- 验收层级：`S3 开发软件验收`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 按环境分层，不属于本次 S3 软件结论
- 验收日期：2026-07-29
- 整体结论：`PASS`
- 环境边界：正式环境 KMS、SSE-S3 和对象存储恢复证据分层延期，不以开发截图替代

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 rendered 服务端生成并写 H-FILE；external_file 引用权威 PDF | `V0 / V1 / V2 / V3` | Worker 浏览器测试；Rust HTTP 端口测试；PostgreSQL 故事测试；关闭 dev-mock 的 Playwright 下载实际文件 | `apps/h9-render-worker`；`render_worker.rs`；`category_pdf.rs`；真实 PDF 页面截图 | PASS | - |
| AC-2 保存来源、版本、哈希、结果和模板版本 | `V0 / V1 / V2 / V3` | OpenAPI 同步；数据库回读；分类 PDF 页面中文列回读 | `h9_category_pdf_outputs`；`CategoryPdfOutput`；`category-pdfs-ready.png` | PASS | - |
| AC-3 全部/部分选择；完整组套仅临时合并 | `V1 / V2 / V3` | 选择单分类下载、空选择下载全部；断言按冻结顺序合并且附件数不增加 | PostgreSQL `external_pdf_is_referenced...`；Playwright 下载事件 | PASS | - |
| AC-4 分类留存 | `V0 / V1 / V2 / V3` | 数据库 CHECK、H-FILE 回读和页面留存列断言 | rendered `gsp_5_year`；权威文件引用/多文件 `short_cache` 七天 | PASS | - |
| AC-5 失败不入队；同实例同幂等键重试 | `V1 / V2 / V3` | 禁用 H-FILE 触发失败，同键恢复后重试，异键冲突；页面从失败记录回读重试键 | `failed_render_retries_same_instance_output_and_idempotency_key`；OpenAPI 列表重试字段 | PASS | - |
| AC-6 独立权限与 H2 审计 | `V0 / V1 / V2 / V3` | handler 权限反向测试；数据库断言实例入队与准备审计同事务；下载/应急审计回读 | 四个 H1 权限；H2 `prepare/download/emergency_print/upload` 事件 | PASS | - |
| MENU-VISIBILITY | `V0 / V3` | 真实登录后经“基础能力 → H9 打印能力 → 作业·随货同行单归集 → 打印组套”进入；页面按钮权限登记 | 菜单迁移；真实 Playwright spec；分类 PDF 页面截图 | PASS | - |
| UI-SEMANTICS | `V0 / V3` | 中文分类、状态、来源、留存和操作文案；多分类使用复选选择，实例使用单选下拉；请求体断言选择 ID | `H9CategoryPdfPanel.tsx`；`category-pdfs-selection.png` | PASS | - |
| BUSINESS-CONTENT | `V2 / V3` | V2/V3 复用 `OUT-H9-E2E-010`；数据库测试断言冻结源单号，浏览器下载 rendered PDF 后校验 hiprint 图片对象，并从该 PDF 的实际页面图像生成截图 | PostgreSQL 测试；Playwright 实际下载文件；`category-pdf-rendered-document.png` | PASS | - |

## 聚合验证

- V0：`just openapi-check`；质量矩阵检查；页面 self-check；`just gov-t1`。
- V1：Worker 真实 Chromium 渲染 2/2；Rust HTTP 端口 2/2；`pdf_document`
  临时合并；handler 权限 2/2；H9 PostgreSQL 9/9。
- V2：隔离 PostgreSQL 覆盖真实源数据、外部权威 PDF、
  顺序、哈希、留存、失败、不入队、同键重试和临时合并不落库。
- V3：一次性真实 PostgreSQL + API + 独立 Render Worker + Web 管理端 E2E 通过；
  下载文件校验 `%PDF-`、体积和 PDF 图片对象，再从同一 PDF 嵌入页图生成浏览器截图，
  可见 `出库单号：OUT-H9-E2E-010`，不以 mock 响应或模板预览代替。
- V3 界面语义：实例单选、分类复选，中文来源/状态/留存与按钮，API 选择参数和结果一致。
- V3 业务内容：组套实例、分类 PDF 元数据和浏览器下载文件复用同一真实业务键。
- V4：正式 KMS/S3 写入、读取和恢复属于目标环境证据，本次不冒充。
- 页面截图：
  - `artifacts/screenshot-portal/real-web/h9-category-pdfs/category-pdfs-ready.png`
  - `artifacts/screenshot-portal/real-web/h9-category-pdfs/category-pdfs-selection.png`
  - `artifacts/screenshot-portal/real-web/h9-category-pdfs/category-pdf-rendered-document.png`

## H-FILE 与环境边界

- 统一附件元数据为 `attachments`，H9 覆盖关系为 `h9_document_file_bindings`；
  已删除 `h9_ingested_document_files` 占位表。
- 对象 key 含 `owner_id/module/entity_type/entity_id`；前端不接触 bucket、object key
  或长期 URL。
- 开发 H2 和 staging compose 已增加固定版本 MinIO、私有 bucket 初始化和显式
  `WMS_HFILE_SSE_MODE=none`。MinIO 的 SSE-S3 需要 KMS；正式环境必须另行验证 KMS、
  `aes256` 写入/读取和恢复，当前仅完成支持该模式的 S3 适配器。

## Review 修复记录

1. 错误使用宽范围 Cargo 测试导致构建缓存占满磁盘；改为 `cargo clean -p wms-api`
   后只编译 `--lib` 和目标集成测试。
2. 首轮列表按创建时间/UUID 排序，可能打乱组套顺序；已改为关联冻结实例项并按
   `sort_order` 排序，全部/部分临时合并沿用同一顺序。
3. E2E 最初读取已被浏览器消费的网络响应体；已改为校验浏览器实际下载文件。
4. MinIO 未配置 KMS 时强制 `AES256` 会使开发上传失败；改为必须显式配置
   `none|aes256`，并将正式加密证据留在正式环境层。
5. 首轮准备成功先提交实例状态、后追加审计；已改为准备结果、实例入队和 H2 审计
   在同一事务提交，避免“可打印但无审计”。
6. H-FILE 元数据确认或上传审计失败时曾遗留对象；已增加 S3/内存适配器补偿删除并将
   元数据标记为失败。
7. 治理复查补齐 MkDocs 验收文档导航和分类 PDF DataGrid 隐藏创建时间系统列。
8. 复审发现 `rendered` 路径只生成 ASCII 单行占位 PDF，未执行冻结 hiprint 模板；已改为
   独立 Chromium Worker，Rust 只传入模板/冻结数据并将真实 PDF 写入 H-FILE。
9. hiprint/jsPDF 的合法 PDF 以 `%%EOF` 结束但不保证尾部换行；H-FILE 原校验器误拒绝。
   已按 PDF 尾标记和可选尾部空白校验，并补单元回归。
10. Chromium PDF 插件截图不包含插件内部页面；已改为从 E2E 实际下载 PDF 的嵌入页面图
    生成浏览器截图，避免用黑色插件外壳冒充业务证据。
11. 首轮独立 Worker 容器文件沿用 `Dockerfile`，触发前端路径 kebab-case 门禁；已改为
    小写 `dockerfile` 并同步两套 compose，`just gov-t1` 恢复 56/56。

## 范围声明

本记录只证明 US-H9-009 开发软件故事完成，不代表 US-H9-010 打印任务队列、
US-H9-012～015 Print Agent/Windows 客户端或真实打印机 S4 已完成。

# ADR-0045：首版前保留 PostgreSQL migration 链

- 状态：Accepted
- 日期：2026-08-01
- 决策者：项目主人
- 关联：ADR-0038、ADR-0016、`docs/database/table-catalog.md`

## 背景

当前仓库包含 114 个按顺序执行的 PostgreSQL migration。它们已经形成可执行的
schema、约束、索引和确定性种子数据；表目录由同一条 migration 链生成。首个正式版本
尚未发布，但本地数据库可能停留在较早的 migration，dev/staging 的运行状态也必须以
环境证据为准，不能靠猜测或直接探测生产数据库解决。

需要在首版基线建立前决定：继续维护现有链，还是把当前 schema 压缩成一条新 baseline。

## 候选方案

1. **保留现有链**：仓库、local/dev/staging 都执行同一组有序 migration；本地可丢弃
   数据库可以重建。
2. **压缩为新 baseline**：删除或重写历史 migration，并要求各环境先完成审批、备份、
   证据作废和重建。

## 决策

采用方案 1，保留现有 migration 链。

- 不删除、合并、改写或重新编号已有 migration。
- 新 migration 继续追加到 `backend/migrations/*.sql`，由 SQLx 按版本顺序执行。
- 仓库、local、dev、staging 使用同一条链；不得让 staging 长期运行另一条 baseline。
- 对已有 schema 只执行缺失的后续 migration，不把新 baseline 直接覆盖到已有 schema。
- `docs/database/table-catalog.md` 继续由
  `python3 scripts/governance/generate_table_catalog.py` 生成，不建立第二份表目录。

## 环境盘点边界

| 环境 | 当前可验证事实 | 处理规则 |
|---|---|---|
| 仓库 | 114 个 migration，最新文件为 `202607280001_h_file_h9_category_pdfs.sql` | 作为唯一迁移链来源 |
| local/test | `.env` 指向 `127.0.0.1:5434/wms_test`；现场 `_sqlx_migrations` 有 32 条，最新为 `202607120007` | 数据可丢弃；需要时从零重建 |
| dev | 本仓库没有可提交的实时连接或应用历史；运行手册要求使用真实 dev DB 证据 | 发布前必须用环境证据核对 `_sqlx_migrations`，不能用 local 结果替代 |
| staging | 现有 runbook 中的 38 张表结果是历史记录，不代表当前 schema | 部署前重新核对 migration、备份和证据；禁止未经批准销毁重建 |
| production | 本次不探测、不修改 | 由正式发布流程和 ADR-0016 管理 |

## 空库基线证据

从空 PostgreSQL 执行当前 migration 链的回归测试固定验证：

- 192 个静态关系、564 个索引、1159 个约束；
- schema fingerprint `63fd1daab6ad7b04d1bea02b310e8ca7`；
- 种子契约：126 个权限、11 个系统字典分类、46 个系统字典项。

证据入口：

```bash
python3 scripts/governance/generate_table_catalog.py --check --json
python3 -m pytest scripts/governance/tests/test_generate_table_catalog.py -q
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test schema_baseline_postgres -- --test-threads=1
```

## 后果

- 正面：迁移历史、表归属和图谱输入保持可追溯；local/dev/staging 不会出现两条 schema
  演进路径。
- 负面：首版前的本地旧数据库需要重建；跨 migration 的表所有权问题继续由目录和审查
  记录治理，而不是通过压缩历史隐藏。
- 风险：dev/staging 的实时应用版本仍需在发布前补齐环境证据；证据缺失时不得宣称
  migration 已同步。

## 重新评估条件

只有在首个正式版本基线需要建立、或 staging 已发生不可恢复的链分叉时，才重新评估
baseline。届时必须先完成审批、备份、证据影响评估和可回滚方案，不能在已有 schema 上
直接运行新 baseline。

## 参考

- [ADR-0038：首个正式版本前不保留版本兼容层](0038-pre-v1-compatibility-policy.md)
- [ADR-0016：部署形态](0016-deployment.md)
- [数据库表目录](../database/table-catalog.md)
- [Wave 6 staging 部署演练](../runbooks/wave-6-staging-deploy-dry-run.md)

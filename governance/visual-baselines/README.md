# 视觉对照基线（visual-baselines）

> 入仓的 PNG = 业务方走查 approved 的"老版本"
> 每次 UI 改动后跑 `check_visual_regression.py`，对比新截图 vs 这里的 baseline

## 何时更新 baseline

- ✅ 新增 P0 原型页 → 跑 `capture_visual_snapshots.py` → 用 `accept_baseline.py` 接受
- ✅ 业务方批准的视觉调整 → 同上（中/大变化按门禁规则带 `--confirm-medium` 或 `--force-major`）
- ❌ **回归 bug 不能更新 baseline**（先修代码，让脚本回到 0 差异）

## 工作流

### 推荐：用 accept_baseline.py 工具（带 4 类接受门禁）

```bash
# 1. 起 vite dev
cd prototypes && pnpm dev &

# 2. 截当前快照
python3 scripts/governance/capture_visual_snapshots.py

# 3. dry-run：哪些可接受、哪些被拒、原因（详见 docs/prototypes/baseline-acceptance.md）
python3 scripts/governance/accept_baseline.py --reviewer="项目主人"

# 4a. 全部通过 → apply
python3 scripts/governance/accept_baseline.py --apply --reviewer="项目主人"

# 4b. 中等变化（mean_diff 5-30）→ 浏览器确认后
python3 scripts/governance/accept_baseline.py --apply --reviewer="项目主人" --confirm-medium

# 4c. 大变化（mean_diff > 30 或 pixel_ratio > 30%）→ 必须人工确认
python3 scripts/governance/accept_baseline.py --apply --reviewer="项目主人" --force-major

# 单 tab 模式：加 --tab=NAME
python3 scripts/governance/accept_baseline.py --apply --reviewer="项目主人" --tab=h2-audit
```

### 已废弃（不要用）

```bash
# ❌ 裸 cp 绕过接受门禁，会让退化图静默成为 baseline
# cp prototypes/.visual-snapshots/*.png governance/visual-baselines/
```

## 接受标准摘要

详见 `docs/prototypes/baseline-acceptance.md`。

| 类别 | 标准 | 强制度 |
|---|---|---|
| **A 结构健康** | 文件 1KB-3MB / OCR 关键字命中 / 底部不截断 | 强制（无 force 选项）|
| **B 变化幅度** | small 自动 / medium 需 confirm / major 需 force | 分档强制 |
| **C 签字** | --reviewer 必填非占位 / reviewed_at 自动更新 | 强制 |
| **D 字段完整** | tab 在 Tabs.tsx + manifest 注册 | 强制（由 baseline_completeness 治理）|

## 检查阈值

| 指标 | 阈值 |
|---|---|
| `mean_diff`（64×64 灰度均值差，0-255）| ≤ 5 small / 5-30 medium / > 30 major |
| `pixel_diff_ratio` | ≤ 30% medium / > 30% major |
| 文件大小 | 1 KB - 3 MB |
| OCR 字符数 | ≥ 100（兜底；hits ≥ 1 时不强制） |
| 底部 30 行非白 | < 5%（防截断）|

## 治理脚本一览

| 脚本 | Tier | 作用 |
|---|---|---|
| `check_baseline_completeness.py` | T1 | Tabs.tsx ↔ manifest ↔ PNG 三者一致 + 字段必填 |
| `accept_baseline.py` | 工具 | 候选图能否接受为新 baseline（4 类标准） |
| `capture_visual_snapshots.py` | 工具 | chrome headless 截图 |
| `check_visual_regression.py` | T3 | baseline ↔ snapshot 像素差 + 截断检测 |
| `check_visual_keywords.py` | T3 | OCR 关键字 sanity check |

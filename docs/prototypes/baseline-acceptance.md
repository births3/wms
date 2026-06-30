# 视觉基线接受标准（baseline acceptance）

> 治理目的：防止"裸 cp 把退化图覆盖到 baseline"导致视觉债务静默积累。
> 工具：`scripts/governance/accept_baseline.py`（默认 dry-run，必须显式 --apply）

## 接受标准（必须全部通过）

### A. 结构健康（自动判定）

| ID | 检查 | 阈值 | 失败原因 |
|---|---|---|---|
| A1 | 文件大小 | 1 KB ≤ size ≤ 3 MB | 截图失败 / 图爆炸 |
| A2 | 底部截断检测 | 底部 30 行非白 < 5% | 内容超出 viewport |

### B. 变化幅度（与现有 baseline 对比）

| ID | 变化档位 | 判定 | 接受方式 |
|---|---|---|---|
| B1 | identical（MD5 一致）| 0 差异 | 自动接受 |
| B2 | small | mean_diff ≤ 5 + pixel_ratio < 30% | 自动接受 |
| B3 | medium | 5 < mean_diff ≤ 30 | 必须 `--confirm-medium` |
| B4 | major | mean_diff > 30 或 pixel_ratio > 30% | 必须 `--force-major`（人工确认后） |
| B5 | resize | baseline 与 candidate 尺寸不同（manifest viewport 改了但 PNG 未跟上） | 必须 `--accept-resize`（人工确认 viewport 是预期变化） |

### C. 签字（强制）

| ID | 检查 | 要求 |
|---|---|---|
| C1 | reviewer 必填 | `--reviewer="<具体姓名>"` 且非占位符 |
| C2 | reviewed_at 自动更新 | 接受时同步写入今天 |

### D. 字段完整（自动判定）

| ID | 检查 | 失败原因 |
|---|---|---|
| D1 | tab 在 Tabs.tsx | 否则是孤儿（不接受） |
| D2 | manifest 有 [[snapshots]] | 否则未注册（不接受） |
> D1/D2 由 `check_baseline_completeness.py` 单独治理；accept_baseline 假定它们已通过。

## 工作流

### 标准场景：UI 改动后接受新 baseline

```bash
# 1. 起 vite + 截图
cd prototypes && pnpm dev &
python3 scripts/governance/capture_visual_snapshots.py

# 2. dry-run 看哪些可接受
python3 scripts/governance/accept_baseline.py --reviewer="<your-name>"

# 3. 全部通过且变化都是 small → 直接 apply
python3 scripts/governance/accept_baseline.py --apply --reviewer="<your-name>"

# 4. 中等变化（B3） → 浏览器确认后加 --confirm-medium
python3 scripts/governance/accept_baseline.py --apply --reviewer="<name>" --confirm-medium

# 5. 大变化（B4） → 浏览器逐张确认后加 --force-major
python3 scripts/governance/accept_baseline.py --apply --reviewer="<name>" --force-major

# 5b. 尺寸变化（B5，manifest viewport 改了 + PNG 未跟上） → 加 --accept-resize
python3 scripts/governance/accept_baseline.py --apply --reviewer="<name>" --accept-resize

# 6. 单 tab：加 --tab=NAME
python3 scripts/governance/accept_baseline.py --apply --reviewer="<name>" --tab=h2-audit
```

### 拒绝场景：A 类结构问题

`--force-major` 也跳不过 A 类（结构健康）。修代码或调 viewport 重新截，不能强行接受坏图。

## 与其他治理的关系

```
┌─────────────────────────────────────────────────────────┐
│ 静态门禁（T1）                                            │
│ check_baseline_completeness.py                            │
│   - Tabs.tsx ↔ manifest ↔ PNG 三者一致                     │
│   - reviewed_by + reviewed_at 必填                          │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│ 接受门禁（人工触发的工具）                                  │
│ accept_baseline.py（本规范）                                │
│   - 候选图能否替换为新 baseline                              │
│   - 4 类标准（A 健康 + B 变化幅度 + C 签字 + D 完整）       │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│ 持续验证（T3）                                              │
│ check_visual_regression.py（baseline ↔ 当前 snapshot 差异）  │
└─────────────────────────────────────────────────────────┘
```

## 红线

- ❌ 不允许 `cp prototypes/.visual-snapshots/*.png governance/visual-baselines/`（裸 cp 绕过门禁）
- ❌ 不允许把 `--reviewer` 填成 "TODO" / "?" / "" 等占位符
- ❌ 不允许 `--force-major` 跳过 A 类结构问题（A 类无 force 选项）
- ✅ 允许大变化但必须人工浏览器确认 + `--force-major`
- ✅ 允许批量 apply（每张图各自评估，按规则独立通过/拒绝）

## 阈值依据

| 阈值 | 取值 | 依据 |
|---|---|---|
| TRUNCATION = 5% | 底部 30 行非白比例 | 实测：所有完整页 = 0%；截断时 ≥ 13% |
| SMALL_CHANGE = 5.0 | 64×64 灰度 mean_diff | 实测：色调微调 1.2，无明显视觉变化 |
| LARGE_CHANGE = 30.0 | mean_diff | 实测：主题色变深 25%；整页变暗 50% mean_diff > 100 |
| LARGE_PIXEL_RATIO = 30% | 像素差异比例 | 实测：导航重构后所有页 pixel_ratio ≈ 15-45%（重构变化）|

阈值如需调整，更新本文档并 commit。

## 演进

- v1（2026-05-23）：初版，4 类标准 + 5 档变化幅度
- 未来：加 perceptual hash（imagehash phash）做更智能的相似度判定（当前仅 PIL 像素差）

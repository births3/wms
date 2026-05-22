# 视觉对照基线（visual-baselines）

> 入仓的 PNG = 业务方走查 approved 的"老版本"
> 每次 UI 改动后跑 `check_visual_regression.py`，对比新截图 vs 这里的 baseline

## 何时更新 baseline

- ✅ 新增 P0 原型页 → 跑 `capture_visual_snapshots.py` 生成 PNG → cp 到这里 → 更新 manifest
- ✅ 业务方批准的视觉调整 → 同上
- ❌ **回归 bug 不能更新 baseline**（先修代码，让脚本回到 0 差异）

## 工作流

```bash
# 1. 起 vite dev
cd prototypes && pnpm dev &

# 2. 截当前快照
python3 scripts/governance/capture_visual_snapshots.py

# 3. 对比 baseline ↔ snapshot
python3 scripts/governance/check_visual_regression.py
# → 0 差异：全绿
# → 有差异：列出文件 + 写 diff 图到 prototypes/.visual-diffs/

# 4. 看 diff 图，判断"预期 vs 退化"
#    - 预期视觉变化（如改了主题色）→ 更新 baseline：cp prototypes/.visual-snapshots/*.png governance/visual-baselines/
#    - 退化（如组件错位、颜色错乱）→ 修代码，再跑
```

## 阈值（参 check_visual_regression.py）

- mean_diff（缩放 64×64 灰度后均值差）：≤ 2 通过 / 2-10 警告 / > 10 错误
- pixel_diff_ratio（像素级不同比例）：≤ 0.5% 通过 / 0.5%-3% 警告 / > 3% 错误
- 任一指标 error → PR 阻断

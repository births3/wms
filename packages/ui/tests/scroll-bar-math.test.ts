import assert from "node:assert/strict";
import {
  computeScrollBarMetrics,
  scrollLeftForDrag,
  scrollLeftForJump,
  scrollLeftForKey,
} from "../src/ui/scroll-bar-math.ts";

// 指标计算：800/2000/300 → maxLeft=1200、ratio=0.4、thumb=40%、thumbLeft=15%、aria=300
{
  const metrics = computeScrollBarMetrics(800, 2000, 300);
  assert.equal(metrics.scrollable, true);
  assert.equal(metrics.ratio, 0.4);
  assert.equal(metrics.maxLeft, 1200);
  assert.equal(metrics.thumbWidthPct, 40);
  assert.equal(metrics.thumbLeftPct, 15);
  assert.equal(Math.round(metrics.left), 300);
}

// 滑块宽度下钳制：内容极宽（ratio 极小）时滑块 ≥6%
{
  const metrics = computeScrollBarMetrics(800, 80000, 0);
  assert.equal(metrics.scrollable, true);
  assert.equal(metrics.thumbWidthPct, 6);
}

// 内容只比容器宽一点时滑块接近铺满（与原生滚动条一致的等比表现）
{
  const metrics = computeScrollBarMetrics(800, 840, 10);
  assert.equal(metrics.scrollable, true);
  assert.equal(metrics.thumbWidthPct, 95.23809523809523);
}

// 空容器边界：0/0 → ratio 兜底 1，不产生 NaN
{
  const metrics = computeScrollBarMetrics(0, 0, 0);
  assert.equal(metrics.scrollable, false);
  assert.equal(metrics.ratio, 1);
  assert.equal(metrics.maxLeft, 0);
}

// 不可滚动边界：scrollWidth ≤ clientWidth → scrollable=false、maxLeft=0、thumb 铺满
{
  const equal = computeScrollBarMetrics(800, 800, 0);
  assert.equal(equal.scrollable, false);
  assert.equal(equal.maxLeft, 0);
  assert.equal(equal.thumbLeftPct, 0);

  const narrower = computeScrollBarMetrics(800, 700, 0);
  assert.equal(narrower.scrollable, false);
  assert.equal(narrower.maxLeft, 0);
}

// scrollLeft 越界时 left 保留原始值（事件源透传），滑块偏移渲染钳制在轨道内
{
  const metrics = computeScrollBarMetrics(800, 2000, 1500);
  assert.equal(metrics.left, 1500);
  assert.equal(metrics.thumbLeftPct, 60);
}

// 轨道点击跳转：中点 → maxLeft/2；越界钳制
{
  const mid = scrollLeftForJump(500, 100, 800, 1200);
  assert.equal(mid, 600);

  const before = scrollLeftForJump(50, 100, 800, 1200);
  assert.equal(before, 0);

  const after = scrollLeftForJump(1000, 100, 800, 1200);
  assert.equal(after, 1200);
}

// 拖动：thumb 位移 1px 对应内容 scrollWidth/clientWidth px（1/ratio），WIP 灵敏度语义
{
  const forward = scrollLeftForDrag(300, 40, 0.4);
  assert.equal(forward, 400);

  const backward = scrollLeftForDrag(300, -40, 0.4);
  assert.equal(backward, 200);

  const ratioOne = scrollLeftForDrag(300, 40, 1);
  assert.equal(ratioOne, 340);
}

// 键盘步进与钳制
{
  const maxLeft = 1200;
  assert.equal(scrollLeftForKey("ArrowRight", 0, 800, maxLeft), 80);
  assert.equal(scrollLeftForKey("ArrowRight", 1150, 800, maxLeft), 1200);
  assert.equal(scrollLeftForKey("ArrowLeft", 50, 800, maxLeft), 0);
  assert.equal(scrollLeftForKey("PageDown", 100, 800, maxLeft), 900);
  assert.equal(scrollLeftForKey("PageDown", 1100, 800, maxLeft), 1200);
  assert.equal(scrollLeftForKey("PageUp", 100, 800, maxLeft), 0);
  assert.equal(scrollLeftForKey("Home", 500, 800, maxLeft), 0);
  assert.equal(scrollLeftForKey("End", 500, 800, maxLeft), 1200);
}

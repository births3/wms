import assert from "node:assert/strict";
import {
  computeScrollAreaMaxHeight,
  SCROLL_AREA_BOTTOM_MARGIN_REM,
  SCROLL_AREA_MIN_MAX_HEIGHT_REM,
} from "../src/lib/viewport-math.ts";

const REM_PX = 16;

// 正常值：视口 - 表格顶部偏移 - 底部栏高度 - 底部预留
{
  const height = computeScrollAreaMaxHeight(900, 200, 56);
  assert.equal(height, 900 - 200 - 56 - SCROLL_AREA_BOTTOM_MARGIN_REM * REM_PX);
}

// 底部栏高度为 0（无 footer 分支）时同样按公式计算
{
  const height = computeScrollAreaMaxHeight(900, 200, 0);
  assert.equal(height, 900 - 200 - SCROLL_AREA_BOTTOM_MARGIN_REM * REM_PX);
}

// 过低/负值钳制到最小高度（8rem），避免滚动区被压没
{
  assert.equal(computeScrollAreaMaxHeight(300, 250, 60), SCROLL_AREA_MIN_MAX_HEIGHT_REM * REM_PX);
  assert.equal(computeScrollAreaMaxHeight(200, 250, 60), SCROLL_AREA_MIN_MAX_HEIGHT_REM * REM_PX);
  assert.equal(computeScrollAreaMaxHeight(100, 200, 0), SCROLL_AREA_MIN_MAX_HEIGHT_REM * REM_PX);
}

// 单调性：视口越大 / 顶部偏移越小 → 可用高度越大
{
  const taller = computeScrollAreaMaxHeight(1000, 200, 56);
  const shorter = computeScrollAreaMaxHeight(900, 200, 56);
  assert.ok(taller > shorter);

  const topLower = computeScrollAreaMaxHeight(900, 150, 56);
  const topHigher = computeScrollAreaMaxHeight(900, 200, 56);
  assert.ok(topLower > topHigher);
}

// 视口低于最小高度时仍保底（极端小屏）
{
  assert.equal(computeScrollAreaMaxHeight(100, 50, 40), SCROLL_AREA_MIN_MAX_HEIGHT_REM * REM_PX);
}

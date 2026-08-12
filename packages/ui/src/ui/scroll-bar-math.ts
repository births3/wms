/**
 * scroll-bar-math — ScrollBar 滚动条的纯计算函数（无 DOM，供 node 无 jsdom 单测）
 *
 * 语义与原生横向滚动条一致：thumb 位移 1px 对应内容 scrollWidth/clientWidth px（1/ratio）。
 */

export interface ScrollBarMetrics {
  /** 当前 scrollLeft（来自 scroll 事件源，不做钳制） */
  left: number;
  /** 内容宽于容器时可滚动 */
  scrollable: boolean;
  /** clientWidth / scrollWidth（拖动灵敏度基准） */
  ratio: number;
  /** 最大可滚动距离 */
  maxLeft: number;
  /** 滑块宽度百分比（下钳 6%） */
  thumbWidthPct: number;
  /** 滑块左偏移百分比 */
  thumbLeftPct: number;
}

export function computeScrollBarMetrics(clientWidth: number, scrollWidth: number, left: number): ScrollBarMetrics {
  const scrollable = scrollWidth > clientWidth;
  // scrollWidth 为 0（初始/空容器）时 ratio 兜底 1，避免 NaN 污染滑块样式
  const ratio = scrollWidth > 0 ? clientWidth / scrollWidth : 1;
  // 内容比容器窄时 maxLeft 为负，钳制到 0（aria-valuemax 不允许小于 valuemin）
  const maxLeft = Math.max(scrollWidth - clientWidth, 0);
  const thumbWidthPct = Math.max(ratio * 100, 6);
  // 渲染钳制：scrollLeft 瞬时越界时滑块不溢出轨道
  const thumbLeftPct = maxLeft > 0 ? clamp((left / maxLeft) * (100 - thumbWidthPct), 0, 100 - thumbWidthPct) : 0;
  return { left, scrollable, ratio, maxLeft, thumbWidthPct, thumbLeftPct };
}

/** 点击轨道跳转：按点击位置在轨道上的比例换算 scrollLeft，越界钳制 */
export function scrollLeftForJump(clientX: number, trackLeft: number, trackWidth: number, maxLeft: number): number {
  const pct = (clientX - trackLeft) / trackWidth;
  return clamp(pct, 0, 1) * maxLeft;
}

/**
 * 拖动滑块：thumb 位移 1px 对应内容 1/ratio px，与原生滚动条一致
 * （保留 WMS 既有灵敏度语义：滑块拖动 1px 表头内容移动 scrollWidth/clientWidth px）
 */
export function scrollLeftForDrag(startLeft: number, dx: number, ratio: number): number {
  return startLeft + dx / ratio;
}

export type ScrollBarKey = "ArrowLeft" | "ArrowRight" | "PageUp" | "PageDown" | "Home" | "End";

/** 键盘步进：方向键 ±10% 视宽、翻页 ±100% 视宽、Home/End 到两端，结果钳制在 [0, maxLeft] */
export function scrollLeftForKey(key: ScrollBarKey, left: number, clientWidth: number, maxLeft: number): number {
  const step = (percent: number) => left + clientWidth * percent;
  switch (key) {
    case "ArrowLeft":
      return clamp(step(-0.1), 0, maxLeft);
    case "ArrowRight":
      return clamp(step(0.1), 0, maxLeft);
    case "PageUp":
      return clamp(step(-1), 0, maxLeft);
    case "PageDown":
      return clamp(step(1), 0, maxLeft);
    case "Home":
      return 0;
    case "End":
      return maxLeft;
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

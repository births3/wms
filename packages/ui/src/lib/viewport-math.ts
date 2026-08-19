/**
 * viewport-math — 列表滚动区视口高度测量的纯计算函数（无 DOM，供 node 无 jsdom 单测）
 *
 * 替代历史魔法数 max-h-[calc(100vh-23rem)]：按滚动区实际顶部偏移与底部栏实测高度计算，
 * 保证悬停表格时滚轮只滚动列表数据、页面不因表格内容产生滚动。
 */

/** 滚动区最小高度兜底（rem）：极端小屏/负值时保底，避免滚动区被压没 */
export const SCROLL_AREA_MIN_MAX_HEIGHT_REM = 8;
/** 滚动区底部预留边距（rem）：表格底与视口底之间的呼吸空间 */
export const SCROLL_AREA_BOTTOM_MARGIN_REM = 0.5;

const REM_PX = 16;

/**
 * 滚动区可用最大高度 = 视口 - 滚动区顶部偏移 - 底部栏高度 - 底部预留；
 * 结果钳制到最小高度。全部数值均为 px。
 */
export function computeScrollAreaMaxHeight(
  viewportHeight: number,
  scrollAreaTop: number,
  bottomBarHeight: number,
): number {
  const raw = viewportHeight - scrollAreaTop - bottomBarHeight - SCROLL_AREA_BOTTOM_MARGIN_REM * REM_PX;
  return Math.max(raw, SCROLL_AREA_MIN_MAX_HEIGHT_REM * REM_PX);
}

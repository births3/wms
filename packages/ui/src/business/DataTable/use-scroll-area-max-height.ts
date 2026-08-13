import * as React from "react";
import { computeScrollAreaMaxHeight } from "../../lib/viewport-math";

/**
 * useScrollAreaMaxHeight — 列表滚动区视口高度测量 hook
 *
 * 列表自管纵向滚动：滚动区最大高度 = 视口 - 滚动区顶部偏移 - 底部栏实测高度 - 底部预留，
 * 悬停表格时滚轮只滚动列表数据、页面不因表格内容产生滚动。替代历史魔法数 calc(100vh-23rem)。
 *
 * 触发点：mount 首帧前同步测量（useLayoutEffect）、窗口 resize、页面 scroll（capture）、
 * root 与底部栏尺寸变化（ResizeObserver）。工具栏/筛选 chips 换行仅位移 root 时 RO 不触发，
 * 短暂超高 → 页面产生滚动 → scroll 事件重测收敛（自愈）。
 *
 * 返回值含 maxHeight 与 minHeight 两个测量值：
 * - stable 触发（首帧/resize/RO）：两者同时更新 —— min 锚点随视口收缩重锚定，窗口放大后"内容少撑满"重新生效；
 * - scroll 触发：只更新 maxHeight —— min 冻结，避免页面滚动时表格高度随视口相对位置伸缩（内容少时页脚稳定）。
 */
export function useScrollAreaMaxHeight(
  maxHeightProp: string | number | undefined,
  scrollAreaRef: React.RefObject<HTMLDivElement | null>,
  bottomBarRef: React.RefObject<HTMLDivElement | null>,
  rootRef: React.RefObject<HTMLDivElement | null>,
): { maxHeight: string | undefined; minHeight: string | undefined } {
  const [measuredMax, setMeasuredMax] = React.useState<string | undefined>(undefined);
  const [measuredMin, setMeasuredMin] = React.useState<string | undefined>(undefined);

  React.useLayoutEffect(() => {
    // 显式 maxHeight 优先：跳过测量，直接由调用方应用
    if (maxHeightProp !== undefined) return;

    const measure = (kind: "stable" | "scroll") => {
      const area = scrollAreaRef.current;
      if (!area) return;
      const top = area.getBoundingClientRect().top;
      const bottomBarHeight = bottomBarRef.current?.getBoundingClientRect().height ?? 0;
      const value = `${computeScrollAreaMaxHeight(window.innerHeight, top, bottomBarHeight)}px`;
      setMeasuredMax(value);
      if (kind === "stable") {
        setMeasuredMin(value);
      } else {
        // scroll 触发：min 单调钳制（只降不升），保证 min 永不击穿 maxHeight（min-height 优先于 max-height），
        // 同时保持"内容少时页脚稳定"的冻结语义（不随页面滚动上升）
        setMeasuredMin((prev) => (prev === undefined || parseFloat(value) < parseFloat(prev) ? value : prev));
      }
    };
    // 具名引用：add/removeEventListener 必须使用同一函数实例，否则移除失效导致监听器泄漏
    const onResize = () => measure("stable");
    const onScroll = () => measure("scroll");

    measure("stable");
    window.addEventListener("resize", onResize);
    // capture：页面滚动改变滚动区顶部偏移时重测（min 单调钳制）
    window.addEventListener("scroll", onScroll, { capture: true, passive: true });

    if (typeof ResizeObserver !== "undefined") {
      const observer = new ResizeObserver(onResize);
      if (rootRef.current) observer.observe(rootRef.current);
      if (bottomBarRef.current) observer.observe(bottomBarRef.current);
      return () => {
        window.removeEventListener("resize", onResize);
        window.removeEventListener("scroll", onScroll, { capture: true } as EventListenerOptions);
        observer.disconnect();
      };
    }

    return () => {
      window.removeEventListener("resize", onResize);
      window.removeEventListener("scroll", onScroll, { capture: true } as EventListenerOptions);
    };
  }, [maxHeightProp, scrollAreaRef, bottomBarRef, rootRef]);

  return { maxHeight: measuredMax, minHeight: measuredMin };
}

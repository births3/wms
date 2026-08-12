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
 */
export function useScrollAreaMaxHeight(
  maxHeightProp: string | number | undefined,
  scrollAreaRef: React.RefObject<HTMLDivElement | null>,
  bottomBarRef: React.RefObject<HTMLDivElement | null>,
  rootRef: React.RefObject<HTMLDivElement | null>,
): string | undefined {
  const [measured, setMeasured] = React.useState<string | undefined>(undefined);

  React.useLayoutEffect(() => {
    // 显式 maxHeight 优先：跳过测量，直接由调用方应用
    if (maxHeightProp !== undefined) return;

    const measure = () => {
      const area = scrollAreaRef.current;
      if (!area) return;
      const top = area.getBoundingClientRect().top;
      const bottomBarHeight = bottomBarRef.current?.getBoundingClientRect().height ?? 0;
      setMeasured(`${computeScrollAreaMaxHeight(window.innerHeight, top, bottomBarHeight)}px`);
    };

    measure();
    window.addEventListener("resize", measure);
    // capture：页面滚动改变滚动区顶部偏移时重测
    window.addEventListener("scroll", measure, { capture: true, passive: true });

    if (typeof ResizeObserver !== "undefined") {
      const observer = new ResizeObserver(measure);
      if (rootRef.current) observer.observe(rootRef.current);
      if (bottomBarRef.current) observer.observe(bottomBarRef.current);
      return () => {
        window.removeEventListener("resize", measure);
        window.removeEventListener("scroll", measure, { capture: true } as EventListenerOptions);
        observer.disconnect();
      };
    }

    return () => {
      window.removeEventListener("resize", measure);
      window.removeEventListener("scroll", measure, { capture: true } as EventListenerOptions);
    };
  }, [maxHeightProp, scrollAreaRef, bottomBarRef, rootRef]);

  return measured;
}

import * as React from "react";
import { cn } from "../lib/utils";
import {
  computeScrollBarMetrics,
  scrollLeftForDrag,
  scrollLeftForJump,
  scrollLeftForKey,
  type ScrollBarKey,
} from "./scroll-bar-math";

/**
 * ScrollBar — 自绘横向滚动条（Layer 1 基础组件）
 *
 * 用于原生横向滚动条被隐藏（WebKit `::-webkit-scrollbar:horizontal` 或整体 scrollbar-width）的场景：
 * 点击轨道跳转、拖动滑块、键盘操作（方向键/翻页/Home/End），并暴露 aria 值与可访问性语义。
 * 计算全部委托 scroll-bar-math 纯函数（node 无 jsdom 可单测）。
 *
 * @example
 *   <ScrollBar container={scrollContainerRef.current} contentRef={tableRef} />
 */
export interface ScrollBarProps extends React.HTMLAttributes<HTMLDivElement> {
  /** 横向滚动容器（overflow-x-auto），必选 */
  container: HTMLElement | null;
  /** 额外尺寸观察对象（如 <table>）：列宽拖拽/列显隐改变 scrollWidth 但容器盒子不变时的刷新盲区 */
  contentRef?: React.RefObject<HTMLElement | null>;
  /** 可滚动状态变化回调（供父组件联动显隐；需保持稳定引用） */
  onScrollableChange?: (scrollable: boolean) => void;
}

export const ScrollBar = React.forwardRef<HTMLDivElement, ScrollBarProps>(
  ({ container, contentRef, onScrollableChange, className, onKeyDown, ...rest }, ref) => {
    const [view, setView] = React.useState(() => computeScrollBarMetrics(0, 0, 0));
    const dragRef = React.useRef<{ startX: number; startLeft: number } | null>(null);

    React.useEffect(() => {
      if (!container) return;
      const update = () => {
        const next = computeScrollBarMetrics(container.clientWidth, container.scrollWidth, container.scrollLeft);
        setView(next);
        onScrollableChange?.(next.scrollable);
      };
      update();
      container.addEventListener("scroll", update, { passive: true });
      // 双观察：容器（clientWidth 变化）+ 内容（scrollWidth 变化，列宽/列显隐盲区）
      const observer = new ResizeObserver(update);
      observer.observe(container);
      if (contentRef?.current) observer.observe(contentRef.current);
      return () => {
        container.removeEventListener("scroll", update);
        observer.disconnect();
      };
    }, [container, contentRef, onScrollableChange]);

    // container 未就绪时也常驻渲染（hidden 隐藏），避免「条件挂载 → 永远测不到 scrollable」的鸡生蛋问题
    return (
      <div
        ref={ref}
        role="scrollbar"
        aria-orientation="horizontal"
        aria-valuemin={0}
        aria-valuemax={view.maxLeft}
        aria-valuenow={Math.round(Math.min(view.left, view.maxLeft))}
        tabIndex={0}
        className={cn(
          "relative h-2.5 w-full cursor-pointer touch-none select-none",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          !view.scrollable && "hidden",
          className,
        )}
        onKeyDown={(event) => {
          const key = event.key as ScrollBarKey;
          if (!["ArrowLeft", "ArrowRight", "PageUp", "PageDown", "Home", "End"].includes(key) || !container || !view.scrollable) {
            onKeyDown?.(event);
            return;
          }
          event.preventDefault();
          container.scrollLeft = scrollLeftForKey(key, container.scrollLeft, container.clientWidth, view.maxLeft);
        }}
        onPointerDown={(event) => {
          if (!container) return;
          const onThumb = Boolean((event.target as HTMLElement).closest("[data-thumb]"));
          if (onThumb) {
            dragRef.current = { startX: event.clientX, startLeft: container.scrollLeft };
            event.currentTarget.setPointerCapture(event.pointerId);
          } else {
            const rect = event.currentTarget.getBoundingClientRect();
            container.scrollLeft = scrollLeftForJump(event.clientX, rect.left, rect.width, view.maxLeft);
          }
        }}
        onPointerMove={(event) => {
          if (!dragRef.current || !container) return;
          const dx = event.clientX - dragRef.current.startX;
          // 拖动灵敏度：thumb 位移 1px 对应内容 scrollWidth/clientWidth px（1/ratio），与原生滚动条一致
          container.scrollLeft = scrollLeftForDrag(dragRef.current.startLeft, dx, view.ratio);
        }}
        onPointerUp={() => {
          dragRef.current = null;
        }}
        onPointerCancel={() => {
          dragRef.current = null;
        }}
        {...rest}
      >
        <div
          data-thumb
          className="absolute top-0 bottom-0 rounded-full bg-muted-foreground/30 transition-colors hover:bg-muted-foreground/50"
          style={{ left: `${view.thumbLeftPct}%`, width: `${view.thumbWidthPct}%` }}
        />
      </div>
    );
  },
);
ScrollBar.displayName = "ScrollBar";

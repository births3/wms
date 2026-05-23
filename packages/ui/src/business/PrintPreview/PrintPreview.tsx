import * as React from "react";
import { cn } from "../../lib/utils";
import { Printer, Download, ZoomIn, ZoomOut } from "lucide-react";
import { Button } from "../../ui/button";

/**
 * PrintPreview — 打印预览容器（A4 / 标签 / 面单 三种模板）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-M2-007（收货单据打印）/ US-M4-005（随货同行单）/ US-H5-003（快递面单）/ US-PK-006（包装站面单）
 * Wave：Wave 3（M2/M4 打印业务）
 * 业务约束：A4 = 210x297mm；标签 100x70mm；面单 100x180mm；GSP 法定单据格式
 *
 * @example
 *   <PrintPreview template="a4" pageCount={2}><h1>随货同行单</h1>...</PrintPreview>
 */

export type PrintTemplate = "a4" | "label" | "shipping";

export interface PrintPreviewProps extends React.HTMLAttributes<HTMLDivElement> {
  template: PrintTemplate;
  pageCount?: number;
  currentPage?: number;
  zoom?: number;
  onZoomChange?: (zoom: number) => void;
  onPageChange?: (page: number) => void;
  /** 模拟打印预览的内容 */
  children?: React.ReactNode;
}

const TEMPLATE_META: Record<PrintTemplate, { label: string; w: number; h: number; ratio: string }> = {
  a4: { label: "A4 (210×297mm)", w: 210, h: 297, ratio: "210/297" },
  label: { label: "标签 (100×70mm)", w: 100, h: 70, ratio: "100/70" },
  shipping: { label: "快递面单 (100×180mm)", w: 100, h: 180, ratio: "100/180" },
};

export const PrintPreview = React.forwardRef<HTMLDivElement, PrintPreviewProps>(
  (
    {
      template,
      pageCount = 1,
      currentPage = 1,
      zoom = 1,
      onZoomChange,
      onPageChange,
      children,
      className,
      ...rest
    },
    ref
  ) => {
    const meta = TEMPLATE_META[template];
    // 渲染宽度（mm → px，1mm ≈ 3.78px）
    const baseW = meta.w * 3.78 * zoom;

    return (
      <div ref={ref} className={cn("bg-muted rounded-md border overflow-hidden font-sans", className)} {...rest}>
        {/* 工具栏 */}
        <div className="flex items-center justify-between px-3 py-2 bg-background border-b">
          <div className="flex items-center gap-3">
            <span className="text-xs font-medium">{meta.label}</span>
            {pageCount > 1 && (
              <span className="text-xs text-muted-foreground">
                第 {currentPage} / {pageCount} 页
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="sm" className="size-7 p-0" onClick={() => onZoomChange?.(Math.max(0.5, zoom - 0.1))}>
              <ZoomOut className="size-3.5" />
            </Button>
            <span className="text-xs text-muted-foreground w-12 text-center">{Math.round(zoom * 100)}%</span>
            <Button variant="ghost" size="sm" className="size-7 p-0" onClick={() => onZoomChange?.(Math.min(2, zoom + 0.1))}>
              <ZoomIn className="size-3.5" />
            </Button>
            <div className="w-px h-4 bg-border mx-1" />
            <Button variant="outline" size="sm" className="h-7">
              <Download className="size-3.5" />
              导出 PDF
            </Button>
            <Button size="sm" className="h-7">
              <Printer className="size-3.5" />
              打印
            </Button>
          </div>
        </div>
        {/* 预览区 */}
        <div className="p-6 flex justify-center overflow-auto max-h-[900px]">
          <div
            className="bg-background shadow-md border border-border/40 origin-top"
            style={{
              width: baseW,
              aspectRatio: meta.ratio,
              padding: template === "a4" ? "16mm" : "4mm",
              transform: zoom > 1.2 ? `scale(${zoom / zoom})` : undefined, // 保留扩展空间
            }}
          >
            {children}
          </div>
        </div>
        {/* 底部翻页 */}
        {pageCount > 1 && (
          <div className="px-3 py-2 bg-background border-t flex items-center justify-center gap-2">
            <Button variant="outline" size="sm" disabled={currentPage <= 1} onClick={() => onPageChange?.(currentPage - 1)}>
              上一页
            </Button>
            <Button variant="outline" size="sm" disabled={currentPage >= pageCount} onClick={() => onPageChange?.(currentPage + 1)}>
              下一页
            </Button>
          </div>
        )}
      </div>
    );
  }
);
PrintPreview.displayName = "PrintPreview";

import * as React from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@wms/ui";
import { Printer } from "lucide-react";

import type { PrintTemplatePreviewResponse } from "@/features/print-template/print-template-queries";

import "hiprint/dist/print-lock.css";

interface H9TemplatePreviewDialogProps {
  open: boolean;
  preview: PrintTemplatePreviewResponse | null;
  onOpenChange: (open: boolean) => void;
  onPrint: () => Promise<void>;
}

export function H9TemplatePreviewDialog({
  open,
  preview,
  onOpenChange,
  onPrint,
}: H9TemplatePreviewDialogProps) {
  const id = React.useId().replace(/:/g, "");
  const containerId = `h9-hiprint-preview-${id}`;
  const templateRef = React.useRef<HiprintTemplate | null>(null);
  const [paperDirection, setPaperDirection] = React.useState<PaperDirection>("portrait");
  const [error, setError] = React.useState<string | null>(null);
  const [printing, setPrinting] = React.useState(false);
  const [ready, setReady] = React.useState(false);

  React.useEffect(() => {
    if (open && preview) setPaperDirection(readPaperDirection(preview.hiprint_json));
  }, [open, preview]);

  React.useEffect(() => {
    if (!open || !preview) return;
    const currentPreview = preview;
    let disposed = false;
    async function renderPreview() {
      setError(null);
      setReady(false);
      try {
        const jqueryModule = await import("jquery");
        const win = window as unknown as { jQuery?: unknown; $?: unknown };
        win.jQuery = jqueryModule.default;
        win.$ = jqueryModule.default;
        const { disAutoConnect, hiprint, defaultElementTypeProvider } = await import("hiprint");
        if (disposed) return;
        disAutoConnect();
        hiprint.init({ providers: [new defaultElementTypeProvider()] });
        const template = new hiprint.PrintTemplate({
          template: applyPreviewPaperDirection(currentPreview.hiprint_json, paperDirection),
        });
        templateRef.current = template;
        const container = document.getElementById(containerId);
        if (!container) return;
        container.innerHTML = "";
        const html = template.getHtml(currentPreview.data);
        const node = html.get(0);
        if (node) container.appendChild(node);
        setReady(Boolean(node));
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "hiprint 预览失败");
      }
    }
    void renderPreview();
    return () => {
      disposed = true;
      templateRef.current = null;
    };
  }, [containerId, open, paperDirection, preview]);

  async function print() {
    if (!preview) return;
    setPrinting(true);
    try {
      templateRef.current?.print(preview.data);
      await onPrint();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "打印记录写入失败");
    } finally {
      setPrinting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[92vh] max-w-[72rem] overflow-auto">
        <DialogHeader>
          <DialogTitle>{preview?.template_name ?? "打印预览"}</DialogTitle>
          <DialogDescription>
            {preview ? `${preview.template_code} · v${preview.version_no}` : "选择模板后预览"}
          </DialogDescription>
        </DialogHeader>
        <label className="flex w-40 flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">纸张方向</span>
          <select
            className="h-9 rounded-md border bg-background px-3 text-sm"
            value={paperDirection}
            onChange={(event) => setPaperDirection(event.target.value as PaperDirection)}
          >
            <option value="portrait">竖向</option>
            <option value="landscape">横向</option>
          </select>
        </label>
        {error && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</div>}
        <div id={containerId} className="min-h-[32rem] overflow-auto rounded-md border bg-muted/30 p-4" />
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
          <Button type="button" disabled={!preview || !ready || printing} onClick={() => void print()}>
            <Printer className="size-4" aria-hidden />
            打印
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

type HiprintTemplate = {
  getHtml(data?: unknown): { get(index: number): HTMLElement | undefined; length: number };
  print(data?: unknown): void;
};

type PaperDirection = "portrait" | "landscape";

function applyPreviewPaperDirection(value: unknown, direction: PaperDirection) {
  const record = cloneRecord(value);
  const panels = Array.isArray(record.panels) ? record.panels : [];
  const firstPanel = cloneRecord(panels[0]);
  const width = positiveNumber(firstPanel.width, 210);
  const height = positiveNumber(firstPanel.height, 297);
  const short = Math.min(width, height);
  const long = Math.max(width, height);
  record.panels = [
    {
      ...firstPanel,
      width: direction === "landscape" ? long : short,
      height: direction === "landscape" ? short : long,
      orient: direction,
    },
    ...panels.slice(1),
  ];
  return record;
}

function readPaperDirection(value: unknown): PaperDirection {
  const panels = cloneRecord(value).panels;
  const firstPanel = Array.isArray(panels) ? cloneRecord(panels[0]) : {};
  if (firstPanel.orient === "landscape" || firstPanel.direction === "landscape") return "landscape";
  const width = positiveNumber(firstPanel.width, 0);
  const height = positiveNumber(firstPanel.height, 0);
  return width > height ? "landscape" : "portrait";
}

function positiveNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : fallback;
}

function cloneRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
}

import * as React from "react";
import { Button, cn } from "@wms/ui";
import { RefreshCw } from "lucide-react";

import "hiprint/dist/print-lock.css";

interface H9HiprintDesignerProps {
  templateJson: unknown;
  fieldBindingPanel: React.ReactNode;
  fields: Array<{ fieldPath: string; displayName: string }>;
  onJsonChange: (value: unknown) => void;
}

export interface H9HiprintDesignerHandle {
  getJson: () => unknown;
  print: (data: unknown) => void;
}

export const H9HiprintDesigner = React.forwardRef<H9HiprintDesignerHandle, H9HiprintDesignerProps>(
  function H9HiprintDesigner({ templateJson, fieldBindingPanel, fields, onJsonChange }, ref) {
    const id = React.useId().replace(/:/g, "");
    const paletteId = `h9-hiprint-palette-${id}`;
    const canvasId = `h9-hiprint-canvas-${id}`;
    const settingId = `h9-hiprint-setting-${id}`;
    const paginationId = `h9-hiprint-pagination-${id}`;
    const templateRef = React.useRef<HiprintTemplate | null>(null);
    const [status, setStatus] = React.useState("hiprint 设计器初始化中");
    const [error, setError] = React.useState<string | null>(null);
    const [fieldPanelOpen, setFieldPanelOpen] = React.useState(true);
    const [fieldPanelTab, setFieldPanelTab] = React.useState<"binding" | "components">("binding");

    React.useImperativeHandle(ref, () => ({
      getJson: () => {
        const json = templateRef.current?.getJson() ?? templateJson;
        onJsonChange(json);
        return json;
      },
      print: (data: unknown) => {
        templateRef.current?.print(data);
      },
    }));

    React.useEffect(() => {
      let disposed = false;
      async function setupDesigner() {
        setError(null);
        try {
          const jqueryModule = await import("jquery");
          const win = window as unknown as { jQuery?: unknown; $?: unknown };
          win.jQuery = jqueryModule.default;
          win.$ = jqueryModule.default;
          const { disAutoConnect, hiprint, defaultElementTypeProvider } = await import("hiprint");
          if (disposed) return;
          disAutoConnect();
          hiprint.init({ providers: [new defaultElementTypeProvider()] });
          const palette = document.getElementById(paletteId);
          if (palette && hiprint.PrintElementTypeManager) {
            hiprint.PrintElementTypeManager.buildByHtml(jqueryModule.default(palette).find(".ep-draggable-item"));
          }
          const canvas = document.getElementById(canvasId);
          if (!canvas) return;
          canvas.innerHTML = "";
          const template = new hiprint.PrintTemplate({
            template: normalizeHiprintTemplate(templateJson),
            settingContainer: `#${settingId}`,
            paginationContainer: `#${paginationId}`,
          });
          template.design(`#${canvasId}`);
          templateRef.current = template;
          setStatus("hiprint 设计器已就绪");
        } catch (cause) {
          const message = cause instanceof Error ? cause.message : "hiprint 初始化失败";
          setError(message);
          setStatus("hiprint 设计器不可用");
        }
      }
      void setupDesigner();
      return () => {
        disposed = true;
        templateRef.current = null;
      };
    }, [canvasId, paletteId, paginationId, settingId, templateJson]);

    function syncJson() {
      const json = templateRef.current?.getJson() ?? templateJson;
      onJsonChange(json);
      setStatus("hiprint JSON 已同步");
    }

    return (
      <div
        className={cn(
          "grid min-h-[34rem] gap-3",
          fieldPanelOpen ? "lg:grid-cols-[18rem_minmax(0,1fr)_18rem]" : "lg:grid-cols-[4rem_minmax(0,1fr)_18rem]",
        )}
      >
        <aside className={cn("rounded-md border bg-muted/20 p-3", !fieldPanelOpen && "flex items-start justify-center p-2")}>
          {!fieldPanelOpen && (
            <Button type="button" variant="outline" size="sm" title="显示字段面板" onClick={() => setFieldPanelOpen(true)}>
              字段
            </Button>
          )}
          <div className={cn("space-y-3", !fieldPanelOpen && "hidden")}>
            <div className="flex items-center justify-between">
              <div className="text-sm font-medium text-foreground">字段面板</div>
              <Button type="button" variant="ghost" size="sm" onClick={() => setFieldPanelOpen(false)}>
                隐藏
              </Button>
            </div>
            <div className="grid grid-cols-2 rounded-md border bg-background p-1">
              <Button
                type="button"
                variant={fieldPanelTab === "binding" ? "secondary" : "ghost"}
                size="sm"
                className="h-7"
                aria-pressed={fieldPanelTab === "binding"}
                onClick={() => setFieldPanelTab("binding")}
              >
                绑定
              </Button>
              <Button
                type="button"
                variant={fieldPanelTab === "components" ? "secondary" : "ghost"}
                size="sm"
                className="h-7"
                aria-pressed={fieldPanelTab === "components"}
                onClick={() => setFieldPanelTab("components")}
              >
                组件
              </Button>
            </div>
            <div className={fieldPanelTab === "binding" ? undefined : "hidden"}>{fieldBindingPanel}</div>
            <div id={paletteId} className={fieldPanelTab === "components" ? "flex flex-col gap-2" : "hidden"}>
              <HiprintDragItem tid="defaultModule.text" label="文本" />
              <HiprintDragItem tid="defaultModule.longText" label="长文" />
              <HiprintDragItem tid="defaultModule.barcode" label="条码" />
              <HiprintDragItem tid="defaultModule.qrcode" label="二维码" />
              {fields.slice(0, 10).map((field) => (
                <HiprintDragItem
                  key={field.fieldPath}
                  tid="defaultModule.text"
                  label={field.displayName}
                  data={field.fieldPath}
                />
              ))}
            </div>
          </div>
        </aside>

        <main className="min-w-0 rounded-md border bg-background">
          <div className="flex items-center justify-between border-b px-3 py-2">
            <span className={cn("text-sm", error ? "text-destructive" : "text-muted-foreground")}>{status}</span>
            <Button type="button" variant="outline" size="sm" onClick={syncJson}>
              <RefreshCw className="size-4" aria-hidden />
              同步
            </Button>
          </div>
          {error && <div className="border-b bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</div>}
          <div id={paginationId} className="border-b px-3 py-2 text-sm text-muted-foreground" />
          <div id={canvasId} className="h-[30rem] overflow-auto bg-muted/30 p-4" />
        </main>

        <aside className="rounded-md border bg-muted/20 p-3">
          <div className="text-sm font-medium text-foreground">参数</div>
          <div id={settingId} className="mt-3 max-h-[30rem] overflow-auto text-sm" />
        </aside>
      </div>
    );
  },
);

function HiprintDragItem({ tid, label, data }: { tid: string; label: string; data?: string }) {
  return (
    <div
      className="ep-draggable-item cursor-grab rounded-md border bg-background px-3 py-2 text-sm hover:border-primary hover:text-primary"
      {...{ tid }}
      data-title={label}
      data-field={data}
    >
      {label}
    </div>
  );
}

function normalizeHiprintTemplate(value: unknown) {
  if (value && typeof value === "object" && !Array.isArray(value)) return value;
  return { panels: [{ index: 0, paperType: "A4", printElements: [] }] };
}

type HiprintTemplate = {
  design(target: string | HTMLElement): void;
  getJson(): unknown;
  print(data?: unknown): void;
};

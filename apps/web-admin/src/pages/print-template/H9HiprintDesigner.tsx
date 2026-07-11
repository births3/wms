import * as React from "react";
import { Button, cn } from "@wms/ui";
import { Maximize2, Minimize2, RefreshCw, Save, X } from "lucide-react";

import "hiprint/dist/print-lock.css";

type DesignerReadyState = "initializing" | "ready" | "error";

interface H9HiprintDesignerProps {
  templateJson: unknown;
  /** 切换模板/打开设计器时变化，用于重新初始化，避免每次 JSON 变更触发重载 */
  designSessionKey?: string;
  templateSettingsPanel: React.ReactNode;
  fieldBindingPanel: React.ReactNode;
  fields: Array<{ fieldPath: string; displayName: string }>;
  onJsonChange: (value: unknown) => void;
  onCancel?: () => void;
  onSave?: () => void;
  saveLabel?: string;
  saveDisabled?: boolean;
  onReadyStateChange?: (state: DesignerReadyState) => void;
}

export interface H9HiprintDesignerHandle {
  getJson: () => unknown;
  print: (data: unknown) => void;
  getReadyState: () => DesignerReadyState;
}

export const H9HiprintDesigner = React.forwardRef<H9HiprintDesignerHandle, H9HiprintDesignerProps>(
  function H9HiprintDesigner(
    {
      templateJson,
      designSessionKey = "default",
      templateSettingsPanel,
      fieldBindingPanel,
      fields,
      onJsonChange,
      onCancel,
      onSave,
      saveLabel = "保存",
      saveDisabled = false,
      onReadyStateChange,
    },
    ref,
  ) {
    const id = React.useId().replace(/:/g, "");
    const paletteId = `h9-hiprint-palette-${id}`;
    const canvasId = `h9-hiprint-canvas-${id}`;
    const settingId = `h9-hiprint-setting-${id}`;
    const paginationId = `h9-hiprint-pagination-${id}`;
    const designerRootRef = React.useRef<HTMLDivElement | null>(null);
    const templateRef = React.useRef<HiprintTemplate | null>(null);
    const initialTemplateJsonRef = React.useRef(templateJson);
    const [readyState, setReadyState] = React.useState<DesignerReadyState>("initializing");
    const [status, setStatus] = React.useState("设计器初始化中…");
    const [error, setError] = React.useState<string | null>(null);
    const [fieldPanelOpen, setFieldPanelOpen] = React.useState(true);
    const [fieldPanelTab, setFieldPanelTab] = React.useState<"binding" | "components">("binding");
    const [designerFullscreen, setDesignerFullscreen] = React.useState(false);

    const updateReadyState = React.useCallback(
      (next: DesignerReadyState, nextStatus: string, nextError: string | null = null) => {
        setReadyState(next);
        setStatus(nextStatus);
        setError(nextError);
        onReadyStateChange?.(next);
      },
      [onReadyStateChange],
    );

    React.useImperativeHandle(ref, () => ({
      getJson: () => {
        const json = templateRef.current?.getJson() ?? templateJson;
        onJsonChange(json);
        return json;
      },
      print: (data: unknown) => {
        templateRef.current?.print(data);
      },
      getReadyState: () => readyState,
    }));

    React.useEffect(() => {
      initialTemplateJsonRef.current = templateJson;
    }, [designSessionKey, templateJson]);

    React.useEffect(() => {
      let disposed = false;
      async function setupDesigner() {
        updateReadyState("initializing", "设计器初始化中…");
        templateRef.current = null;
        try {
          const jqueryModule = await import("jquery");
          if (disposed) return;
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
          if (!canvas) {
            updateReadyState("error", "设计器加载失败", "未找到设计器画布容器，请关闭后重试");
            return;
          }
          canvas.innerHTML = "";
          const template = new hiprint.PrintTemplate({
            template: normalizeHiprintTemplate(initialTemplateJsonRef.current),
            settingContainer: `#${settingId}`,
            paginationContainer: `#${paginationId}`,
          });
          template.design(`#${canvasId}`);
          if (disposed) return;
          templateRef.current = template;
          updateReadyState("ready", "设计器已就绪");
        } catch (cause) {
          if (disposed) return;
          const message = cause instanceof Error ? cause.message : "设计器初始化失败";
          updateReadyState("error", "设计器加载失败", message);
        }
      }
      void setupDesigner();
      return () => {
        disposed = true;
        templateRef.current = null;
      };
    }, [canvasId, designSessionKey, paletteId, paginationId, settingId, updateReadyState]);

    React.useEffect(() => {
      function syncFullscreenState() {
        setDesignerFullscreen(document.fullscreenElement === designerRootRef.current);
      }

      document.addEventListener("fullscreenchange", syncFullscreenState);
      return () => document.removeEventListener("fullscreenchange", syncFullscreenState);
    }, []);

    function syncJson() {
      if (readyState !== "ready" || !templateRef.current) {
        setStatus(readyState === "error" ? "设计器不可用，无法同步" : "设计器尚未就绪");
        return;
      }
      const json = templateRef.current.getJson();
      onJsonChange(json);
      setStatus("模板 JSON 已同步");
    }

    async function toggleDesignerFullscreen() {
      if (designerFullscreen) {
        if (document.fullscreenElement) {
          await document.exitFullscreen().catch(() => undefined);
        }
        setDesignerFullscreen(false);
        return;
      }

      setDesignerFullscreen(true);
      await designerRootRef.current?.requestFullscreen?.().catch(() => undefined);
    }

    const showActionBar = Boolean(onCancel || onSave);
    const canSave = readyState === "ready" && !saveDisabled;

    return (
      <div
        ref={designerRootRef}
        data-h9-hiprint-designer="true"
        data-h9-ready-state={readyState}
        className={cn(
          "grid min-h-[34rem] gap-3",
          fieldPanelOpen ? "lg:grid-cols-[18rem_minmax(0,1fr)_18rem]" : "lg:grid-cols-[4rem_minmax(0,1fr)_18rem]",
          designerFullscreen && "fixed inset-0 z-[70] h-screen bg-background p-3 shadow-2xl",
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
          <div className="flex flex-wrap items-center justify-between gap-2 border-b px-3 py-2">
            <span
              className={cn(
                "text-sm",
                readyState === "error" ? "text-destructive" : readyState === "ready" ? "text-emerald-700" : "text-muted-foreground",
              )}
              data-h9-designer-status={readyState}
            >
              {status}
            </span>
            <div className="flex flex-wrap items-center gap-2">
              <Button type="button" variant="outline" size="sm" onClick={syncJson} disabled={readyState !== "ready"}>
                <RefreshCw className="size-4" aria-hidden />
                同步
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                title={designerFullscreen ? "退出全屏" : "全屏设计"}
                aria-pressed={designerFullscreen}
                onClick={() => void toggleDesignerFullscreen()}
              >
                {designerFullscreen ? <Minimize2 className="size-4" aria-hidden /> : <Maximize2 className="size-4" aria-hidden />}
                {designerFullscreen ? "退出" : "全屏"}
              </Button>
              {showActionBar && (
                <>
                  {onCancel && (
                    <Button type="button" variant="outline" size="sm" onClick={onCancel}>
                      <X className="size-4" aria-hidden />
                      取消
                    </Button>
                  )}
                  {onSave && (
                    <Button type="button" size="sm" onClick={onSave} disabled={!canSave}>
                      <Save className="size-4" aria-hidden />
                      {saveLabel}
                    </Button>
                  )}
                </>
              )}
            </div>
          </div>
          <div className="border-b bg-muted/10">{templateSettingsPanel}</div>
          {error && <div className="border-b bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</div>}
          {readyState === "initializing" && (
            <div className="border-b bg-muted/20 px-3 py-2 text-sm text-muted-foreground">
              正在加载 hiprint 组件与画布，请稍候…
            </div>
          )}
          <div id={paginationId} className="border-b px-3 py-2 text-sm text-muted-foreground" />
          <div id={canvasId} className={cn("h-[30rem] overflow-auto bg-muted/30 p-4", designerFullscreen && "h-[calc(100vh-9rem)]")} />
        </main>

        <aside className="rounded-md border bg-muted/20 p-3">
          <div className="text-sm font-medium text-foreground">参数</div>
          <div id={settingId} className={cn("mt-3 max-h-[30rem] overflow-auto text-sm", designerFullscreen && "max-h-[calc(100vh-8rem)]")} />
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

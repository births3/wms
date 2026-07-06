import * as React from "react";
import { createPortal } from "react-dom";
import { Bookmark } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import type { DataGridColumn } from "./data-grid-types";
import {
  dataGridFloatingPanelPosition,
  type DataGridFloatingPanelPosition,
  type DataGridLogicState,
} from "./data-grid-logic";
import { useDataGridPopoverDismiss } from "./data-grid-popover-dismiss";
import {
  dataGridNamedViewsStorageKey,
  loadDataGridNamedViewsFromStorage,
  nextDataGridNamedViewDraftName,
  pickDefaultDataGridNamedView,
  removeDataGridNamedView,
  saveDataGridNamedViewsToStorage,
  upsertDataGridNamedView,
  type DataGridNamedView,
} from "./data-grid-views";

/**
 * DataGridNamedViewsToolbar — DataGrid 命名视图操作条
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表字段视图保存
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：只保存当前客户端视图状态；无 storageKey 或 localStorage 时不渲染。
 *
 * @example
 *   <DataGridNamedViewsToolbar settings={settings} onApplyView={setSettings} />
 */
export interface DataGridNamedViewsToolbarProps<T>
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  storageKey?: string;
  columns: DataGridColumn<T>[];
  actionKeys: string[];
  pageSizeOptions: number[];
  defaultPageSize: number;
  settings: DataGridLogicState;
  queryState?: unknown;
  onApplyView: (state: DataGridLogicState, queryState?: unknown) => void;
}

export function DataGridNamedViewsToolbar<T>({
  storageKey,
  columns,
  actionKeys,
  pageSizeOptions,
  defaultPageSize,
  settings,
  queryState,
  onApplyView,
  className,
  ...rest
}: DataGridNamedViewsToolbarProps<T>) {
  const storage = getDataGridNamedViewStorage(storageKey);
  const options = React.useMemo(
    () => ({
      columns,
      actionKeys,
      pageSizeOptions,
      defaultPageSize,
      now: new Date().toISOString(),
    }),
    [actionKeys, columns, defaultPageSize, pageSizeOptions],
  );
  const [views, setViews] = React.useState<DataGridNamedView[]>(() =>
    loadDataGridNamedViewsFromStorage(storage, storageKey, options),
  );
  const buttonRef = React.useRef<HTMLButtonElement | null>(null);
  const [viewName, setViewName] = React.useState(
    () => pickDefaultDataGridNamedView(views)?.name ?? "",
  );
  const [open, setOpen] = React.useState(false);
  const [panelPosition, setPanelPosition] = React.useState<DataGridFloatingPanelPosition | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const panelId = React.useId();

  React.useEffect(() => {
    if (!storage || !storageKey) {
      setViews([]);
      setViewName("");
      setError(null);
      return;
    }

    const loadedViews = loadDataGridNamedViewsFromStorage(storage, storageKey, options);
    setViews(loadedViews);
    setViewName((current) => {
      if (loadedViews.some((view) => view.name === current)) return current;
      return pickDefaultDataGridNamedView(loadedViews)?.name ?? "";
    });
    setError(null);
  }, [options, storage, storageKey]);

  useDataGridPopoverDismiss({
    open,
    onDismiss: () => setOpen(false),
  });

  React.useEffect(() => {
    if (!open) return;

    function updatePosition() {
      const rect = buttonRef.current?.getBoundingClientRect();
      if (!rect) return;
      setPanelPosition(
        dataGridFloatingPanelPosition(rect, { width: window.innerWidth, height: window.innerHeight }, 320),
      );
    }

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [open]);

  if (!storage || !storageKey) return null;

  const selectedView = views.find((view) => view.name === viewName) ?? null;

  function saveView() {
    if (!storage || !storageKey) return;

    const result = upsertDataGridNamedView(
      views,
      {
        name: viewName,
        state: settings,
        queryState,
      },
      { ...options, now: new Date().toISOString() },
    );

    if (!result.ok) {
      setError(result.error);
      return;
    }

    setViews(result.views);
    setViewName(nextDataGridNamedViewDraftName(views, result.view.name));
    const saved = saveDataGridNamedViewsToStorage(storage, storageKey, result.views);
    setError(saved.ok ? null : saved.error);
  }

  function applyView() {
    if (!selectedView) return;
    onApplyView(selectedView.state, selectedView.queryState);
    setError(null);
  }

  function deleteView() {
    if (!storage || !storageKey) return;

    const removed = removeDataGridNamedView(views, viewName);
    if (!removed.ok) {
      setError(removed.error);
      return;
    }

    setViews(removed.views);
    setViewName(pickDefaultDataGridNamedView(removed.views)?.name ?? "");
    const saved = saveDataGridNamedViewsToStorage(storage, storageKey, removed.views);
    setError(saved.ok ? null : saved.error);
  }

  return (
    <div
      className={cn("inline-flex", className)}
      data-datagrid-named-views
      data-storage-key={dataGridNamedViewsStorageKey(storageKey)}
      {...rest}
    >
      <Button
        ref={buttonRef}
        type="button"
        variant="outline"
        size="sm"
        className="h-8 shrink-0"
        aria-label="视图"
        title="视图保存、应用、删除"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpen((current) => !current)}
        data-datagrid-popover
      >
        <Bookmark className="size-4" aria-hidden />
        视图
      </Button>
      {open && panelPosition && typeof document !== "undefined"
        ? createPortal(
            <div
              id={panelId}
              className="fixed z-50 w-80 overflow-auto rounded-md border bg-background p-3 text-left text-sm shadow-lg"
              // 动态：命名视图浮层跟随触发按钮位置和视口高度。
              style={{ top: panelPosition.top, left: panelPosition.left, maxHeight: panelPosition.maxHeight }}
              data-datagrid-popover
            >
              <div className="grid gap-2">
                <input
                  type="text"
                  value={viewName}
                  onChange={(event) => setViewName(event.target.value)}
                  placeholder="命名视图"
                  aria-label="命名视图"
                  className="h-8 rounded-md border border-input bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                />
                <select
                  value={selectedView ? selectedView.name : ""}
                  aria-label="选择命名视图"
                  onChange={(event) => setViewName(event.target.value)}
                  className="h-8 rounded-md border border-input bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <option value="">选择视图</option>
                  {views.map((view) => (
                    <option key={view.name} value={view.name}>
                      {view.name}
                    </option>
                  ))}
                </select>
                <div className="flex flex-wrap justify-end gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={!viewName.trim()}
                    onClick={saveView}
                  >
                    保存视图
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={!selectedView}
                    onClick={applyView}
                  >
                    应用视图
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={!selectedView}
                    onClick={deleteView}
                  >
                    删除视图
                  </Button>
                </div>
                {error ? (
                  <span role="status" className="text-xs text-destructive">
                    {error}
                  </span>
                ) : null}
              </div>
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}

function getDataGridNamedViewStorage(storageKey: string | undefined): Storage | null {
  if (!storageKey || typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

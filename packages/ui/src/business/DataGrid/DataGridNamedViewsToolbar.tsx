import * as React from "react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import type { DataGridColumn } from "./DataGrid";
import type { DataGridLogicState } from "./data-grid-logic";
import {
  dataGridNamedViewsStorageKey,
  loadDataGridNamedViewsFromStorage,
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
  pageSizeOptions: number[];
  defaultPageSize: number;
  settings: DataGridLogicState;
  onApplyView: (state: DataGridLogicState) => void;
}

export function DataGridNamedViewsToolbar<T>({
  storageKey,
  columns,
  pageSizeOptions,
  defaultPageSize,
  settings,
  onApplyView,
  className,
  ...rest
}: DataGridNamedViewsToolbarProps<T>) {
  const listId = React.useId();
  const storage = getDataGridNamedViewStorage(storageKey);
  const options = React.useMemo(
    () => ({
      columns,
      pageSizeOptions,
      defaultPageSize,
      now: new Date().toISOString(),
    }),
    [columns, defaultPageSize, pageSizeOptions],
  );
  const [views, setViews] = React.useState<DataGridNamedView[]>(() =>
    loadDataGridNamedViewsFromStorage(storage, storageKey, options),
  );
  const [viewName, setViewName] = React.useState(
    () => pickDefaultDataGridNamedView(views)?.name ?? "",
  );
  const [error, setError] = React.useState<string | null>(null);

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

  if (!storage || !storageKey) return null;

  const selectedView = views.find((view) => view.name === viewName) ?? null;

  function saveView() {
    if (!storage || !storageKey) return;

    const result = upsertDataGridNamedView(
      views,
      {
        name: viewName,
        state: settings,
      },
      { ...options, now: new Date().toISOString() },
    );

    if (!result.ok) {
      setError(result.error);
      return;
    }

    setViews(result.views);
    setViewName(result.view.name);
    const saved = saveDataGridNamedViewsToStorage(storage, storageKey, result.views);
    setError(saved.ok ? null : saved.error);
  }

  function applyView() {
    if (!selectedView) return;
    onApplyView(selectedView.state);
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
      className={cn("flex flex-wrap items-center justify-end gap-1", className)}
      data-datagrid-named-views
      data-storage-key={dataGridNamedViewsStorageKey(storageKey)}
      {...rest}
    >
      <input
        type="text"
        list={listId}
        value={viewName}
        onChange={(event) => setViewName(event.target.value)}
        placeholder="命名视图"
        aria-label="命名视图"
        className="h-8 w-32 rounded-md border border-input bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
      />
      <datalist id={listId}>
        {views.map((view) => (
          <option key={view.name} value={view.name} />
        ))}
      </datalist>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="h-8"
        disabled={!viewName.trim()}
        onClick={saveView}
      >
        保存视图
      </Button>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="h-8"
        disabled={!selectedView}
        onClick={applyView}
      >
        应用视图
      </Button>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="h-8"
        disabled={!selectedView}
        onClick={deleteView}
      >
        删除视图
      </Button>
      {error ? (
        <span role="status" className="text-xs text-destructive">
          {error}
        </span>
      ) : null}
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

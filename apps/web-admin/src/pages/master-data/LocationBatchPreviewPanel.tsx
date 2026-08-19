import * as React from "react";
import {
  Button,
  LOCATION_BATCH_MAX_COUNT,
  locationBatchRangeCsv,
  type LocationBatchPreview,
  type LocationBatchRange,
} from "@wms/ui";
import { locationBatchGroupKey, uniqueSorted } from "./location-batch-helpers";
import { LayerCodes, MatrixTable, ThumbnailGrid } from "./LocationBatchPreviewGrid";

export interface LocationBatchPreviewPanelProps {
  preview: LocationBatchPreview;
  range: LocationBatchRange;
  errors: string[];
}

export function LocationBatchPreviewPanel({ preview, range, errors }: LocationBatchPreviewPanelProps) {
  const hasErrors = errors.length > 0;
  const [selectedGroupKey, setSelectedGroupKey] = React.useState<string | null>(null);
  const matrixViewportRef = React.useRef<HTMLDivElement>(null);
  const previewGroupMap = React.useMemo(
    () => new Map(preview.groups.map((group) => [locationBatchGroupKey(group), group])),
    [preview.groups],
  );
  const selectedGroup =
    (selectedGroupKey ? previewGroupMap.get(selectedGroupKey) : undefined) ?? preview.groups[0];
  const activeGroupKey = selectedGroup ? locationBatchGroupKey(selectedGroup) : null;
  const previewRows = uniqueSorted(preview.groups.map((group) => group.rowNo));
  const previewColumns = uniqueSorted(preview.groups.map((group) => group.columnNo));

  React.useEffect(() => {
    if (!activeGroupKey) return;
    const viewport = matrixViewportRef.current;
    const selectedCell = viewport?.querySelector<HTMLElement>(
      `[data-location-batch-group="${activeGroupKey}"]`,
    );
    if (!viewport || !selectedCell) return;
    viewport.scrollTo({
      left: selectedCell.offsetLeft - viewport.clientWidth / 2 + selectedCell.clientWidth / 2,
      top: selectedCell.offsetTop - viewport.clientHeight / 2 + selectedCell.clientHeight / 2,
    });
  }, [activeGroupKey]);

  return (
    <div className="rounded-md border bg-muted/30 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-sm font-medium">生成预览</div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>共 {preview.totalCount} 个；单次上限 {LOCATION_BATCH_MAX_COUNT} 个</span>
          {preview.codes.length > 0 ? (
            <Button
              type="button"
              variant="outline"
              className="h-7 px-2"
              onClick={() => {
                const blob = new Blob([locationBatchRangeCsv(range)], {
                  type: "text/csv;charset=utf-8",
                });
                const url = URL.createObjectURL(blob);
                const link = document.createElement("a");
                link.href = url;
                link.download = "location-batch-preview.csv";
                link.click();
                URL.revokeObjectURL(url);
              }}
            >
              导出 CSV
            </Button>
          ) : null}
        </div>
      </div>
      {hasErrors ? (
        <ul className="mt-3 grid gap-1 text-sm text-destructive">
          {errors.map((error) => (
            <li key={error}>{error}</li>
          ))}
        </ul>
      ) : (
        <div className="mt-3 grid h-[58vh] min-h-[24rem] gap-3 overflow-hidden lg:grid-cols-[14rem_minmax(28rem,1fr)_minmax(16rem,22rem)]">
          <ThumbnailGrid
            previewRows={previewRows}
            previewColumns={previewColumns}
            previewGroupMap={previewGroupMap}
            activeGroupKey={activeGroupKey}
            onSelect={setSelectedGroupKey}
          />
          <MatrixTable
            viewportRef={matrixViewportRef}
            previewRows={previewRows}
            previewColumns={previewColumns}
            previewGroupMap={previewGroupMap}
            activeGroupKey={activeGroupKey}
            onSelect={setSelectedGroupKey}
          />
          <LayerCodes preview={preview} selectedGroup={selectedGroup} />
        </div>
      )}
    </div>
  );
}


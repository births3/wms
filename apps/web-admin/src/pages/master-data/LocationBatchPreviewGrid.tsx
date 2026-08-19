import * as React from "react";
import type { LocationBatchPreview } from "@wms/ui";
import { locationBatchGroupKey, pad2 } from "./location-batch-helpers";

export function ThumbnailGrid({
  previewRows,
  previewColumns,
  previewGroupMap,
  activeGroupKey,
  onSelect,
}: {
  previewRows: number[];
  previewColumns: number[];
  previewGroupMap: Map<string, { codes: string[] }>;
  activeGroupKey: string | null;
  onSelect: (key: string) => void;
}) {
  return (
    <div className="flex min-h-0 flex-col rounded-md border bg-background p-2">
      <div className="px-1 pb-2 text-xs font-medium text-muted-foreground">缩略定位</div>
      <div className="min-h-0 overflow-auto">
        <div
          className="grid w-max min-w-full gap-0.5"
          style={{ gridTemplateColumns: `repeat(${previewColumns.length}, minmax(0.45rem, 1fr))` }}
        >
          {previewRows.map((rowNo) =>
            previewColumns.map((columnNo) => {
              const groupKey = locationBatchGroupKey({ rowNo, columnNo });
              const group = previewGroupMap.get(groupKey);
              const selected = groupKey === activeGroupKey;
              return (
                <button
                  key={groupKey}
                  type="button"
                  aria-label={`定位 排${rowNo} 列${columnNo}`}
                  disabled={!group}
                  onClick={() => group && onSelect(groupKey)}
                  className={
                    selected
                      ? "h-2.5 rounded-[2px] bg-primary"
                      : "h-2.5 rounded-[2px] bg-muted-foreground/30 hover:bg-primary/50 disabled:bg-muted/40"
                  }
                />
              );
            }),
          )}
        </div>
      </div>
    </div>
  );
}

export function MatrixTable({
  viewportRef,
  previewRows,
  previewColumns,
  previewGroupMap,
  activeGroupKey,
  onSelect,
}: {
  viewportRef: React.RefObject<HTMLDivElement | null>;
  previewRows: number[];
  previewColumns: number[];
  previewGroupMap: Map<string, { codes: string[] }>;
  activeGroupKey: string | null;
  onSelect: (key: string) => void;
}) {
  return (
    <div ref={viewportRef} className="min-h-0 overflow-auto rounded-md border bg-background p-2 pb-32">
      <div className="sticky top-0 z-20 bg-background px-1 pb-2 text-xs font-medium text-muted-foreground">
        排 × 列
      </div>
      <table className="min-w-max border-separate border-spacing-1 text-xs">
        <thead>
          <tr>
            <th className="sticky left-0 top-6 z-20 bg-background px-2 py-1 text-left font-medium text-muted-foreground">
              排\列
            </th>
            {previewColumns.map((columnNo) => (
              <th
                key={columnNo}
                className="sticky top-6 z-10 bg-background px-2 py-1 text-center font-medium text-muted-foreground"
              >
                列{pad2(columnNo)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {previewRows.map((rowNo) => (
            <tr key={rowNo}>
              <th className="sticky left-0 z-10 bg-background px-2 py-1 text-left font-medium text-muted-foreground">
                排{pad2(rowNo)}
              </th>
              {previewColumns.map((columnNo) => {
                const groupKey = locationBatchGroupKey({ rowNo, columnNo });
                const group = previewGroupMap.get(groupKey);
                const selected = groupKey === activeGroupKey;
                return (
                  <td key={columnNo} data-location-batch-group={groupKey}>
                    {group ? (
                      <button
                        type="button"
                        onClick={() => onSelect(groupKey)}
                        className={
                          selected
                            ? "w-16 rounded-md bg-primary px-2 py-2 text-center font-medium text-primary-foreground"
                            : "w-16 rounded-md bg-muted px-2 py-2 text-center hover:bg-muted/70"
                        }
                      >
                        {group.codes.length}层
                      </button>
                    ) : (
                      <span className="block rounded-md bg-muted/40 px-2 py-2 text-center text-muted-foreground">
                        -
                      </span>
                    )}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function LayerCodes({
  preview,
  selectedGroup,
}: {
  preview: LocationBatchPreview;
  selectedGroup: { label: string; codes: string[] } | undefined;
}) {
  return (
    <div className="flex min-h-0 flex-col overflow-hidden rounded-md border bg-background p-2">
      <div className="shrink-0 px-1 pb-2 text-xs font-medium text-muted-foreground">
        带层库位{selectedGroup ? ` · ${selectedGroup.label}` : ""}
      </div>
      <div className="min-h-0 overflow-auto">
        <div className="flex flex-wrap gap-2">
          {(selectedGroup?.codes ?? []).map((code) => {
            const sequence = preview.sequences.find((item) => item.code === code);
            return (
              <span key={code} className="rounded-md bg-muted px-2 py-1 font-mono text-xs">
                {code}
                {sequence ? ` · 拣${sequence.pickSequenceNo}/上${sequence.putawaySequenceNo}` : ""}
              </span>
            );
          })}
        </div>
      </div>
    </div>
  );
}

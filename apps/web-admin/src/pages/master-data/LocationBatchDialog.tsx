import * as React from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  LOCATION_BATCH_MAX_COUNT,
  type LocationBatchPreview,
  type LocationBatchRange,
} from "@wms/ui";

export const initialLocationBatchRange: LocationBatchRange = {
  areaCode: "",
  rowStart: 1,
  rowEnd: 1,
  columnStart: 1,
  columnEnd: 1,
  layerStart: 1,
  layerEnd: 1,
};

export const defaultLocationBatchType = "storage";

const locationTypeOptions: Array<[string, string]> = [
  ["storage", "存储位"],
  ["case_pick", "箱拣位"],
  ["piece_pick", "零拣位"],
];

interface LocationBatchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  scopeOptions: Array<{ value: string; label: string }>;
  scopeValue: string;
  onScopeValueChange: (value: string) => void;
  areaOptions: string[];
  range: LocationBatchRange;
  onRangeChange: (patch: Partial<LocationBatchRange>) => void;
  locationType: string;
  onLocationTypeChange: (value: string) => void;
  errors: string[];
  preview: LocationBatchPreview;
  message: string | null;
  confirmDisabled?: boolean;
  confirmLabel?: string;
  onConfirm: () => void | Promise<void>;
}

export function LocationBatchDialog({
  open,
  onOpenChange,
  scopeOptions,
  scopeValue,
  onScopeValueChange,
  areaOptions,
  range,
  onRangeChange,
  locationType,
  onLocationTypeChange,
  errors,
  preview,
  message,
  confirmDisabled = false,
  confirmLabel = "确认新增",
  onConfirm,
}: LocationBatchDialogProps) {
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
    const selectedCell = matrixViewportRef.current?.querySelector<HTMLElement>(
      `[data-location-batch-group="${activeGroupKey}"]`,
    );
    if (!viewport || !selectedCell) return;
    viewport.scrollTo({
      left: selectedCell.offsetLeft - viewport.clientWidth / 2 + selectedCell.clientWidth / 2,
      top: selectedCell.offsetTop - viewport.clientHeight / 2 + selectedCell.clientHeight / 2,
    });
  }, [activeGroupKey]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[92vh] w-[calc(100vw-2rem)] max-w-none overflow-y-auto p-4 sm:p-6 lg:w-[94vw] 2xl:w-[1500px]">
        <DialogHeader>
          <DialogTitle>批量新增库位</DialogTitle>
          <DialogDescription>按区域、排、列、层范围生成库位编码预览并批量创建。</DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-2">
          <div className="grid gap-3 md:grid-cols-[minmax(14rem,1fr)_minmax(12rem,18rem)_12rem]">
            <label className="grid gap-1.5 text-sm">
              <span className="font-medium">仓库 / 库区</span>
              <select
                value={scopeValue}
                onChange={(event) => onScopeValueChange(event.target.value)}
                className="h-9 rounded-md border border-input bg-background px-3 text-sm"
                aria-label="仓库库区"
              >
                {scopeOptions.length === 0 ? (
                  <option value="">暂无可用仓库/库区</option>
                ) : (
                  scopeOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))
                )}
              </select>
            </label>

            <label className="grid gap-1.5 text-sm">
              <span className="font-medium">区域编码</span>
              <Input
                value={range.areaCode}
                onChange={(event) => onRangeChange({ areaCode: event.target.value })}
                placeholder="例如 A01"
                list="m1-location-area-options"
                aria-label="区域编码"
              />
              <datalist id="m1-location-area-options">
                {areaOptions.map((area) => (
                  <option key={area} value={area} />
                ))}
              </datalist>
            </label>

            <label className="grid gap-1.5 text-sm">
              <span className="font-medium">库位类型</span>
              <select
                value={locationType}
                onChange={(event) => onLocationTypeChange(event.target.value)}
                className="h-9 rounded-md border border-input bg-background px-3 text-sm"
                aria-label="库位类型"
              >
                {locationTypeOptions.map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            <RangeInputs
              label="排"
              start={range.rowStart}
              end={range.rowEnd}
              onStartChange={(rowStart) => onRangeChange({ rowStart })}
              onEndChange={(rowEnd) => onRangeChange({ rowEnd })}
            />
            <RangeInputs
              label="列"
              start={range.columnStart}
              end={range.columnEnd}
              onStartChange={(columnStart) => onRangeChange({ columnStart })}
              onEndChange={(columnEnd) => onRangeChange({ columnEnd })}
            />
            <RangeInputs
              label="层"
              start={range.layerStart}
              end={range.layerEnd}
              onStartChange={(layerStart) => onRangeChange({ layerStart })}
              onEndChange={(layerEnd) => onRangeChange({ layerEnd })}
            />
          </div>

          <div className="rounded-md border bg-muted/30 p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="text-sm font-medium">生成预览</div>
              <div className="text-xs text-muted-foreground">
                共 {preview.totalCount} 个；单次上限 {LOCATION_BATCH_MAX_COUNT} 个
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
                <div className="flex min-h-0 flex-col rounded-md border bg-background p-2">
                  <div className="px-1 pb-2 text-xs font-medium text-muted-foreground">缩略定位</div>
                  <div className="min-h-0 overflow-auto">
                    <div
                      className="grid w-max min-w-full gap-0.5"
                      style={{
                        gridTemplateColumns: `repeat(${previewColumns.length}, minmax(0.45rem, 1fr))`,
                      }}
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
                              onClick={() => group && setSelectedGroupKey(groupKey)}
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

                <div ref={matrixViewportRef} className="min-h-0 overflow-auto rounded-md border bg-background p-2 pb-32">
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
                                    onClick={() => setSelectedGroupKey(groupKey)}
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

                <div className="flex min-h-0 flex-col overflow-hidden rounded-md border bg-background p-2">
                  <div className="shrink-0 px-1 pb-2 text-xs font-medium text-muted-foreground">
                    带层库位{selectedGroup ? ` · ${selectedGroup.label}` : ""}
                  </div>
                  <div className="min-h-0 overflow-auto">
                    <div className="flex flex-wrap gap-2">
                    {(selectedGroup?.codes ?? []).map((code) => (
                      <span key={code} className="rounded-md bg-muted px-2 py-1 font-mono text-xs">
                        {code}
                      </span>
                    ))}
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>

          {message && (
            <div className="rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-sm text-primary">
              {message}
            </div>
          )}
        </div>

        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline">
              取消
            </Button>
          </DialogClose>
          <Button type="button" onClick={onConfirm} disabled={hasErrors || confirmDisabled}>
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RangeInputs({
  label,
  start,
  end,
  onStartChange,
  onEndChange,
}: {
  label: string;
  start: number;
  end: number;
  onStartChange: (value: number) => void;
  onEndChange: (value: number) => void;
}) {
  return (
    <fieldset className="rounded-md border p-3">
      <legend className="px-1 text-sm font-medium">{label}范围</legend>
      <div className="grid grid-cols-2 gap-2">
        <label className="grid gap-1 text-xs text-muted-foreground">
          起始
          <Input
            type="number"
            min={1}
            value={start}
            onChange={(event) => onStartChange(numberInputValue(event.target.value))}
            aria-label={`${label}起始`}
          />
        </label>
        <label className="grid gap-1 text-xs text-muted-foreground">
          结束
          <Input
            type="number"
            min={1}
            value={end}
            onChange={(event) => onEndChange(numberInputValue(event.target.value))}
            aria-label={`${label}结束`}
          />
        </label>
      </div>
    </fieldset>
  );
}

function numberInputValue(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

function uniqueSorted(values: number[]) {
  return Array.from(new Set(values)).sort((left, right) => left - right);
}

function locationBatchGroupKey(group: { rowNo: number; columnNo: number }) {
  return `${group.rowNo}:${group.columnNo}`;
}

function pad2(value: number) {
  return String(value).padStart(2, "0");
}

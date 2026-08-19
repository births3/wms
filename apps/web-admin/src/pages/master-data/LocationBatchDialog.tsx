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
  type LocationBatchPreview,
  type LocationBatchRange,
} from "@wms/ui";
import type { SystemDictionaryOption } from "@/features/master-data/master-data-queries";
import { LocationBatchPreviewPanel } from "./LocationBatchPreviewPanel";
import { LocationBatchRangeFields } from "./LocationBatchRangeFields";

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
  locationTypeOptions: SystemDictionaryOption[];
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
  locationTypeOptions,
  onLocationTypeChange,
  errors,
  preview,
  message,
  confirmDisabled = false,
  confirmLabel = "确认新增",
  onConfirm,
}: LocationBatchDialogProps) {
  const [importError, setImportError] = React.useState<string | null>(null);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[92vh] w-[calc(100vw-2rem)] max-w-none overflow-y-auto p-4 sm:p-6 lg:w-[94vw] 2xl:w-[1500px]">
        <DialogHeader>
          <DialogTitle>批量新增库位</DialogTitle>
          <DialogDescription>按区域、排、列、层范围生成库位编码预览并批量创建。</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-2">
          <LocationBatchRangeFields
            scopeOptions={scopeOptions}
            scopeValue={scopeValue}
            onScopeValueChange={onScopeValueChange}
            areaOptions={areaOptions}
            range={range}
            onRangeChange={onRangeChange}
            locationType={locationType}
            locationTypeOptions={locationTypeOptions}
            onLocationTypeChange={onLocationTypeChange}
            onImportError={setImportError}
          />
          <LocationBatchPreviewPanel preview={preview} range={range} errors={errors} />
          {importError ? (
            <div className="rounded-md border border-destructive/20 bg-destructive/5 px-3 py-2 text-sm text-destructive">
              {importError}
            </div>
          ) : null}
          {message ? (
            <div className="rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-sm text-primary">
              {message}
            </div>
          ) : null}
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline">
              取消
            </Button>
          </DialogClose>
          <Button type="button" onClick={onConfirm} disabled={errors.length > 0 || confirmDisabled}>
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

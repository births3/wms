import {
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  parseLocationBatchCsv,
  type LocationBatchRange,
} from "@wms/ui";
import type { SystemDictionaryOption } from "@/features/master-data/master-data-queries";
import { numberInputValue } from "./location-batch-helpers";

export interface LocationBatchRangeFieldsProps {
  scopeOptions: Array<{ value: string; label: string }>;
  scopeValue: string;
  onScopeValueChange: (value: string) => void;
  areaOptions: string[];
  range: LocationBatchRange;
  onRangeChange: (patch: Partial<LocationBatchRange>) => void;
  locationType: string;
  locationTypeOptions: SystemDictionaryOption[];
  onLocationTypeChange: (value: string) => void;
  onImportError: (message: string | null) => void;
}

export function LocationBatchRangeFields({
  scopeOptions,
  scopeValue,
  onScopeValueChange,
  areaOptions,
  range,
  onRangeChange,
  locationType,
  locationTypeOptions,
  onLocationTypeChange,
  onImportError,
}: LocationBatchRangeFieldsProps) {
  return (
    <>
      <div className="grid gap-3 md:grid-cols-[minmax(14rem,1fr)_minmax(12rem,18rem)_12rem]">
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium">仓库 / 库区</span>
          <Select value={scopeValue} onValueChange={onScopeValueChange}>
            <SelectTrigger aria-label="仓库库区">
              <SelectValue placeholder="暂无可用仓库/库区" />
            </SelectTrigger>
            <SelectContent>
              {scopeOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
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
          <span className="font-medium">编码规则</span>
          <Select
            value={range.encoding ?? "standard"}
            onValueChange={(value) => onRangeChange({ encoding: value === "agv" ? "agv" : "standard" })}
          >
            <SelectTrigger aria-label="编码规则">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="standard">高架 A01-排-列-层</SelectItem>
              <SelectItem value="agv">AGV POD排-F层-列</SelectItem>
            </SelectContent>
          </Select>
        </label>
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium">Excel/CSV 导入</span>
          <Input
            type="file"
            accept=".csv,text/csv"
            aria-label="导入库位范围 CSV"
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (!file) return;
              void file.text().then((text) => {
                const parsed = parseLocationBatchCsv(text);
                if (typeof parsed === "string") {
                  onImportError(parsed);
                  return;
                }
                onImportError(null);
                onRangeChange(parsed);
              });
            }}
          />
        </label>
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium">库位类型</span>
          <Select value={locationType} onValueChange={onLocationTypeChange}>
            <SelectTrigger aria-label="库位类型">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {locationTypeOptions.map(([value, label]) => (
                <SelectItem key={value} value={value}>
                  {label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
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
    </>
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

export const LOCATION_BATCH_MAX_COUNT = 500;

export interface LocationBatchRange {
  areaCode: string;
  rowStart: number;
  rowEnd: number;
  columnStart: number;
  columnEnd: number;
  layerStart: number;
  layerEnd: number;
}

export interface LocationBatchPreview {
  totalCount: number;
  codes: string[];
  groups: LocationBatchPreviewGroup[];
}

export interface LocationBatchPreviewGroup {
  rowNo: number;
  columnNo: number;
  label: string;
  codes: string[];
}

export function validateLocationBatchRange(
  range: LocationBatchRange,
  maxCount = LOCATION_BATCH_MAX_COUNT,
): string[] {
  const errors: string[] = [];
  if (!range.areaCode.trim()) errors.push("区域不能为空");
  validateBounds(errors, "排", range.rowStart, range.rowEnd);
  validateBounds(errors, "列", range.columnStart, range.columnEnd);
  validateBounds(errors, "层", range.layerStart, range.layerEnd);

  const totalCount = locationBatchTotalCount(range);
  if (totalCount === 0 && errors.length === 0) errors.push("范围数量不能为 0");
  if (totalCount > maxCount) errors.push(`一次最多生成 ${maxCount} 个库位`);
  return errors;
}

export function buildLocationBatchPreview(range: LocationBatchRange): LocationBatchPreview {
  const totalCount = locationBatchTotalCount(range);
  if (validateLocationBatchRange(range).length > 0) return { totalCount, codes: [], groups: [] };

  const codes: string[] = [];
  const groups: LocationBatchPreviewGroup[] = [];
  const areaCode = range.areaCode.trim().toUpperCase();
  for (let row = range.rowStart; row <= range.rowEnd; row += 1) {
    for (let column = range.columnStart; column <= range.columnEnd; column += 1) {
      const group: LocationBatchPreviewGroup = {
        rowNo: row,
        columnNo: column,
        label: `排 ${pad2(row)} / 列 ${pad2(column)}`,
        codes: [],
      };
      for (let layer = range.layerStart; layer <= range.layerEnd; layer += 1) {
        const code = `${areaCode}-${pad2(row)}-${pad2(column)}-${pad2(layer)}`;
        codes.push(code);
        group.codes.push(code);
      }
      groups.push(group);
    }
  }
  return { totalCount, codes, groups };
}

function locationBatchTotalCount(range: LocationBatchRange) {
  return (
    rangeSize(range.rowStart, range.rowEnd) *
    rangeSize(range.columnStart, range.columnEnd) *
    rangeSize(range.layerStart, range.layerEnd)
  );
}

function rangeSize(start: number, end: number) {
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1 || end < 1 || start > end) {
    return 0;
  }
  return end - start + 1;
}

function validateBounds(errors: string[], label: string, start: number, end: number) {
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1 || end < 1) {
    errors.push(`${label}范围必须是正整数`);
    return;
  }
  if (start > end) errors.push(`${label}起始不能大于${label}结束`);
}

function pad2(value: number) {
  return String(value).padStart(2, "0");
}

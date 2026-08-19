export const LOCATION_BATCH_MAX_COUNT = 500;

export type LocationBatchEncoding = "standard" | "agv";

export interface LocationBatchRange {
  areaCode: string;
  rowStart: number;
  rowEnd: number;
  columnStart: number;
  columnEnd: number;
  layerStart: number;
  layerEnd: number;
  encoding?: LocationBatchEncoding;
  initialPickSequence?: number;
  initialPutawaySequence?: number;
}

export interface LocationBatchPreview {
  totalCount: number;
  codes: string[];
  groups: LocationBatchPreviewGroup[];
  sequences: Array<{ code: string; pickSequenceNo: number; putawaySequenceNo: number }>;
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
  if (validateLocationBatchRange(range).length > 0) {
    return { totalCount, codes: [], groups: [], sequences: [] };
  }

  const codes: string[] = [];
  const groups: LocationBatchPreviewGroup[] = [];
  const sequences: LocationBatchPreview["sequences"] = [];
  const areaCode = range.areaCode.trim().toUpperCase();
  const encoding = range.encoding ?? "standard";
  let pickSequence = range.initialPickSequence ?? 1;
  let putawaySequence = range.initialPutawaySequence ?? 1;
  for (let row = range.rowStart; row <= range.rowEnd; row += 1) {
    for (let column = range.columnStart; column <= range.columnEnd; column += 1) {
      const group: LocationBatchPreviewGroup = {
        rowNo: row,
        columnNo: column,
        label: `排 ${pad2(row)} / 列 ${pad2(column)}`,
        codes: [],
      };
      for (let layer = range.layerStart; layer <= range.layerEnd; layer += 1) {
        const code =
          encoding === "agv"
            ? `POD${pad2(row)}-F${layer}-${pad2(column)}`
            : `${areaCode}-${pad2(row)}-${pad2(column)}-${pad2(layer)}`;
        codes.push(code);
        group.codes.push(code);
        sequences.push({
          code,
          pickSequenceNo: pickSequence,
          putawaySequenceNo: putawaySequence,
        });
        pickSequence += 1;
        putawaySequence += 1;
      }
      groups.push(group);
    }
  }
  return { totalCount, codes, groups, sequences };
}

export interface LocationBatchGeneratePayload {
  rule_type: "high_rack" | "agv";
  prefix?: string;
  row_start?: number;
  row_end?: number;
  column_start?: number;
  column_end?: number;
  layer_start?: number;
  layer_end?: number;
  pod_prefix?: string;
  pod_start?: number;
  pod_end?: number;
  grid_start?: number;
  grid_end?: number;
  initial_pick_sequence: number;
  pick_sequence_step: number;
  initial_putaway_sequence: number;
  putaway_sequence_step: number;
}

/** 预览编码规则 → 后端 batch-generate 请求体，保证确认落库与预览一致。 */
export function toLocationBatchGeneratePayload(range: LocationBatchRange): LocationBatchGeneratePayload {
  const encoding = range.encoding ?? "standard";
  if (encoding === "agv") {
    return {
      rule_type: "agv",
      pod_prefix: "POD",
      pod_start: range.rowStart,
      pod_end: range.rowEnd,
      layer_start: range.layerStart,
      layer_end: range.layerEnd,
      grid_start: range.columnStart,
      grid_end: range.columnEnd,
      initial_pick_sequence: range.initialPickSequence ?? 1,
      pick_sequence_step: 1,
      initial_putaway_sequence: range.initialPutawaySequence ?? 1,
      putaway_sequence_step: 1,
    };
  }
  return {
    rule_type: "high_rack",
    prefix: range.areaCode.trim().toUpperCase(),
    row_start: range.rowStart,
    row_end: range.rowEnd,
    column_start: range.columnStart,
    column_end: range.columnEnd,
    layer_start: range.layerStart,
    layer_end: range.layerEnd,
    initial_pick_sequence: range.initialPickSequence ?? 1,
    pick_sequence_step: 1,
    initial_putaway_sequence: range.initialPutawaySequence ?? 1,
    putaway_sequence_step: 1,
  };
}

export function locationBatchRangeCsv(range: LocationBatchRange): string {
  const encoding = range.encoding ?? "standard";
  const pick = range.initialPickSequence ?? 1;
  const putaway = range.initialPutawaySequence ?? 1;
  const header = "区域,排,列,层,编码,拣选序,上架序";
  const first = [range.areaCode, range.rowStart, range.columnStart, range.layerStart, encoding, pick, putaway].join(",");
  const last = [range.areaCode, range.rowEnd, range.columnEnd, range.layerEnd, encoding, pick, putaway].join(",");
  return `${header}\n${first}\n${last}\n`;
}

export function parseLocationBatchCsv(text: string): LocationBatchRange | string {
  const lines = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (lines.length < 2) return "Excel/CSV 至少需要表头和一行数据";
  const header = lines[0].split(",").map((item) => item.trim().toLowerCase());
  const areaIdx = header.findIndex((item) => item === "area" || item === "区域");
  const rowIdx = header.findIndex((item) => item === "row" || item === "排");
  const colIdx = header.findIndex((item) => item === "column" || item === "列");
  const layerIdx = header.findIndex((item) => item === "layer" || item === "层");
  if (areaIdx < 0 || rowIdx < 0 || colIdx < 0 || layerIdx < 0) {
    return "表头需包含 区域/排/列/层";
  }
  const encodingIdx = header.findIndex((item) => item === "encoding" || item === "编码");
  const pickIdx = header.findIndex((item) => item === "pick_sequence_no" || item === "拣选序");
  const putawayIdx = header.findIndex((item) => item === "putaway_sequence_no" || item === "上架序");
  const first = lines[1].split(",").map((item) => item.trim());
  const last = lines[lines.length - 1].split(",").map((item) => item.trim());
  const toInt = (value: string) => Number.parseInt(value, 10);
  const encodingRaw = encodingIdx >= 0 ? (first[encodingIdx] ?? "").toLowerCase() : "";
  const areaCode = first[areaIdx] ?? "";
  const encoding: LocationBatchEncoding | undefined =
    encodingRaw === "agv" || areaCode.toUpperCase().startsWith("POD") ? "agv" : encodingRaw === "standard" ? "standard" : undefined;
  const pickRaw = pickIdx >= 0 ? toInt(first[pickIdx] ?? "") : Number.NaN;
  const putawayRaw = putawayIdx >= 0 ? toInt(first[putawayIdx] ?? "") : Number.NaN;
  return {
    areaCode,
    rowStart: toInt(first[rowIdx] ?? "1"),
    rowEnd: toInt(last[rowIdx] ?? first[rowIdx] ?? "1"),
    columnStart: toInt(first[colIdx] ?? "1"),
    columnEnd: toInt(last[colIdx] ?? first[colIdx] ?? "1"),
    layerStart: toInt(first[layerIdx] ?? "1"),
    layerEnd: toInt(last[layerIdx] ?? first[layerIdx] ?? "1"),
    encoding,
    initialPickSequence: Number.isInteger(pickRaw) && pickRaw > 0 ? pickRaw : undefined,
    initialPutawaySequence: Number.isInteger(putawayRaw) && putawayRaw > 0 ? putawayRaw : undefined,
  };
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

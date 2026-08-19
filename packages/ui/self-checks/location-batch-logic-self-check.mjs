import assert from "node:assert/strict";
import {
  buildLocationBatchPreview,
  locationBatchRangeCsv,
  parseLocationBatchCsv,
  toLocationBatchGeneratePayload,
  validateLocationBatchRange,
} from "../src/lib/location-batch.ts";

const validRange = {
  areaCode: "A01",
  rowStart: 1,
  rowEnd: 2,
  columnStart: 1,
  columnEnd: 3,
  layerStart: 1,
  layerEnd: 2,
};

assert.deepEqual(validateLocationBatchRange(validRange), []);

const preview = buildLocationBatchPreview(validRange);
assert.equal(preview.totalCount, 12);
assert.deepEqual(preview.codes, [
  "A01-01-01-01",
  "A01-01-01-02",
  "A01-01-02-01",
  "A01-01-02-02",
  "A01-01-03-01",
  "A01-01-03-02",
  "A01-02-01-01",
  "A01-02-01-02",
  "A01-02-02-01",
  "A01-02-02-02",
  "A01-02-03-01",
  "A01-02-03-02",
]);
assert.equal(preview.sequences.length, 12);
assert.equal(preview.sequences[0].pickSequenceNo, 1);
const agv = buildLocationBatchPreview({ ...validRange, encoding: "agv" });
assert.equal(agv.codes[0], "POD01-F1-01");
assert.equal(agv.codes[1], "POD01-F2-01");
const csv = parseLocationBatchCsv("区域,排,列,层\nA01,1,1,1\nA01,2,3,2");
assert.equal(typeof csv === "object" && csv.areaCode, "A01");
const csvAgv = parseLocationBatchCsv("区域,排,列,层,编码\nPOD,1,1,1,agv\nPOD,2,3,2,agv");
assert.equal(typeof csvAgv === "object" && csvAgv.encoding, "agv");
assert.equal(parseLocationBatchCsv("坏表头\n1"), "表头需包含 区域/排/列/层");
const exported = locationBatchRangeCsv({
  ...validRange,
  encoding: "agv",
  initialPickSequence: 10,
  initialPutawaySequence: 20,
});
const reimported = parseLocationBatchCsv(exported);
assert.equal(typeof reimported === "object" && reimported.encoding, "agv");
assert.equal(typeof reimported === "object" && reimported.rowStart, 1);
assert.equal(typeof reimported === "object" && reimported.rowEnd, 2);
assert.equal(typeof reimported === "object" && reimported.columnEnd, 3);
assert.equal(typeof reimported === "object" && reimported.layerEnd, 2);
assert.equal(typeof reimported === "object" && reimported.initialPickSequence, 10);
assert.equal(typeof reimported === "object" && reimported.initialPutawaySequence, 20);
const generateFromCsv = toLocationBatchGeneratePayload(reimported);
assert.equal(generateFromCsv.rule_type, "agv");
assert.equal(generateFromCsv.initial_pick_sequence, 10);
assert.equal(generateFromCsv.initial_putaway_sequence, 20);
const previewFromCsv = buildLocationBatchPreview(reimported);
assert.equal(previewFromCsv.sequences[0].pickSequenceNo, 10);
assert.equal(previewFromCsv.sequences[0].putawaySequenceNo, 20);
const generateAgv = toLocationBatchGeneratePayload({ ...validRange, encoding: "agv" });
assert.equal(generateAgv.rule_type, "agv");
assert.equal(generateAgv.pod_start, 1);
assert.equal(generateAgv.grid_end, 3);
const generateRack = toLocationBatchGeneratePayload(validRange);
assert.equal(generateRack.rule_type, "high_rack");
assert.equal(generateRack.prefix, "A01");
assert.deepEqual(preview.groups, [
  {
    rowNo: 1,
    columnNo: 1,
    label: "排 01 / 列 01",
    codes: ["A01-01-01-01", "A01-01-01-02"],
  },
  {
    rowNo: 1,
    columnNo: 2,
    label: "排 01 / 列 02",
    codes: ["A01-01-02-01", "A01-01-02-02"],
  },
  {
    rowNo: 1,
    columnNo: 3,
    label: "排 01 / 列 03",
    codes: ["A01-01-03-01", "A01-01-03-02"],
  },
  {
    rowNo: 2,
    columnNo: 1,
    label: "排 02 / 列 01",
    codes: ["A01-02-01-01", "A01-02-01-02"],
  },
  {
    rowNo: 2,
    columnNo: 2,
    label: "排 02 / 列 02",
    codes: ["A01-02-02-01", "A01-02-02-02"],
  },
  {
    rowNo: 2,
    columnNo: 3,
    label: "排 02 / 列 03",
    codes: ["A01-02-03-01", "A01-02-03-02"],
  },
]);

const fullMatrixPreview = buildLocationBatchPreview(
  {
    areaCode: "A01",
    rowStart: 1,
    rowEnd: 2,
    columnStart: 1,
    columnEnd: 3,
    layerStart: 1,
    layerEnd: 5,
  }
);
assert.equal(fullMatrixPreview.totalCount, 30);
assert.deepEqual(
  fullMatrixPreview.groups.map((group) => group.label),
  [
    "排 01 / 列 01",
    "排 01 / 列 02",
    "排 01 / 列 03",
    "排 02 / 列 01",
    "排 02 / 列 02",
    "排 02 / 列 03",
  ],
);
assert.deepEqual(fullMatrixPreview.groups.at(-1)?.codes, [
  "A01-02-03-01",
  "A01-02-03-02",
  "A01-02-03-03",
  "A01-02-03-04",
  "A01-02-03-05",
]);

assert.deepEqual(validateLocationBatchRange({ ...validRange, rowStart: 3 }), [
  "排起始不能大于排结束",
]);

assert.deepEqual(validateLocationBatchRange({ ...validRange, rowEnd: 10, columnEnd: 10, layerEnd: 10 }), [
  "一次最多生成 500 个库位",
]);

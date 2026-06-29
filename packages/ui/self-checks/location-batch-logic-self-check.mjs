import assert from "node:assert/strict";
import {
  buildLocationBatchPreview,
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

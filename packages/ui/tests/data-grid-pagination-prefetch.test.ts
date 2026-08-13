import assert from "node:assert/strict";
import { getDataGridPrefetchPageIndexes } from "../src/business/DataGrid/data-grid-pagination-prefetch.ts";

assert.deepEqual(
  getDataGridPrefetchPageIndexes({ pageIndex: 0, pageSize: 20, total: 13757, prefetchCount: 2 }),
  [1, 2],
);

assert.deepEqual(
  getDataGridPrefetchPageIndexes({ pageIndex: 686, pageSize: 20, total: 13757, prefetchCount: 2 }),
  [687],
);

assert.deepEqual(
  getDataGridPrefetchPageIndexes({ pageIndex: 687, pageSize: 20, total: 13757, prefetchCount: 2 }),
  [],
);

console.log("data-grid-pagination-prefetch tests passed");

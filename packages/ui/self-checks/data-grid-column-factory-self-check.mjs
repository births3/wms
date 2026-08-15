import assert from "node:assert/strict";
import {
  createDateColumn,
  createMonoColumn,
  createNumericColumn,
  createStatusColumn,
} from "../src/business/DataGrid/data-grid-column-factory.ts";

const mockRow = {
  id: "1",
  orderNo: "OUT-001",
  status: "pending",
  qty: 1200,
  createdAt: "2026-08-15",
};

// 1. 状态列测试
const statusCol = createStatusColumn({
  key: "status",
  header: "单据状态",
  statusMap: {
    pending: { label: "处理中", status: "pending" },
    done: { label: "已完成", status: "completed" },
  },
});
assert.equal(statusCol.key, "status");
assert.equal(statusCol.header, "单据状态");
assert.equal(statusCol.sortable, true);
assert.equal(statusCol.copyValue(mockRow), "处理中");

// 2. 单号列测试
const monoCol = createMonoColumn({
  key: "orderNo",
  header: "出库单号",
});
assert.equal(monoCol.mono, true);
assert.equal(monoCol.copyable, true);
assert.equal(monoCol.copyValue(mockRow), "OUT-001");

// 3. 日期列测试
const dateCol = createDateColumn({
  key: "createdAt",
  header: "创建时间",
  formatter: (val) => `[${val}]`,
});
assert.equal(dateCol.copyValue(mockRow), "[2026-08-15]");

// 4. 数值列测试
const numericCol = createNumericColumn({
  key: "qty",
  header: "件数",
  unit: "件",
});
assert.equal(numericCol.align, "right");
assert.equal(numericCol.sortValue(mockRow), 1200);
assert.equal(numericCol.copyValue(mockRow), "1200 件");

console.log("data-grid-column-factory self-check passed");

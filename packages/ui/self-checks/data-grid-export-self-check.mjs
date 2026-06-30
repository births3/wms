import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  buildDataGridCsv,
  downloadDataGridCsv,
} from "../src/business/DataGrid/data-grid-export.ts";

const columns = [
  { key: "code", header: "单号" },
  {
    key: "status",
    header: "状态",
    filterValue: (row) => row.status,
    copyValue: (row) => row.statusLabel,
  },
  { key: "note", header: "备注" },
  { key: "hidden", header: "隐藏列" },
  { key: "createdAt", header: "创建时间" },
  { key: "tags", header: "标签" },
  { key: "meta", header: "元数据" },
  { key: "empty", header: "空值" },
  { key: "operator", header: { label: "操作人" }, filterValue: (row) => row.operatorName },
];

const rows = [
  {
    code: "ASN-001",
    status: "released",
    statusLabel: '待"收,货',
    note: "冷链\n换行",
    hidden: "不应导出",
    createdAt: new Date("2026-06-30T00:00:00.000Z"),
    tags: ["冷链", "急,件"],
    meta: { owner: "华东", count: 2 },
    empty: null,
    operatorName: "张三",
  },
];

const csv = buildDataGridCsv({
  columns,
  visibleColumnKeys: ["status", "code", "note", "createdAt", "tags", "meta", "empty", "operator"],
  rows,
});

assert.equal(
  csv,
  [
    "状态,单号,备注,创建时间,标签,元数据,空值,operator",
    '"待""收,货",ASN-001,"冷链\n换行",2026-06-30T00:00:00.000Z,' +
      '"冷链 急,件",华东 2,,张三',
  ].join("\n"),
);
assert.equal(csv.includes("隐藏列"), false);
assert.equal(csv.includes("不应导出"), false);
assert.equal(downloadDataGridCsv({ csv, fileName: "data-grid.csv" }), false);

let removed = false;
let revokedHref = "";
const failingLink = {
  href: "",
  download: "",
  click() {
    throw new Error("click failed");
  },
  remove() {
    removed = true;
  },
};
const fakeDocument = {
  body: {
    appendChild(link) {
      assert.equal(link, failingLink);
    },
  },
  createElement(tag) {
    assert.equal(tag, "a");
    return failingLink;
  },
};
const fakeUrl = {
  createObjectURL() {
    return "blob:data-grid";
  },
  revokeObjectURL(href) {
    revokedHref = href;
  },
};

assert.throws(
  () =>
    downloadDataGridCsv({
      csv,
      fileName: "data-grid.csv",
      document: fakeDocument,
      url: fakeUrl,
    }),
  /click failed/,
);
assert.equal(removed, true);
assert.equal(revokedHref, "blob:data-grid");

const dataGridSource = readFileSync(
  new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url),
  "utf8",
);

assert.match(
  dataGridSource,
  /import \{\s*buildDataGridCsv,\s*downloadDataGridCsv\s*\} from "\.\/data-grid-export";/s,
);
assert.match(dataGridSource, /import \{ Button \} from "\.\.\/\.\.\/ui\/button";/);
assert.match(dataGridSource, /import \{ Download \} from "lucide-react";/);
assert.match(
  dataGridSource,
  /const csv = buildDataGridCsv\(\{\s*columns,\s*visibleColumnKeys: visibleColumns\.map\(\(column\) => column\.key\),\s*rows: page\.filteredRows,\s*\}\);/s,
);
assert.match(
  dataGridSource,
  /fileName: storageKey \? `\$\{storageKey\}\.csv` : "data-grid\.csv"/,
);
assert.match(
  dataGridSource,
  /document: typeof document === "undefined" \? undefined : document/,
);
assert.match(
  dataGridSource,
  /<Button[\s\S]*disabled=\{page\.filteredRows\.length === 0\}[\s\S]*导出 CSV[\s\S]*<\/Button>/,
);

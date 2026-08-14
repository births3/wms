import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const dataTableSource = readFileSync(new URL("../src/business/DataTable/DataTable.tsx", import.meta.url), "utf8");
const dataGridSource = readFileSync(new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url), "utf8");

// 1. DataTable 根容器必须具备 min-h-[380px] 兜底，防止数据少或空数据时表格高度塌陷
assert.match(
  dataTableSource,
  /className=\{cn\("flex min-h-\[380px\] flex-col rounded-md border bg-background/,
  "DataTable root element must include min-h-[380px] to prevent height collapse",
);

// 2. DataGrid 根容器必须具备 min-h-[380px]
assert.match(
  dataGridSource,
  /className=\{cn\("flex h-full min-h-\[380px\] flex-col gap-3"/,
  "DataGrid root element must include min-h-[380px]",
);

// 3. 底部翻页控制栏必须包含 mt-auto，确保在任何高度下均稳定贴附在表格容器底部
assert.match(
  dataTableSource,
  /ref=\{bottomBarRef\}[^>]*className=\{cn\(\s*"mt-auto shrink-0 border-t bg-background/,
  "DataTable bottomBarRef must include mt-auto for pinned footer behavior",
);

// 4. 空状态必须包含垂直居中和充足的可视高度
assert.match(
  dataTableSource,
  /TableCell colSpan=\{columns\.length\} className="h-64 py-12 text-center align-middle"/,
  "DataTable empty state TableCell must have adequate height and vertical alignment",
);
assert.match(
  dataTableSource,
  /div className="sticky left-0 flex w-full justify-center"/,
  "DataTable empty state must be centered within the viewport width",
);

console.log("✓ data-grid-layout-expand tests passed!");

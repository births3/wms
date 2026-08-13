import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const dataTableSource = readFileSync(new URL("../src/business/DataTable/DataTable.tsx", import.meta.url), "utf8");
const tableSource = readFileSync(new URL("../src/ui/table.tsx", import.meta.url), "utf8");

// DataTable（business 层）：数据行自带不透明背景（[&_tr]:bg-background 挂在 TableBody 调用处），
// 不再透出外层容器底色；放 business 层而非 ui 原语层，直接使用 ui/Table 的消费方（如 Card 内表格）不受影响
assert.match(dataTableSource, /<TableBody[^>]*\[&_tr\]:bg-background/);
// 负向断言：ui 原语层 thead/tbody 不得携带不透明行底色（防止回归把底色下放到共享原语）
assert.doesNotMatch(tableSource, /<thead[^>]*bg-background/);
assert.doesNotMatch(tableSource, /\[&_tr\]:bg-background/);
// hover/selected 半透明态在 TableRow 上同元素叠加覆盖（特异性 0-2-0 高于 [&_tr]: 规则的 0-1-1）
assert.match(tableSource, /TableRow = /);
assert.match(tableSource, /hover:bg-accent\/40/);
assert.match(tableSource, /data-\[state=selected\]:bg-primary\/10/);

console.log("table-row-background: OK");

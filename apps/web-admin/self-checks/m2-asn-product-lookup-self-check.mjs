import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const dialogs = readFileSync(new URL("../src/pages/inbound/M2InboundDialogs.tsx", import.meta.url), "utf8");
const lookup = readFileSync(new URL("../src/pages/inbound/M2InboundProductLookup.tsx", import.meta.url), "utf8");
const queries = readFileSync(new URL("../src/features/master-data/master-data-queries/queries.ts", import.meta.url), "utf8");

assert.match(dialogs, /useMasterDataRowsQuery\("m1-products", activeDialog === "create"\)/);
assert.match(dialogs, /<ProductLookupField/);
assert.match(dialogs, /<ProductLookupField[\s\S]*placeholder="例如 P-M2-002"[\s\S]*required/);
assert.match(lookup, /onDoubleClick=\{onOpenLookup\}/);
assert.match(lookup, /placeholder=\{placeholder\}/);
assert.match(lookup, /required=\{required\}/);
assert.match(lookup, /filterAsnProductLookupRows\(products, value\)\.slice\(0, 6\)/);
assert.match(lookup, /<span>商品编码<\/span>[\s\S]*<span>商品名称<\/span>[\s\S]*<span>规格<\/span>[\s\S]*<span>批号<\/span>/);
assert.match(lookup, /<DialogTitle>关联商品档案<\/DialogTitle>/);
assert.match(queries, /enabled,/);

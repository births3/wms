import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);
const read = (file) => readFileSync(new URL(file, root), "utf8");
const page = read("src/pages/master-data/DualPersonPolicyMatrix.tsx");
const queries = read("src/features/validation-rules/dual-person-policy-queries.ts");
const dictionaryPage = read("src/pages/master-data/SystemDictionaryPage.tsx");
const inboundPage = read("src/pages/inbound/M2InboundPage.tsx");
const outboundPage = read("src/pages/outbound/M4OutboundPage.tsx");

assert.match(dictionaryPage, /<DualPersonPolicyMatrix/);
for (const process of ["入库", "出库", "报损", "报溢", "销毁", "退货"]) {
  assert.match(page, new RegExp(process));
}
for (const policy of ["single", "dual_scan", "dual_scan_with_approval"]) {
  assert.match(page + queries, new RegExp(policy));
}
assert.match(page, /confirmed_by_user_id/);
assert.match(page, /仓库级|货主级/);
assert.match(queries, /m-vr\/dual-person-policy\/rules/);
assert.match(queries, /Idempotency-Key/);
assert.match(queries, /invalidateQueries/);
assert.match(inboundPage, /process: "入库"[\s\S]*node: "验收"/);
assert.match(outboundPage, /process: "出库"[\s\S]*node: "复核"/);
console.log("M-VR dual-person matrix self-check passed");

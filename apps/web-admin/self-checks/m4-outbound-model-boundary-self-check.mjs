import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const names = [
  "m4-outbound-page-model.ts",
  "M4OutboundDetailDialog.tsx",
  "M4OutboundPageParts.tsx",
  "M4OutboundPage.tsx",
  "M4OutboundActionDialog.tsx",
  "m4-outbound-page-helpers.ts",
  "M4OutboundGridColumns.tsx",
];
const files = new Map(names.map((name) => [name, readFileSync(resolve(root, "src/pages/outbound", name), "utf8")]));
const graph = new Map(names.map((name) => [name, []]));

for (const [name, source] of files) {
  for (const [, imported] of source.matchAll(/from\s+["'](\.\/[^"']+)["']/g)) {
    const target = names.find((candidate) => imported === `./${candidate.replace(/\.tsx?$/, "")}`);
    if (target) graph.get(name).push(target);
  }
}

const visiting = new Set();
const visited = new Set();
function visit(name) {
  if (visiting.has(name)) return true;
  if (visited.has(name)) return false;
  visiting.add(name);
  const cycle = graph.get(name).some(visit);
  visiting.delete(name);
  visited.add(name);
  return cycle;
}

assert.equal(visit("m4-outbound-page-model.ts"), false, "M4 页面模型不能参与组件循环依赖");
assert.doesNotMatch(files.get("m4-outbound-page-model.ts"), /M4OutboundDetailDialog/);
console.log("m4 outbound model boundary self-check passed");

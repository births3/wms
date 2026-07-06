import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const pageSource = read("src/pages/print-template/H9PrintTemplatePage.tsx");
const querySource = read("src/features/print-template/print-template-queries.ts");
const businessIndexSource = read("../../packages/ui/src/business/index.ts");
const componentRegistrySource = read("../../docs/prototypes/component-registry.md");

assert.match(pageSource, /TreeCatalog/);
assert.match(pageSource, /buildH9TreeNodes/);
assert.match(pageSource, /filterRowsByTree/);
assert.match(pageSource, /storageKey="h9\.print-template\.tree"/);
assert.match(querySource, /usePrintTemplateTypesQuery/);
assert.match(querySource, /print_template_type/);
assert.match(businessIndexSource, /export \{ TreeCatalog \}/);
assert.match(componentRegistrySource, /\*\*TreeCatalog\*\*/);

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

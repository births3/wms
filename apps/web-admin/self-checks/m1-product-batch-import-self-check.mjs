import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { createServer } from "vite";

const root = path.resolve(new URL("..", import.meta.url).pathname);
const page = fs.readFileSync(path.join(root, "src/pages/master-data/M1MasterDataPage.tsx"), "utf8");
const api = fs.readFileSync(
  path.join(root, "src/features/master-data/master-data-queries/api.ts"),
  "utf8",
);
const devMock = fs.readFileSync(
  path.join(root, "dev-mocks/web-admin-dev-mock-core.ts"),
  "utf8",
);

assert.doesNotMatch(page, /ProductBatchImportDialog/);
assert.doesNotMatch(page, /ProductCreateDialog/);
assert.doesNotMatch(api, /batchCreateProducts/);
assert.doesNotMatch(api, /web-m1-product-create/);
assert.doesNotMatch(api, /web-m1-product-update/);
assert.match(devMock, /AUTH-005/);
assert.doesNotMatch(devMock, /devCreateProduct/);
assert.doesNotMatch(devMock, /handleProductUpdate/);

const server = await createServer({
  root,
  logLevel: "silent",
  server: { middlewareMode: true },
  appType: "custom",
});
try {
  const { masterDataActionLabels } = await server.ssrLoadModule(
    "/src/pages/master-data/m1-product-page-model.ts",
  );
  assert.deepEqual(masterDataActionLabels("m1-products"), []);
} finally {
  await server.close();
}

console.log("m1 ERP-only product projection self-check passed");

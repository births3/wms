import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const server = await createServer({
  root: fileURLToPath(new URL("..", import.meta.url)),
  logLevel: "silent",
  server: { middlewareMode: true },
  appType: "custom",
});

try {
  const { parseFeatureFlagImportJson } = await server.ssrLoadModule(
    "/src/features/config-center/feature-flag-queries.ts",
  );

  assert.equal(
    parseFeatureFlagImportJson(JSON.stringify({ flags: [flag("m1_config_center_feature_flags")] }))
      .length,
    1,
  );
  assert.equal(parseFeatureFlagImportJson(JSON.stringify([flag("m1_array_flag")]))[0]?.source, "file");
  assert.throws(() => parseFeatureFlagImportJson("{}"), /flags 数组/);
  assert.throws(() => parseFeatureFlagImportJson(JSON.stringify([{}])), /enabled 必须是布尔值/);
} finally {
  await server.close();
}

function flag(key) {
  return {
    key,
    owner: "platform",
    created_at: "2026-07-03",
    cleanup_by: "2026-10-01",
    enabled: true,
    source: "file",
  };
}

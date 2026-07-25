import { defineConfig, devices } from "@playwright/test";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const baseURL = process.env.WMS_MDI_E2E_WEB_URL ?? "http://127.0.0.1:19281";
const apiURL = process.env.WMS_MDI_E2E_API_URL ?? "http://127.0.0.1:19280";
const databaseURL = process.env.DATABASE_URL ?? process.env.WMS_DB_URL;
const attachmentRoot = process.env.WMS_E2E_ATTACHMENT_ROOT ??
  path.resolve("../artifacts/screenshot-portal/real-web/m-di/attachments");
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ||
  (fs.existsSync("/usr/bin/google-chrome") ? "/usr/bin/google-chrome" : undefined);

if (!databaseURL) throw new Error("DATABASE_URL or WMS_DB_URL is required for M-DI real E2E");

function bindAddress(url: string) {
  const parsed = new URL(url);
  return `${parsed.hostname}:${parsed.port || "80"}`;
}

export default defineConfig({
  testDir: "./e2e",
  testMatch: /web-admin-mdi-real\.spec\.ts/,
  timeout: 180_000,
  expect: { timeout: 12_000 },
  workers: 1,
  reporter: [
    ["list"],
    ["json", { outputFile: "../apps/web-admin/.e2e-artifacts/mdi-real/playwright-report.json" }],
  ],
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    trace: "on",
    screenshot: "only-on-failure",
    actionTimeout: 12_000,
    launchOptions: executablePath ? { executablePath } : undefined,
  },
  webServer: [
    {
      command: "CARGO_INCREMENTAL=0 cargo run --manifest-path ../backend/Cargo.toml -p wms-api --example wms_api_e2e",
      url: `${apiURL}/api/v1/healthz`,
      reuseExistingServer: false,
      timeout: 180_000,
      env: {
        ...process.env,
        DATABASE_URL: databaseURL,
        WMS_BIND_ADDR: bindAddress(apiURL),
        WMS_E2E_SEED: "1",
        WMS_JWT_SECRET: `mdi-real-e2e-${crypto.randomUUID()}`,
        WMS_E2E_ATTACHMENT_ROOT: attachmentRoot,
      },
    },
    {
      command: `pnpm --dir ${path.resolve("../apps/web-admin")} exec vite --host 127.0.0.1 --port ${new URL(baseURL).port || "19281"} --strictPort`,
      url: baseURL,
      reuseExistingServer: false,
      timeout: 120_000,
      env: { ...process.env, WMS_WEB_ADMIN_E2E_API_URL: apiURL },
    },
  ],
});

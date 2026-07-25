import { defineConfig, devices } from "@playwright/test";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const baseURL = process.env.PORTAL_E2E_WEB_URL ?? "http://127.0.0.1:19191";
const apiURL = process.env.PORTAL_E2E_API_URL ?? "http://127.0.0.1:19190";
const databaseURL = process.env.PORTAL_DATABASE_URL;
const storageRoot = process.env.PORTAL_H_FILE_STORAGE_ROOT ??
  path.resolve("../artifacts/screenshot-portal/real-web/customer-portal/storage");
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ||
  (fs.existsSync("/usr/bin/google-chrome") ? "/usr/bin/google-chrome" : undefined);

if (!databaseURL) throw new Error("PORTAL_DATABASE_URL is required for customer portal real E2E");

export default defineConfig({
  testDir: "./e2e",
  testMatch: /customer-portal-real\.spec\.ts/,
  timeout: 120_000,
  expect: { timeout: 12_000 },
  workers: 1,
  reporter: [
    ["list"],
    ["json", { outputFile: "../apps/customer-portal/.e2e-artifacts/real/playwright-report.json" }],
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
      command: "CARGO_INCREMENTAL=0 cargo run --manifest-path ../backend/Cargo.toml -p wms-customer-portal-api --example customer_portal_e2e",
      url: `${apiURL}/health`,
      reuseExistingServer: false,
      timeout: 120_000,
      env: {
        ...process.env,
        PORTAL_DATABASE_URL: databaseURL,
        PORTAL_JWT_SECRET: `portal-e2e-${crypto.randomUUID()}`,
        PORTAL_PROJECTION_KEY: "portal-real-e2e-projection-key",
        PORTAL_H_FILE_STORAGE_ROOT: storageRoot,
        PORTAL_BIND: new URL(apiURL).host,
      },
    },
    {
      command: `pnpm --dir ${path.resolve("../apps/customer-portal")} exec vite --host 127.0.0.1 --port ${new URL(baseURL).port || "19191"} --strictPort`,
      url: baseURL,
      reuseExistingServer: false,
      timeout: 120_000,
      env: { ...process.env, PORTAL_E2E_API_URL: apiURL },
    },
  ],
});

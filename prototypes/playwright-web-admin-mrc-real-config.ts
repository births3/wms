import { defineConfig, devices } from "@playwright/test";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const baseURL = process.env.WMS_WEB_ADMIN_E2E_BASE_URL ?? "http://127.0.0.1:19206";
const apiURL = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19207";
const databaseURL = process.env.DATABASE_URL ?? process.env.WMS_DB_URL;
const jwtSigningKey = process.env.WMS_JWT_SECRET ?? `web-admin-mrc-real-e2e-${crypto.randomUUID()}`;
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ||
  (fs.existsSync("/usr/bin/google-chrome") ? "/usr/bin/google-chrome" : undefined);

if (!databaseURL) throw new Error("DATABASE_URL or WMS_DB_URL is required for M-RC real-data E2E");

function bindAddr(url: string) {
  const parsed = new URL(url);
  return `${parsed.hostname}:${parsed.port || (parsed.protocol === "https:" ? "443" : "80")}`;
}

export default defineConfig({
  testDir: "./e2e",
  testMatch: /web-admin-mrc-real\.spec\.ts/,
  timeout: 90_000,
  expect: { timeout: 10_000 },
  workers: 1,
  reporter: [["list"], ["json", { outputFile: "../apps/web-admin/.e2e-artifacts/mrc-real/playwright-report.json" }]],
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    trace: "on",
    screenshot: "only-on-failure",
    launchOptions: executablePath ? { executablePath } : undefined,
  },
  webServer: [
    {
      command: "cargo run --manifest-path ../backend/Cargo.toml -p wms-api --example wms_api_e2e",
      url: `${apiURL}/api/v1/healthz`,
      reuseExistingServer: false,
      timeout: 120_000,
      env: {
        ...process.env,
        DATABASE_URL: databaseURL,
        WMS_BIND_ADDR: bindAddr(apiURL),
        WMS_E2E_SEED: "1",
        WMS_JWT_SECRET: jwtSigningKey,
      },
    },
    {
      command: `pnpm --dir ${path.join("..", "apps", "web-admin")} dev --host 127.0.0.1 --port ${new URL(baseURL).port || "19206"}`,
      url: baseURL,
      reuseExistingServer: false,
      timeout: 120_000,
      env: { ...process.env, WMS_WEB_ADMIN_E2E_API_URL: apiURL },
    },
  ],
});

import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";

const baseURL = process.env.WMS_WEB_ADMIN_H4_DEV_E2E_BASE_URL ?? "http://127.0.0.1:19083";
const port = new URL(baseURL).port || "19083";
const executablePath =
  process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ||
  (fs.existsSync("/usr/bin/google-chrome") ? "/usr/bin/google-chrome" : undefined);

export default defineConfig({
  testDir: "./e2e",
  testMatch: /web-admin-h4-dev\.spec\.ts/,
  timeout: 45_000,
  expect: { timeout: 8_000 },
  fullyParallel: false,
  workers: 1,
  reporter: [
    ["list"],
    ["json", { outputFile: "../apps/web-admin/.e2e-artifacts/h4-dev/playwright-report.json" }],
  ],
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    trace: "on",
    screenshot: "only-on-failure",
    video: "off",
    actionTimeout: 8_000,
    launchOptions: executablePath ? { executablePath } : undefined,
  },
  webServer: {
    command: `pnpm --dir ../apps/web-admin dev --host 127.0.0.1 --port ${port} --strictPort`,
    url: baseURL,
    reuseExistingServer: false,
    timeout: 120_000,
    env: {
      ...process.env,
      WMS_WEB_ADMIN_DEV_MOCK: "1",
    },
  },
});

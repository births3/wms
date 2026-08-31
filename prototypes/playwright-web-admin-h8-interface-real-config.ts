import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";

const baseURL = process.env.WMS_WEB_ADMIN_H8_INTERFACE_REAL_BASE_URL ?? "http://127.0.0.1:9002";
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ||
  (fs.existsSync("/usr/bin/google-chrome") ? "/usr/bin/google-chrome" : undefined);

export default defineConfig({
  testDir: "./e2e",
  testMatch: /web-admin-h8-interface-tables-real\.spec\.ts/,
  timeout: 60_000,
  expect: { timeout: 15_000 },
  workers: 1,
  reporter: [
    ["list"],
    ["json", { outputFile: "../apps/web-admin/.e2e-artifacts/h8-interface-real/playwright-report.json" }],
  ],
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    trace: "on",
    screenshot: "only-on-failure",
    launchOptions: executablePath ? { executablePath } : undefined,
  },
});

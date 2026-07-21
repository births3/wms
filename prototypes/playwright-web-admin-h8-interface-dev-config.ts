import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";

const baseURL = process.env.WMS_WEB_ADMIN_H8_INTERFACE_DEV_BASE_URL ?? "http://127.0.0.1:19083";
const port = new URL(baseURL).port || "19083";
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ||
  (fs.existsSync("/usr/bin/google-chrome") ? "/usr/bin/google-chrome" : undefined);

export default defineConfig({
  testDir: "./e2e",
  testMatch: /web-admin-h8-interface-tables\.spec\.ts/,
  timeout: 30_000,
  expect: { timeout: 10_000 },
  workers: 1,
  reporter: [["list"]],
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    trace: "on",
    screenshot: "only-on-failure",
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

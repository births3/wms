import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.WMS_WEB_ADMIN_DEV_E2E_BASE_URL ?? "http://127.0.0.1:19082";
const port = new URL(baseURL).port || "19082";
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || undefined;

export default defineConfig({
  testDir: "./e2e",
  testMatch: /web-admin-shell\.spec\.ts/,
  timeout: 30_000,
  expect: { timeout: 5_000 },
  fullyParallel: false,
  workers: 1,
  reporter: [["list"]],
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    trace: "on",
    screenshot: "only-on-failure",
    video: "off",
    actionTimeout: 5_000,
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

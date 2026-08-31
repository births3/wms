import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.MATRIX_E2E_BASE_URL ?? "http://127.0.0.1:5173";
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || undefined;

export default defineConfig({
  testDir: "./e2e",
  testMatch: /matrix\.spec\.ts/,
  timeout: 30_000,
  expect: { timeout: 5_000 },
  fullyParallel: false,
  workers: 1,
  reporter: [
    ["list"],
    ["json", { outputFile: ".e2e-artifacts/playwright-report.json" }],
  ],
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
    command: "pnpm dev --host 127.0.0.1",
    url: baseURL,
    reuseExistingServer: true,
    timeout: 120_000,
  },
});

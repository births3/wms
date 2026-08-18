import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m3-replenishment-strategies");

test("M3 补货策略配置页可打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await page.goto("/#/m3-replenishment-strategies");
  await expect(page.getByTestId("m3-replenishment-strategy-page")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "strategy-saved.png"), fullPage: false });
});

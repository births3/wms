import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m3-replenishment-tasks");

test("M3 补货任务大盘可打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await page.goto("/#/m3-replenishment-tasks");
  await expect(page.getByTestId("m3-replenishment-task-page")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "task-board.png"), fullPage: false });
});

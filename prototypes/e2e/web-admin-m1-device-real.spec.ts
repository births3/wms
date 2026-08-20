import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-devices");



async function openMenu(page: import("@playwright/test").Page, parents: string[], menu: RegExp) {
  const nav = page.getByRole("navigation");
  for (const parent of parents) {
    const btn = nav.getByRole("button", { name: parent, exact: true });
    await expect(btn).toBeVisible();
    if ((await btn.getAttribute("aria-expanded")) !== "true") {
      await btn.click();
    }
  }
  const menuBtn = nav.getByRole("button", { name: menu });
  await expect(menuBtn).toBeVisible();
  await menuBtn.click();
}

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

test("M1 设备档案页可打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openMenu(page, ["基础档案", "主数据"], /M1 设备档案/);
  await expect(page.getByTestId("m1-device-page")).toBeVisible();
  await expect(page.getByText("读取设备失败")).toHaveCount(0);
  await expect(page.getByText("暂无设备")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "device-list.png"), fullPage: false });
});

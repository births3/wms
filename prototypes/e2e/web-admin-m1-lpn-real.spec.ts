import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-lpn-containers");

test("M1 容器管理真实登录后创建托盘并回显 LPN", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础档案", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") {
    await section.click();
  }
  const group = navigation.getByRole("button", { name: "仓储资料", exact: true });
  if ((await group.getAttribute("aria-expanded")) !== "true") {
    await group.click();
  }
  await navigation.getByRole("button", { name: /M1 容器管理/ }).click();

  await expect(page.getByRole("heading", { name: "M1 容器管理", level: 2 })).toBeVisible({
    timeout: 15_000,
  });
  await expect(page.getByText("LPN", { exact: true }).first()).toBeVisible();
  const policyTitle = page.getByText("类型策略（默认禁止混批/混品）");
  await policyTitle.scrollIntoViewIfNeeded();
  await expect(policyTitle).toBeVisible();

  await page.getByRole("button", { name: "新增", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "创建容器" });
  await expect(dialog).toBeVisible();
  const createResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/master-data/lpn-containers") &&
      response.request().method() === "POST",
  );
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  const created = await createResponse;
  expect(created.ok()).toBeTruthy();
  const body = (await created.json()) as { lpn_code?: string };
  expect(body.lpn_code).toMatch(/^LPN-PL-/);
  await expect(page.getByText(body.lpn_code ?? "", { exact: true })).toBeVisible();

  await page.screenshot({
    path: path.join(artifactsDir, "lpn-containers.png"),
    fullPage: false,
  });
});

import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/mcg-real/screenshots");

test("M-CG 规则配置使用真实 API 完成新增、预览、编辑和停用", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  const ruleCode = `cg-e2e-${Date.now()}`;
  await login(page);
  await openNumberingPage(page);

  await expect(page.getByRole("heading", { name: "M-CG 单据号规则" })).toBeVisible();
  await expect(page.getByText("purchase-inbound").first()).toBeVisible();
  await page.getByRole("button", { name: "新增", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "新增单据号规则" });
  await editor.getByLabel("规则编码").fill(ruleCode);
  await editor.getByLabel("规则名称").fill("E2E 编码规则");
  await editor.getByLabel("单据类型").selectOption("purchase_inbound");
  await editor.getByLabel("流水位数").fill("6");
  await editor.getByLabel("编码模板").fill("{OWNER}-E2E-{YYYY}{MM}{DD}-{SEQ}");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("规则已新增");
  await expect(page.getByText(ruleCode).first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "rule-created.png"), fullPage: false });

  const createdRow = page.getByRole("row").filter({ hasText: ruleCode }).first();
  await createdRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "预览", exact: true }).click();
  const preview = page.getByRole("dialog", { name: "规则预览" });
  await expect(preview).toBeVisible();
  await expect(preview.getByText(/PY001-E2E-/)).toBeVisible();
  await preview.getByRole("button", { name: "关闭", exact: true }).click();

  await page.getByRole("button", { name: "编辑", exact: true }).click();
  const editDialog = page.getByRole("dialog", { name: "编辑单据号规则" });
  await editDialog.getByLabel("规则名称").fill("E2E 编码规则已更新");
  await editDialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("规则已更新");

  await createdRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "启停", exact: true }).click();
  const disableDialog = page.getByRole("dialog", { name: "停用编码规则" });
  await disableDialog.getByRole("button", { name: "确认停用", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("已停用");
  await page.screenshot({ path: path.join(artifactsDir, "rule-disabled.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openNumberingPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M-CG 单据号规则/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "基础能力", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "M-CG 编码能力", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
}

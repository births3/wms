import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const roleArtifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/h1-role-permission");
const sessionArtifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/h1-session");

test("H1 角色权限管理使用真实 API 完成读写闭环", async ({ page }) => {
  fs.mkdirSync(roleArtifactsDir, { recursive: true });
  await login(page);
  await openRolePage(page);
  await expect(page.getByRole("heading", { name: "角色列表" })).toBeVisible();
  await expect(page.getByText("系统管理员").first()).toBeVisible();
  await expect(page.getByText("司机").first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "权限矩阵" })).toBeVisible();
  await expect(page.getByText("H1 角色权限维护").first()).toBeVisible();

  const roleName = `E2E 角色 ${Date.now()}`;
  const roleCode = `e2e_role_${Date.now()}`;
  await page.getByRole("button", { name: "新增", exact: true, description: "新增角色" }).click();
  const roleDialog = page.getByRole("dialog");
  await roleDialog.getByLabel("角色编码").fill(roleCode);
  await roleDialog.getByLabel("角色名称").fill(roleName);
  await roleDialog.getByLabel("数据范围").selectOption("owner");
  await roleDialog.getByRole("button", { name: "新增角色" }).click();
  await expect(page.getByText(`${roleName} 已新增`)).toBeVisible();

  await page.locator("tr").filter({ hasText: roleName }).getByRole("checkbox", { name: "选择此行" }).check();
  const permission = page.getByLabel("权限 H1 角色权限维护");
  await permission.check();
  await page.getByRole("button", { name: "保存权限", exact: true }).click();
  await expect(page.getByText(`${roleName} 权限已原子保存`)).toBeVisible();

  const userName = `E2E 用户 ${Date.now()}`;
  const username = `e2e_user_${Date.now()}`;
  await page.getByRole("button", { name: "新增", exact: true }).last().click();
  const userDialog = page.getByRole("dialog", { name: "新增用户" });
  await userDialog.getByRole("button", { name: "新增用户", exact: true }).click();
  await expect(page.locator('[role="alert"]')).toContainText("请填写账号、姓名、有效手机号");
  await userDialog.getByLabel("登录账号").fill(username);
  await userDialog.getByLabel("姓名").fill(userName);
  await userDialog.getByLabel("手机号").fill("13800000001");
  await userDialog.getByLabel("初始密码").fill("CorrectHorse1!");
  await userDialog.getByRole("checkbox", { name: "绑定角色 系统管理员" }).check();
  const createUserResponse = page.waitForResponse((response) => response.url().includes("/api/v1/auth/users") && response.request().method() === "POST");
  await userDialog.getByRole("button", { name: "新增用户", exact: true }).click();
  const createUserResponseValue = await createUserResponse;
  expect(createUserResponseValue.status()).toBe(200);
  const createdUser = (await createUserResponseValue.json()) as { display_name: string; role_ids: string[] };
  expect(createdUser.display_name).toBe(userName);
  expect(createdUser.role_ids).toHaveLength(1);
  await expect(page.getByText(`${userName} 已新增`)).toBeVisible();

  await page.getByRole("button", { name: "授权", exact: true }).click();
  const batchDialog = page.getByRole("dialog", { name: "批量授权" });
  await expect(batchDialog).toBeVisible();
  await expect(batchDialog.getByText("用户（已选 0）")).toBeVisible();
  await expect(batchDialog.getByText(userName, { exact: true })).toBeVisible();
  await expect(batchDialog.locator("tr", { hasText: userName }).getByText("1 个", { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(roleArtifactsDir, "user-create-role-binding.png"), fullPage: false });
  await page.getByRole("button", { name: "取消", exact: true }).last().click();

  await page.getByRole("button", { name: "删除", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "删除角色" })).toBeVisible();
  await page.getByRole("button", { name: "确认删除" }).click();
  await expect(page.getByText(`${roleName} 已删除`)).toBeVisible();
  await page.screenshot({ path: path.join(roleArtifactsDir, "role-permission.png"), fullPage: false });
});

test("H1 登录会话使用真实 API 完成设备失效和截图验证", async ({ page }) => {
  fs.mkdirSync(sessionArtifactsDir, { recursive: true });
  await login(page);
  const secondLogin = await page.request.post(`${process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19085"}/api/v1/auth/login`, {
    data: { owner_code: "PY_OWNER", username: "admin", password: "CorrectHorse1!" },
  });
  expect(secondLogin.ok()).toBeTruthy();
  const secondLoginBody = (await secondLogin.json()) as { access_token?: string };
  expect(secondLoginBody.access_token).toBeTruthy();
  await openSessionPage(page);
  await expect(page.getByRole("heading", { name: "H1 登录会话" })).toBeVisible();
  const activeCaption = page.getByText(/当前 \d+ 个活跃会话/);
  await expect(activeCaption).toBeVisible();
  const beforeCount = Number((await activeCaption.textContent())?.match(/\d+/)?.[0] ?? 0);
  expect(beforeCount).toBeGreaterThanOrEqual(2);

  await page.locator("tbody tr").first().getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "失效设备", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "确认失效此设备" })).toBeVisible();
  await page.getByRole("dialog").getByRole("button", { name: "确认失效" }).click();
  await expect.poll(async () => Number((await activeCaption.textContent())?.match(/\d+/)?.[0] ?? 0)).toBe(beforeCount - 1);

  const apiURL = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19085";
  const logout = await page.request.post(`${apiURL}/api/v1/auth/logout`, {
    headers: { authorization: `Bearer ${secondLoginBody.access_token}` },
  });
  expect(logout.ok()).toBeTruthy();
  const rejected = await page.request.get(`${apiURL}/api/v1/auth/me`, {
    headers: { authorization: `Bearer ${secondLoginBody.access_token}` },
  });
  expect(rejected.status()).toBe(401);
  await page.screenshot({ path: path.join(sessionArtifactsDir, "session-management.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openRolePage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /H1 角色权限/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "基础能力", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "H1 权限租户", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
}

async function openSessionPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /H1 登录会话/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "基础能力", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "H1 权限租户", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
}

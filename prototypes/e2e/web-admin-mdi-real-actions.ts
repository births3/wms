import { expect, type Page } from "@playwright/test";

export async function approveQualityLiaison(page: Page, liaisonId: string) {
  return page.evaluate(async (id) => {
    const session = JSON.parse(
      localStorage.getItem("wms.web-admin.auth-session") ?? "null",
    ) as { accessToken?: string } | null;
    const response = await fetch(`/api/v1/quality-liaisons/${id}/approval-callback`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${session?.accessToken ?? ""}`,
        "Content-Type": "application/json",
        "Idempotency-Key": `mdi-e2e-liaison-${id}`,
      },
      body: JSON.stringify({
        conclusion: "approved",
        opinion: "真实 E2E 批准隔离同商品同批号库存",
        external_approval_id: `MDI-E2E-${id}`,
      }),
    });
    return {
      status: response.status,
      body: await response.json() as { status?: string; code?: string; message?: string },
    };
  }, liaisonId);
}

export async function closeEntryDialog(page: Page) {
  await page.getByRole("dialog").getByRole("button", { name: "取消" }).click();
}

export async function login(page: Page, username: string) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill(username);
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

export async function switchUser(page: Page, username: string) {
  await page.evaluate(() => localStorage.clear());
  await login(page, username);
}

export async function openMenu(page: Page, section: string, group: string, item: RegExp) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: item });
  if (!(await target.isVisible())) {
    const sectionButton = navigation.getByRole("button", { name: section, exact: true });
    if ((await sectionButton.getAttribute("aria-expanded")) !== "true") await sectionButton.click();
    const groupButton = navigation.getByRole("button", { name: group, exact: true });
    if ((await groupButton.getAttribute("aria-expanded")) !== "true") await groupButton.click();
  }
  await target.click();
}

export async function dispatchWindowPointer(
  page: Page,
  type: "pointermove" | "pointerup",
  clientX: number,
  clientY: number,
) {
  await page.evaluate(
    ({ type, clientX, clientY }) => window.dispatchEvent(new PointerEvent(type, {
      pointerId: 1,
      isPrimary: true,
      button: 0,
      clientX,
      clientY,
    })),
    { type, clientX, clientY },
  );
}

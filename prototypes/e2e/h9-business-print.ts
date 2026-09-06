import { expect, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

export async function completeH9BusinessPrint(
  page: Page,
  options: {
    actionName: string;
    dialogName: string;
    businessModule: string;
    templateType: string;
    expectedField: string;
    expectedValue: string;
    screenshotPath: string;
  },
) {
  const previewResponsePromise = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-templates/preview") &&
      response.request().method() === "POST",
  );
  await page.getByRole("button", { name: options.actionName, exact: true }).click();
  const previewResponse = await previewResponsePromise;
  expect(previewResponse.ok(), await previewResponse.text()).toBeTruthy();
  const previewRequest = JSON.parse(previewResponse.request().postData() ?? "{}") as {
    template_type_code?: string;
    data?: Record<string, unknown>;
  };
  expect(previewRequest.template_type_code).toBe(options.templateType);
  expect(previewRequest.data?.[options.expectedField]).toBe(options.expectedValue);

  const dialog = page.getByRole("dialog", { name: options.dialogName });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText(options.expectedValue)).toBeVisible();
  fs.mkdirSync(path.dirname(options.screenshotPath), { recursive: true });
  await dialog.screenshot({ path: options.screenshotPath });
  await dialog.getByRole("button", { name: "打印", exact: true }).click();
  await expect(dialog.getByText("确认打印结果")).toBeVisible();

  const printResponsePromise = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-templates/print") &&
      response.request().method() === "POST",
  );
  await dialog.getByRole("button", { name: "已完成打印", exact: true }).click();
  const printResponse = await printResponsePromise;
  expect(printResponse.ok(), await printResponse.text()).toBeTruthy();
  const printRequest = JSON.parse(printResponse.request().postData() ?? "{}") as {
    business_module?: string;
    business_document_type?: string;
    data?: Record<string, unknown>;
    status?: string;
  };
  expect(printRequest.business_module).toBe(options.businessModule);
  expect(printRequest.business_document_type).toBe(options.templateType);
  expect(printRequest.data?.[options.expectedField]).toBe(options.expectedValue);
  expect(printRequest.status).toBe("printed");
  await expect(printResponse.json()).resolves.toMatchObject({ status: "printed" });
}

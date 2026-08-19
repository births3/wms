import { chromium, login, openPage, CHROME } from "../governance/capture_web_admin_interaction_helpers.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SHOT_DIR = path.resolve(__dirname, "../../artifacts/screenshot-portal/unified-templates");

async function run() {
  fs.mkdirSync(SHOT_DIR, { recursive: true });
  console.log("🚀 Testing ListPageTemplate & MasterDetailPageTemplate in real browser...");

  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"],
  });

  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  try {
    await login(page);

    // 1. 测试上架策略页面 (ListPageTemplate)
    console.log("Testing M2 上架策略页面 (ListPageTemplate)...");
    await openPage(page, "入库业务", /M2 上架策略|上架策略/);
    await page.waitForTimeout(600);

    // 在 QueryPanel 关键词输入框输入 "方案" 并点击查询
    const queryInput = page.getByPlaceholder("编码 / 方案名称").first();
    if (await queryInput.isVisible()) {
      await queryInput.fill("通用");
      await page.getByRole("button", { name: "查询", exact: true }).click();
      await page.waitForTimeout(400);

      // 验证 DataGrid 顶部是否出现统一的【已应用条件】标签
      const filterChips = page.locator("div[aria-label='已启用筛选']");
      const chipsVisible = await filterChips.isVisible();
      console.log(`  • 已应用条件标签栏是否成功唤起: ${chipsVisible ? "✅ YES" : "❌ NO"}`);

      const chipText = chipsVisible ? await filterChips.innerText() : "";
      console.log(`  • 标签内容: ${chipText.replace(/\n/g, " ")}`);

      // 截图保存
      await page.screenshot({
        path: path.join(SHOT_DIR, "m2_putaway_strategy_unified_template.png"),
        fullPage: false,
      });
    }

    // 2. 测试告警定义页面 (ListPageTemplate)
    console.log("Testing H-AL 告警定义页面 (ListPageTemplate)...");
    await openPage(page, "基础能力", /H-AL 告警定义|告警定义/);
    await page.waitForTimeout(600);
    await page.screenshot({
      path: path.join(SHOT_DIR, "hal_alert_definition_unified_template.png"),
      fullPage: false,
    });

    // 3. 测试角色权限页面 (MasterDetailPageTemplate)
    console.log("Testing H1 角色权限页面 (MasterDetailPageTemplate)...");
    await openPage(page, "基础能力", /H1 角色权限|角色权限/);
    await page.waitForTimeout(600);
    await page.screenshot({
      path: path.join(SHOT_DIR, "h1_role_permission_unified_template.png"),
      fullPage: false,
    });

    console.log("🎉 All PageTemplates real browser tests completed!");
  } finally {
    await browser.close();
  }
}

run();

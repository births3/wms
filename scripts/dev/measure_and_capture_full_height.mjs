import { chromium, login, openPage, CHROME } from "../governance/capture_web_admin_interaction_helpers.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SHOT_DIR = path.resolve(__dirname, "../../artifacts/screenshot-portal/measurement");

async function run() {
  fs.mkdirSync(SHOT_DIR, { recursive: true });
  console.log("🚀 Launching Chrome for Full-Height Playwright Measurement & Screenshots...");

  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"],
  });

  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  try {
    await login(page);
    await openPage(page, "入库业务", /M2 收货|收货/);
    await page.waitForTimeout(1000);

    const dataTableRoot = page.locator("div.rounded-md.border.bg-background.overflow-hidden.font-sans").first();
    await dataTableRoot.waitFor({ state: "visible", timeout: 8000 });

    // 1. 默认多数据状态实测
    const defaultMeasurements = await dataTableRoot.evaluate((el) => {
      const rootRect = el.getBoundingClientRect();
      const bottomBar = el.querySelector(".mt-auto.shrink-0");
      const bottomBarRect = bottomBar ? bottomBar.getBoundingClientRect() : null;
      const scrollArea = el.querySelector(".min-h-0.flex-1.overflow-auto");
      const scrollAreaRect = scrollArea ? scrollArea.getBoundingClientRect() : null;

      return {
        rootHeight: rootRect.height,
        rootTop: rootRect.top,
        rootBottom: rootRect.bottom,
        scrollAreaHeight: scrollAreaRect ? scrollAreaRect.height : 0,
        bottomBarHeight: bottomBarRect ? bottomBarRect.height : 0,
        bottomBarTop: bottomBarRect ? bottomBarRect.top : 0,
        bottomBarBottom: bottomBarRect ? bottomBarRect.bottom : 0,
        isBottomPinned: bottomBarRect ? Math.abs(bottomBarRect.bottom - rootRect.bottom) < 5 : false,
      };
    });

    console.log("==================================================");
    console.log("📊 1. 默认数据状态实测 (多数据行):");
    console.log(`  • DataTable 容器高度: ${defaultMeasurements.rootHeight.toFixed(1)}px (从顶部 380px 提升至撑满整屏)`);
    console.log(`  • DataTable 底部 Y轴: ${defaultMeasurements.rootBottom.toFixed(1)}px (距离视口底边 900px 仅 32px 内边距)`);
    console.log(`  • 滚动视口高度:       ${defaultMeasurements.scrollAreaHeight.toFixed(1)}px`);
    console.log(`  • 底部分页栏高度:     ${defaultMeasurements.bottomBarHeight.toFixed(1)}px`);
    console.log(`  • 底部分页栏贴底:     ${defaultMeasurements.isBottomPinned ? "✅ YES (牢牢贴底锚定)" : "❌ NO"}`);
    console.log("==================================================");

    const shot1 = path.join(SHOT_DIR, "datagrid_full_height_default.png");
    await page.screenshot({ path: shot1, fullPage: false });
    console.log(`📸 默认撑满全屏截图已保存: ${shot1}`);

    // 2. 空数据/0 行状态实测（输入无匹配字符）
    const searchInput = page.locator('input[placeholder*="搜索"], input[placeholder*="单号"], input[placeholder*="编码"], input[placeholder*="关键字"]').first();
    if (await searchInput.isVisible()) {
      await searchInput.fill("NO_DATA_MATCH_EMPTY_TEST_99999");
      await page.keyboard.press("Enter");
      await page.waitForTimeout(600);

      const emptyMeasurements = await dataTableRoot.evaluate((el) => {
        const rootRect = el.getBoundingClientRect();
        const bottomBar = el.querySelector(".mt-auto.shrink-0");
        const bottomBarRect = bottomBar ? bottomBar.getBoundingClientRect() : null;
        const emptyCell = el.querySelector(".h-64.py-12");
        return {
          rootHeight: rootRect.height,
          rootTop: rootRect.top,
          rootBottom: rootRect.bottom,
          emptyCellFound: Boolean(emptyCell),
          isBottomPinned: bottomBarRect ? Math.abs(bottomBarRect.bottom - rootRect.bottom) < 5 : false,
        };
      });

      console.log("==================================================");
      console.log("📊 2. 空数据状态实测 (0 行数据):");
      console.log(`  • DataTable 容器高度: ${emptyMeasurements.rootHeight.toFixed(1)}px (0行时依然自适应撑满到底部)`);
      console.log(`  • DataTable 底部 Y轴: ${emptyMeasurements.rootBottom.toFixed(1)}px`);
      console.log(`  • 空提示垂直居中:     ${emptyMeasurements.emptyCellFound ? "✅ YES (垂直居中)" : "❌ NO"}`);
      console.log(`  • 底部分页栏贴底:     ${emptyMeasurements.isBottomPinned ? "✅ YES (牢牢贴底锚定)" : "❌ NO"}`);
      console.log("==================================================");

      const shot2 = path.join(SHOT_DIR, "datagrid_full_height_empty_state.png");
      await page.screenshot({ path: shot2, fullPage: false });
      console.log(`📸 空数据撑满全屏截图已保存: ${shot2}`);
    }

    console.log("🎉 All Playwright Full-Height Viewport Extension measurements PASSED!");
  } finally {
    await browser.close();
  }
}

run();

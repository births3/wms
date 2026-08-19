import { chromium, login, openPage, CHROME } from "./capture_web_admin_interaction_helpers.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SHOT_DIR = path.resolve(__dirname, "../../artifacts/screenshot-portal/layout-consistency");

const TEST_TARGETS = [
  { group: "入库业务", namePattern: /M2 收货|收货/, name: "M2 收货管理" },
  { group: "入库业务", namePattern: /M2 上架策略|上架策略/, name: "M2 上架策略" },
  { group: "入库业务", namePattern: /药检平台/, name: "M-DI 药检平台" },
  { group: "出库业务", namePattern: /M4 出库订单|出库订单/, name: "M4 出库订单" },
  { group: "出库业务", namePattern: /波次规划/, name: "M4 波次规划" },
  { group: "库内业务", namePattern: /M3 批号|批号/, name: "M3 批号管理" },
  { group: "库内业务", namePattern: /M3 状态规则|状态规则/, name: "M3 状态规则" },
  { group: "库内业务", namePattern: /任务类型/, name: "M-TE 任务类型配置" },
  { group: "库内业务", namePattern: /任务组资格/, name: "M-TE 任务组资格" },
  { group: "库内业务", namePattern: /任务调度/, name: "M-TE 任务调度" },
  { group: "基础能力", namePattern: /告警看板/, name: "H-AL 告警看板" },
  { group: "基础能力", namePattern: /告警定义/, name: "H-AL 告警定义" },
  { group: "基础能力", namePattern: /升级规则/, name: "H-AL 升级规则" },
  { group: "基础能力", namePattern: /角色权限/, name: "H1 角色权限" },
  { group: "基础能力", namePattern: /API Key/, name: "H1 API Key 管理" },
];

async function run() {
  fs.mkdirSync(SHOT_DIR, { recursive: true });
  console.log("🚀 Starting Playwright Full-Height Layout Consistency Batch Check across pages...");

  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"],
  });

  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const results = [];
  try {
    await login(page);

    for (const target of TEST_TARGETS) {
      try {
        await openPage(page, target.group, target.namePattern);
        await page.waitForTimeout(600);

        const dataTableRoot = page.locator("div[data-datatable-root='true']").first();
        if (!(await dataTableRoot.isVisible())) {
          results.push({ name: target.name, status: "SKIP (No DataTable visible)" });
          continue;
        }

        const measurements = await dataTableRoot.evaluate((el) => {
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
            isBottomPinned: bottomBarRect ? Math.abs(bottomBarRect.bottom - rootRect.bottom) < 5 : false,
          };
        });

        const isPassed = measurements.rootHeight >= 450 && measurements.isBottomPinned;
        const resultItem = {
          name: target.name,
          rootHeight: measurements.rootHeight.toFixed(1) + "px",
          bottomY: measurements.rootBottom.toFixed(1) + "px",
          scrollAreaHeight: measurements.scrollAreaHeight.toFixed(1) + "px",
          isBottomPinned: measurements.isBottomPinned,
          passed: isPassed,
        };
        results.push(resultItem);

        // 保存截图
        const safeName = target.name.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, "_");
        const shotPath = path.join(SHOT_DIR, `${safeName}.png`);
        await page.screenshot({ path: shotPath, fullPage: false });

        console.log(`  • [${isPassed ? "PASS" : "FAIL"}] ${target.name.padEnd(20)} | Height: ${resultItem.rootHeight} | Bottom: ${resultItem.bottomY} | Pinned: ${measurements.isBottomPinned}`);
      } catch (err) {
        console.warn(`  ⚠️ Error inspecting ${target.name}: ${err.message}`);
        results.push({ name: target.name, error: err.message, passed: false });
      }
    }

    console.log("\n==================================================");
    console.log("📊 Batch Layout Consistency Summary:");
    console.table(results);
    console.log("==================================================");

    const failed = results.filter((r) => r.passed === false);
    if (failed.length > 0) {
      throw new Error(`${failed.length} pages failed full-height layout consistency check!`);
    }

    console.log("🎉 All tested pages PASSED full-height layout consistency!");
  } finally {
    await browser.close();
  }
}

run();

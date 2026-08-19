import { chromium, login, openPage, CHROME } from "../governance/capture_web_admin_interaction_helpers.mjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SHOT_DIR = path.resolve(__dirname, "../../artifacts/screenshot-portal/inspect");

async function run() {
  fs.mkdirSync(SHOT_DIR, { recursive: true });
  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"],
  });

  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  try {
    await login(page);
    console.log("Opening M2 上架管理...");
    await openPage(page, "入库业务", /M2 上架管理|上架管理/);
    await page.waitForTimeout(1000);

    const shotPath = path.join(SHOT_DIR, "m2_putaway_management_current.png");
    await page.screenshot({ path: shotPath, fullPage: false });
    console.log(`Saved screenshot to ${shotPath}`);
  } finally {
    await browser.close();
  }
}

run();

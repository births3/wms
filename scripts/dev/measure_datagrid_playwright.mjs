import { chromium, login, openPage, CHROME } from "../governance/capture_web_admin_interaction_helpers.mjs";
import fs from "node:fs";

async function run() {
  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"],
  });
  const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();

  try {
    await login(page);
    await openPage(page, "入库业务", /M2 收货|收货/);
    await page.waitForTimeout(1000);

    const allSections = await page.evaluate(() => {
      return Array.from(document.querySelectorAll("section")).map((s) => ({
        className: s.className,
        height: s.getBoundingClientRect().height,
        childrenCount: s.children.length,
      }));
    });

    console.log("All sections on page:", allSections);
  } finally {
    await browser.close();
  }
}

run();

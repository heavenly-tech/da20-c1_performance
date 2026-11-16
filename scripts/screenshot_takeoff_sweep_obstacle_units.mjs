/**
 * Takeoff sweep obstacle stepper + table with ft/m unit toggle (ES/EN).
 *
 *   cargo build --release -p da20-server
 *   npx --yes playwright install chromium
 *   OUT_DIR=/opt/cursor/artifacts node scripts/screenshot_takeoff_sweep_obstacle_units.mjs
 */
import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18081";
const outDir = process.env.OUT_DIR ?? "/opt/cursor/artifacts";

async function clearDa20Storage(page) {
  await page.evaluate(() => {
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const k = localStorage.key(i);
      if (k && k.startsWith("da20_")) localStorage.removeItem(k);
    }
  });
}

async function setLocale(page, locale) {
  await page.evaluate((loc) => {
    localStorage.setItem("da20_locale_v1", loc);
  }, locale);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  await page.waitForTimeout(600);
}

async function setObstacleUnit(page, unit) {
  const btn =
    unit === "ft"
      ? page.locator(".unit-toggle button").filter({ hasText: /^ft$/ }).first()
      : page.locator(".unit-toggle button").filter({ hasText: /^m$/ }).first();
  await btn.click();
  await page.waitForTimeout(400);
}

async function enableObstacleSweepAndRun(page) {
  const sweep = page.locator(".sweep-card");
  await sweep.waitFor({ state: "visible", timeout: 30_000 });
  const obsRow = sweep.locator(".sweep-var").filter({
    hasText: /Obstáculo|Obstacle/i,
  });
  await obsRow.locator('input[type="checkbox"]').check();
  await sweep.getByRole("button", { name: /Calcular tabla|Build table/i }).click();
  await page.waitForSelector(".sweep-table", { timeout: 30_000 });
  await page.waitForTimeout(500);
}

async function main() {
  await mkdir(outDir, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1400, height: 1200 } });

  await page.goto(`${baseUrl}/#takeoff`, { waitUntil: "networkidle", timeout: 120_000 });
  await clearDa20Storage(page);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  await page.waitForSelector(".sweep-card", { timeout: 60_000 });

  for (const [locale, locKey] of [
    ["es", "Es"],
    ["en", "En"],
  ]) {
    await setLocale(page, locKey);
    for (const unit of ["m", "ft"]) {
      await setObstacleUnit(page, unit);
      await enableObstacleSweepAndRun(page);
      const card = page.locator(".sweep-card");
      const file = path.join(outDir, `takeoff_sweep_obs_${locale}_${unit}.png`);
      await card.screenshot({ path: file });
      console.log("wrote", file);
    }
  }

  await browser.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
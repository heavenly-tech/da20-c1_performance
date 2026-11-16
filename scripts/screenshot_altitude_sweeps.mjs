import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18081";
const outDir = process.env.OUT_DIR ?? "docs/pr-climb-cruise-sweep-screenshots";
const themes = (process.env.THEMES ?? "light,dark").split(",").filter(Boolean);
const locales = (process.env.LOCALES ?? "Es,En").split(",").filter(Boolean);

const themeStorage = { light: '"Light"', dark: '"Dark"' };
const localeStorage = { Es: '"Es"', En: '"En"' };

async function clearDa20Storage(page) {
  await page.evaluate(() => {
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const k = localStorage.key(i);
      if (k && k.startsWith("da20_")) localStorage.removeItem(k);
    }
  });
}

async function setup(page, theme, locale) {
  await clearDa20Storage(page);
  await page.evaluate(
    (s) => {
      localStorage.setItem("da20_theme_v1", s.theme);
      localStorage.setItem("da20_locale_v1", s.locale);
    },
    { theme: themeStorage[theme], locale: localeStorage[locale] },
  );
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  const themeSelect = page.locator("label.theme-picker select.palette-select");
  await themeSelect.waitFor({ state: "visible", timeout: 30_000 });
  await themeSelect.selectOption(theme);
  await page.waitForTimeout(600);
}

async function runSweep(page, tabLabel) {
  await page.getByRole("button", { name: tabLabel }).click();
  await page.waitForTimeout(800);
  const sweep = page.locator(".sweep-card").first();
  await sweep.waitFor({ state: "visible", timeout: 30_000 });
  await sweep.getByRole("button", { name: /Calcular tabla|Build table/i }).click();
  await page.waitForSelector(".sweep-table tbody tr", { timeout: 30_000 });
  await page.waitForTimeout(400);
}

async function main() {
  await mkdir(outDir, { recursive: true });
  const browser = await chromium.launch();
  const wideVp = { width: 1400, height: 1000 };
  const narrowVp = { width: 390, height: 900 };
  const page = await browser.newPage({ viewport: wideVp });
  await page.goto(baseUrl, { waitUntil: "networkidle", timeout: 120_000 });

  const climbTab = { Es: "Ascenso", En: "Climb" };
  const cruiseTab = { Es: "Crucero y potencia", En: "Cruise & power" };

  for (const theme of themes) {
    for (const loc of locales) {
      await setup(page, theme, loc);
      await runSweep(page, climbTab[loc]);
      await page.screenshot({
        path: path.join(outDir, `climb_sweep_${loc.toLowerCase()}_${theme}.png`),
        fullPage: true,
      });
      await page.setViewportSize(narrowVp);
      await page.waitForTimeout(400);
      await page.screenshot({
        path: path.join(outDir, `climb_sweep_${loc.toLowerCase()}_${theme}_390px.png`),
        fullPage: true,
      });
      await page.setViewportSize(wideVp);
      await runSweep(page, cruiseTab[loc]);
      await page.screenshot({
        path: path.join(outDir, `cruise_sensenich_sweep_${loc.toLowerCase()}_${theme}.png`),
        fullPage: true,
      });
      await page.getByRole("button", { name: cruiseTab[loc] }).click();
      await page.getByRole("button", { name: /MT 175/i }).click();
      await page.waitForTimeout(800);
      await runSweep(page, cruiseTab[loc]);
      await page.screenshot({
        path: path.join(outDir, `cruise_mt_sweep_${loc.toLowerCase()}_${theme}.png`),
        fullPage: true,
      });
    }
  }
  await browser.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

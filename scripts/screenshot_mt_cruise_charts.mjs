import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18084";
const outDir = process.env.OUT_DIR ?? "/opt/cursor/artifacts";

const themeStorage = {
  light: '"Light"',
  dark: '"Dark"',
  night: '"Night"',
};

async function applyTheme(page, theme) {
  await page.evaluate((s) => localStorage.setItem("da20_theme_v1", s), themeStorage[theme]);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  const select = page.locator("label.theme-picker select.palette-select");
  await select.waitFor({ state: "visible", timeout: 15_000 });
  await select.selectOption(theme);
  await page.waitForTimeout(400);
}

async function openMtCruise(page) {
  await page.goto(`${baseUrl}/#/cruise`, { waitUntil: "networkidle", timeout: 120_000 });
  await page.getByRole("button", { name: "MT 175 R 150-2Ca", exact: true }).click();
  await page.waitForSelector(".mt-fig-svg", { timeout: 30_000 });
  await page.waitForTimeout(600);
}

async function setPctBhp(page, pct) {
  const input = page.locator("label").filter({ hasText: "% BHP" }).locator("input");
  if ((await input.count()) === 0) {
    const alt = page.locator(".f input[type='text']").filter({ has: page.locator("xpath=..//label[contains(., 'BHP')]") });
    const fields = page.locator(".f").filter({ hasText: "BHP" }).locator("input");
    await fields.first().click();
    await fields.first().fill(String(pct));
    await fields.first().press("Tab");
  } else {
    await input.first().click();
    await input.first().fill(String(pct));
    await input.first().press("Tab");
  }
  await page.waitForTimeout(800);
}

async function setPa(page, pa) {
  const paField = page.locator(".f").filter({ hasText: "PA" }).filter({ hasText: "crucero" }).locator("input").first();
  if ((await paField.count()) === 0) {
    const fallback = page.getByLabel(/PA.*crucero/i);
    if ((await fallback.count()) > 0) {
      await fallback.fill(String(pa));
      await fallback.press("Tab");
    }
  } else {
    await paField.click();
    await paField.fill(String(pa));
    await paField.press("Tab");
  }
  await page.waitForTimeout(900);
}

async function shotCharts(page, file) {
  const titles = page.locator("p.lab.mono").filter({ hasText: "Fig 2" });
  const fig3Titles = page.locator("p.lab.mono").filter({ hasText: "Fig 3" });
  const box = page.locator(".mt-fig-svg").first();
  await page.locator(".mt-fig-svg").nth(1).scrollIntoViewIfNeeded();
  const titleBb = (await titles.count()) > 0 ? await titles.first().boundingBox() : null;
  const bb = await box.boundingBox();
  const second = page.locator(".mt-fig-svg").nth(1);
  const bb2 = await second.boundingBox();
  if (!bb || !bb2) throw new Error("chart bbox missing");
  const y = Math.max(0, (titleBb?.y ?? bb.y) - 8);
  const fig3TitleY = (await fig3Titles.count()) > 0 ? (await fig3Titles.first().boundingBox())?.y : null;
  const bottom = Math.max(bb2.y + bb2.height, fig3TitleY ?? 0);
  const h = bottom - y + 36;
  await page.screenshot({
    path: file,
    clip: { x: 0, y, width: 1400, height: Math.min(h, 1500) },
  });
  console.log(file);
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 2200 } });

for (const theme of ["light", "dark", "night"]) {
  await page.goto(`${baseUrl}/#/cruise`, { waitUntil: "networkidle", timeout: 120_000 });
  await applyTheme(page, theme);
  await openMtCruise(page);
  await shotCharts(page, path.join(outDir, `mt175_cruise_charts_${theme}.png`));
}

await applyTheme(page, "light");
await openMtCruise(page);
await setPctBhp(page, 60);
await shotCharts(page, path.join(outDir, "mt175_cruise_pct60_light.png"));

await openMtCruise(page);
await setPa(page, 12000);
await setPctBhp(page, 75);
await shotCharts(page, path.join(outDir, "mt175_cruise_fuera_carta_light.png"));

await browser.close();

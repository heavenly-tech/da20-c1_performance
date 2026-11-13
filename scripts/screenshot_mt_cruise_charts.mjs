import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18082";
const outDir = process.env.OUT_DIR ?? "/opt/cursor/artifacts";

const themeStorage = {
  light: '"Light"',
  dark: '"Dark"',
  night: '"Night"',
};

async function applyTheme(page, theme) {
  const storage = themeStorage[theme] ?? '"Light"';
  await page.evaluate((s) => {
    localStorage.setItem("da20_theme_v1", s);
  }, storage);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  const select = page.locator("label.theme-picker select.palette-select");
  await select.waitFor({ state: "visible", timeout: 15_000 });
  await select.selectOption(theme);
  await page.waitForTimeout(400);
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 1100 } });
await page.goto(`${baseUrl}/#/cruise`, { waitUntil: "networkidle", timeout: 120_000 });
await page.waitForTimeout(800);

const mtBtn = page.getByRole("button", { name: "MT 175 R 150-2Ca", exact: true });
await mtBtn.click();
await page.waitForSelector(".mt-fig-svg", { timeout: 30_000 });
await page.waitForTimeout(800);

for (const theme of ["light", "dark", "night"]) {
  await applyTheme(page, theme);
  await page.goto(`${baseUrl}/#/cruise`, { waitUntil: "networkidle", timeout: 120_000 });
  await page.waitForTimeout(400);
  await page.getByRole("button", { name: "MT 175 R 150-2Ca", exact: true }).click();
  await page.waitForSelector(".mt-fig-svg", { timeout: 30_000 });
  await page.waitForTimeout(600);
  const file = path.join(outDir, `mt175_cruise_fig2_fig3_${theme}.png`);
  await page.locator(".mt-fig-svg").first().waitFor({ state: "visible", timeout: 60_000 });
  await page.locator(".mt-fig-svg").first().scrollIntoViewIfNeeded();
  await page.screenshot({ path: file, fullPage: true });
  console.log(file);
}

await browser.close();

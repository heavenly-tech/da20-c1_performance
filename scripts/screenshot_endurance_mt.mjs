import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18081";
const outDir = process.env.OUT_DIR ?? "/opt/cursor/artifacts";
const label = process.env.LABEL ?? "after";
const theme = process.env.THEME ?? "light";

async function clearDa20Storage(page) {
  await page.evaluate(() => {
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const k = localStorage.key(i);
      if (k && k.startsWith("da20_")) localStorage.removeItem(k);
    }
  });
}

async function applyEsLight(page) {
  await clearDa20Storage(page);
  await page.evaluate(() => {
    localStorage.setItem("da20_theme_v1", '"Light"');
    localStorage.setItem("da20_locale_v1", "Es");
  });
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  await page.locator("label.theme-picker select.palette-select").selectOption(theme);
  await page.waitForFunction(
    (t) => document.documentElement.getAttribute("data-theme") === t,
    theme,
    { timeout: 15_000 },
  );
  await page.waitForTimeout(500);
}

async function clickTab(page, name) {
  await page.getByRole("button", { name, exact: false }).first().click();
  await page.waitForTimeout(400);
}

async function setNumByLabel(page, labelRe, value) {
  const row = page.locator(".f").filter({ has: page.locator("label", { hasText: labelRe }) });
  const input = row.locator("input").first();
  await input.click();
  await input.fill(String(value));
  await input.press("Tab");
  await page.waitForTimeout(350);
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
await page.goto(`${baseUrl}/?v=${Date.now()}`, {
  waitUntil: "networkidle",
  timeout: 120_000,
});
await applyEsLight(page);

await clickTab(page, /Autonomía/i);
const mt = page.getByRole("button", { name: /MT 175/i });
await mt.first().click();
await page.waitForTimeout(300);
const bhp = page.getByRole("button", { name: "Fijar %BHP", exact: true });
if ((await bhp.count()) > 0) await bhp.click();
await setNumByLabel(page, /PA crucero|Crucero PA/i, "8000");
await setNumByLabel(page, /%BHP|BHP/i, "55");
await page.waitForTimeout(600);

const inChart = path.join(outDir, `${label}_light_autonomia_mt_8000_55.png`);
await page.locator(".chart-card").screenshot({ path: inChart });
console.log("wrote", inChart);

await clickTab(page, /Autonomía/i);
const bhp2 = page.getByRole("button", { name: "Fijar %BHP", exact: true });
if ((await bhp2.count()) > 0) await bhp2.click();
await setNumByLabel(page, /PA crucero|Crucero PA/i, "12000");
await setNumByLabel(page, /%BHP|BHP/i, "65");
await page.waitForTimeout(600);

const ooc = path.join(outDir, `${label}_light_autonomia_mt_12000_65_fig4_ooc.png`);
await page.locator(".chart-card").screenshot({ path: ooc });
console.log("wrote", ooc);

await browser.close();

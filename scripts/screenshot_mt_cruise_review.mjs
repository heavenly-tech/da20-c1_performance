import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18086";
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
  const fields = page.locator(".f").filter({ hasText: "BHP" }).locator("input");
  await fields.first().click();
  await fields.first().fill(String(pct));
  await fields.first().press("Tab");
  await page.waitForTimeout(700);
}

async function setPa(page, pa) {
  const paField = page
    .locator(".f")
    .filter({ hasText: "PA" })
    .filter({ hasText: "crucero" })
    .locator("input")
    .first();
  await paField.click();
  await paField.fill(String(pa));
  await paField.press("Tab");
  await page.waitForTimeout(700);
}

async function setOatManual(page, oat) {
  const manual = page.getByRole("button", { name: /manual|escrito|written/i }).first();
  if ((await manual.count()) > 0) await manual.click();
  const oatField = page.locator(".f").filter({ hasText: /OAT/i }).locator("input[type='text']").last();
  await oatField.waitFor({ state: "visible", timeout: 10_000 });
  await oatField.click();
  await oatField.fill(String(oat));
  await oatField.press("Tab");
  await page.waitForTimeout(700);
}

async function setPaOat(page, pa, oat) {
  await setPa(page, pa);
  if (oat !== null) await setOatManual(page, oat);
}

async function measureReviewClip(page) {
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(300);
  const mtProp = page.getByRole("button", { name: "MT 175 R 150-2Ca", exact: true });
  const fig2Title = page.locator("p.lab.mono").filter({ hasText: "Fig 2" }).first();
  const fig2 = page.locator(".mt-fig-svg").first();
  const fig3 = page.locator(".mt-fig-svg").nth(1);
  const mtBb = await mtProp.boundingBox();
  const t2bb = await fig2Title.boundingBox();
  const bb2 = await fig2.boundingBox();
  const bb3 = await fig3.boundingBox();
  if (!t2bb || !bb2 || !bb3) throw new Error("review clip bbox missing");
  const y = Math.max(0, Math.min(mtBb?.y ?? t2bb.y, t2bb.y) - 12);
  const bottom = bb3.y + bb3.height + 28;
  const height = bottom - y;
  if (bb2.y < y - 4) {
    throw new Error(`Fig 2 clipped: svgTop=${bb2.y} clipY=${y}`);
  }
  return { y, height };
}

async function shotCharts(page, file) {
  let { y, height } = await measureReviewClip(page);
  let vp = page.viewportSize() ?? { width: 1400, height: 2200 };
  if (height > vp.height - 8) {
    const newH = Math.min(Math.ceil(height + y + 40), 3000);
    await page.setViewportSize({ width: vp.width, height: newH });
    await page.waitForTimeout(200);
    ({ y, height } = await measureReviewClip(page));
    vp = page.viewportSize() ?? { width: 1400, height: newH };
  }
  const clipH = Math.min(height, vp.height - y);
  if (clipH < height - 2) {
    throw new Error(`clip height ${clipH} shorter than content ${height} for ${file}`);
  }
  await page.screenshot({
    path: file,
    clip: { x: 0, y, width: 1400, height: clipH },
  });
  console.log(`${file} (clip y=${y.toFixed(0)} h=${clipH.toFixed(0)})`);
}

/** Drag Fig 2 vertically ~8 px above current marker (stay on 65% above local 65% curve). */
async function dragFig2SlightlyAboveCurve(page) {
  const svg = page.locator(".mt-fig-svg").first();
  const box = await svg.boundingBox();
  if (!box) throw new Error("fig2 svg missing");
  const marker = svg.locator("circle.chart-marker");
  if ((await marker.count()) === 0) throw new Error("no fig2 marker to drag from");
  const m = await marker.boundingBox();
  if (!m) throw new Error("marker bbox");
  const sx = m.x + m.width / 2;
  const sy = m.y + m.height / 2;
  await page.mouse.move(sx, sy);
  await page.mouse.down();
  await page.mouse.move(sx, sy - 10, { steps: 4 });
  await page.mouse.up();
  await page.waitForTimeout(900);
}

const scenarios = [
  { id: "da7500_pct60", pct: 60, pa: 7500, oat: null },
  { id: "da7500_pct65", pct: 65, pa: 7500, oat: null },
  { id: "da11500_pct60", pct: 60, pa: 11500, oat: null },
  { id: "da11500_pct65", pct: 65, pa: 11500, oat: null },
];

function isaOatAtPa(pa) {
  return Math.round((15 - 0.0019812 * pa) * 10) / 10;
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();

for (const scenario of scenarios) {
  for (const theme of ["light", "dark", "night"]) {
    const page = await browser.newPage({ viewport: { width: 1400, height: 2400 } });
    await page.goto(`${baseUrl}/#/cruise`, { waitUntil: "networkidle", timeout: 120_000 });
    await applyTheme(page, theme);
    await openMtCruise(page);
    const oat = scenario.oat ?? isaOatAtPa(scenario.pa);
    await setPaOat(page, scenario.pa, oat);
    await setPctBhp(page, scenario.pct);
    await shotCharts(page, path.join(outDir, `mt175_review_${scenario.id}_${theme}.png`));
    await page.close();
  }
}

for (const theme of ["light", "dark", "night"]) {
  const page = await browser.newPage({ viewport: { width: 1400, height: 2400 } });
  await page.goto(`${baseUrl}/#/cruise`, { waitUntil: "networkidle", timeout: 120_000 });
  await applyTheme(page, theme);
  await openMtCruise(page);
  const pa = 10_000;
  await setPaOat(page, pa, isaOatAtPa(pa));
  await setPctBhp(page, 65);
  await dragFig2SlightlyAboveCurve(page);
  await shotCharts(
    page,
    path.join(outDir, `mt175_review_da10000_drag_above_65pct_${theme}.png`),
  );
  await page.close();
}

await browser.close();

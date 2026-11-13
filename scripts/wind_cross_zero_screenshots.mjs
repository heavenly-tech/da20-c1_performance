/**
 * Fig 5.4 wind-panel strip drag across HW↔TW zero (main vs fixed branch).
 *
 * Prereqs:
 *   trunk build --release (in crates/da20-ui) → repo dist/
 *   npx --yes playwright install chromium
 *
 *   BASE_URL=http://127.0.0.1:18080 LABEL=main OUT_DIR=/opt/cursor/artifacts node scripts/wind_cross_zero_screenshots.mjs
 */
import { chromium } from "playwright";
import { createHash } from "node:crypto";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18081";
const outDir = process.env.OUT_DIR ?? "/opt/cursor/artifacts";
const label = process.env.LABEL ?? "branch";
const themes = (process.env.THEMES ?? "light,dark").split(",").filter(Boolean);

const STRIP_VIEWBOX_W = 1320;
const STRIP_VIEWBOX_H = 560;
const STRIP_TOP = 40;
const STRIP_BOTTOM = 500;
const TAKEOFF_WIND_X = 770;
const TAKEOFF_WIND_W = 400;
const TAKEOFF_TW_W = TAKEOFF_WIND_W * (5 / 25);
const TAKEOFF_HW_X = TAKEOFF_WIND_X + TAKEOFF_TW_W;
const TAKEOFF_HW_W = TAKEOFF_WIND_W - TAKEOFF_TW_W;

function xHeadwindKt(hw) {
  const h = Math.max(0, Math.min(20, hw));
  return TAKEOFF_HW_X + (h / 20) * TAKEOFF_HW_W;
}

function xTailwindKt(tw) {
  const t = Math.max(0, Math.min(5, tw));
  return TAKEOFF_WIND_X + TAKEOFF_TW_W - (t / 5) * TAKEOFF_TW_W;
}

function viewboxToClientMeet(sx, sy, rect) {
  const scale = Math.min(rect.width / STRIP_VIEWBOX_W, rect.height / STRIP_VIEWBOX_H);
  const renderedW = STRIP_VIEWBOX_W * scale;
  const renderedH = STRIP_VIEWBOX_H * scale;
  const ox = rect.x + (rect.width - renderedW) * 0.5;
  const oy = rect.y + (rect.height - renderedH) * 0.5;
  return { x: ox + sx * scale, y: oy + sy * scale };
}

async function clearDa20Storage(page) {
  await page.evaluate(() => {
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const k = localStorage.key(i);
      if (k && k.startsWith("da20_")) localStorage.removeItem(k);
    }
  });
}

async function setupEsTheme(page, theme) {
  await page.goto(`${baseUrl}/#takeoff`, { waitUntil: "networkidle", timeout: 120_000 });
  await clearDa20Storage(page);
  await page.evaluate((t) => {
    localStorage.setItem("da20_theme_v1", JSON.stringify(t === "dark" ? "Dark" : "Light"));
    localStorage.setItem("da20_locale_v1", "Es");
  }, theme);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  const themeSelect = page.locator("label.theme-picker select.palette-select");
  await themeSelect.waitFor({ state: "visible", timeout: 30_000 });
  await themeSelect.selectOption(theme);
  await page.waitForFunction(
    (t) => document.documentElement.getAttribute("data-theme") === t,
    theme,
    { timeout: 15_000 },
  );
  await page.waitForSelector(".strip-svg", { timeout: 60_000 });
  await page.waitForTimeout(500);
}

async function readTakeoffFields(page) {
  return page.evaluate(() => {
    const body = document.body.innerText;
    const windInputs = Array.from(document.querySelectorAll("input.mono"))
      .map((el) => el.value)
      .filter(Boolean);
    const windMeta = body.match(/\d{3}\s*\/\s*\d{2}\s*kt/);
    const along = body.match(/Viento de (frente|cola)[^\d-]*(-?\d+\.\d)/);
    const signedField = Array.from(document.querySelectorAll(".card .f"))
      .find((el) => el.textContent?.includes("Viento en pista"))
      ?.querySelector("input.mono")?.value;
    const oob = body.includes("Fuera de carta (panel distance)");
    const weightBad =
      /\b1326\b/.test(body) ||
      /\b143\.8220480\b/.test(body) ||
      (/53\.9/.test(body) && /PA/.test(body));
    return {
      wind: windMeta?.[0] ?? windInputs.join("|"),
      along,
      signedAlong: signedField ?? "",
      oob,
      weightBad,
    };
  });
}

async function dragWindStripHwToTw(page) {
  const svg = page.locator(".strip-svg").first();
  await svg.waitFor({ state: "visible", timeout: 30_000 });
  let box = await svg.boundingBox();
  for (let i = 0; i < 10 && (!box || box.width < 100); i++) {
    await page.waitForTimeout(200);
    box = await svg.boundingBox();
  }
  if (!box) throw new Error("strip svg missing");
  const sy = (STRIP_TOP + STRIP_BOTTOM) * 0.5;
  const sxStart = xHeadwindKt(8);
  const sxEnd = xTailwindKt(3);
  const start = viewboxToClientMeet(sxStart, sy, box);
  const end = viewboxToClientMeet(sxEnd, sy, box);
  if (!Number.isFinite(start.x) || !Number.isFinite(end.x) || box.width < 100) {
    throw new Error(`bad wind drag coords start=${JSON.stringify(start)} box=${JSON.stringify(box)}`);
  }
  if (start.x < box.x + box.width * 0.45) {
    throw new Error(`wind drag start x ${start.x} looks like OAT/weight panel (rect ${box.x})`);
  }
  await page.mouse.move(Math.round(start.x), Math.round(start.y));
  await page.mouse.down();
  const steps = 14;
  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    const x = Math.round(start.x + (end.x - start.x) * t);
    await page.mouse.move(x, Math.round(start.y));
    await page.waitForTimeout(50);
  }
  await page.mouse.up();
  await page.waitForTimeout(800);
}

async function snap(page, name, theme) {
  const file = path.join(outDir, `${name}_${label}_${theme}.png`);
  await page.screenshot({ path: file, fullPage: false });
  const buf = await readFile(file);
  const md5 = createHash("md5").update(buf).digest("hex");
  console.log("wrote", file, "md5", md5);
  return { file, md5 };
}

async function assertPostDrag(page, label, theme) {
  const f = await readTakeoffFields(page);
  if (f.oob) {
    throw new Error(`${label}/${theme}: Fuera de carta (panel distance) — drag missed wind panel`);
  }
  if (f.weightBad) {
    throw new Error(`${label}/${theme}: PA/OAT/weight strip was hit (off-chart panel values)`);
  }
  if (label === "branch") {
    const signed = Number(f.signedAlong);
    if (!(signed < -2.0)) {
      throw new Error(`${label}/${theme}: expected TW ~−3 kt signed field, got ${f.signedAlong}`);
    }
    const windOk =
      f.wind.includes("020") || f.wind.includes("019") || f.wind.includes("03");
    if (!windOk && !String(f.along?.[1]).includes("cola")) {
      throw new Error(`${label}/${theme}: expected dir flip / TW, wind=${f.wind} along=${f.along}`);
    }
  } else if (label === "main") {
  if (f.along && f.along[1] === "frente" && Number(f.along[2]) <= 0.05) {
      /* stuck at 0 kt headwind */
    }
  }
  return f;
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });

const windInput = () =>
  page.locator(".card .f").filter({ hasText: /Viento en pista/ }).locator("input.mono").first();

for (const theme of themes) {
  await setupEsTheme(page, theme);
  await snap(page, "wind_takeoff_fig54_default", theme);

  const wi = windInput();
  await wi.fill("4");
  await page.waitForTimeout(400);
  await snap(page, "wind_signed_plus4", theme);

  await wi.fill("-3");
  await page.waitForTimeout(400);
  await snap(page, "wind_signed_minus3", theme);

  await setupEsTheme(page, theme);
  await dragWindStripHwToTw(page);
  const after = await assertPostDrag(page, label, theme);
  console.log(`${label} ${theme} after drag:`, after.wind, after.along);
  await snap(page, "wind_strip_drag_cross_zero", theme);
}

await browser.close();

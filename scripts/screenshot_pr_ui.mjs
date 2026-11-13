import { chromium } from "playwright";
import { createHash } from "node:crypto";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:8080";
const outDir = process.env.OUT_DIR ?? "docs/pr11-screenshots";
const label = process.env.LABEL ?? "after";
const themes = (process.env.THEMES ?? "light,dark,night").split(",").filter(Boolean);

const themeStorage = {
  light: '"Light"',
  dark: '"Dark"',
  night: '"Night"',
};

const tabs = [
  { key: "takeoff", slug: "takeoff", wait: ".strip-svg", btn: "Despegue" },
  { key: "cruise", slug: "cruise", wait: ".cruise-table-full, .plot-svg", btn: "Crucero" },
  { key: "endurance", slug: "endurance", wait: ".cruise-table-full, .plot-svg", btn: "Autonomía" },
  { key: "landing", slug: "landing", wait: ".plot-svg", btn: "Aterrizaje" },
  { key: "method", slug: "method", wait: ".method-list", btn: "Método" },
];

async function md5File(file) {
  const buf = await readFile(file);
  return createHash("md5").update(buf).digest("hex");
}

async function clearDa20Storage(page) {
  await page.evaluate(() => {
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const k = localStorage.key(i);
      if (k && k.startsWith("da20_")) {
        localStorage.removeItem(k);
      }
    }
  });
}

async function applyThemeEs(page, theme) {
  const storage = themeStorage[theme] ?? '"Light"';
  await clearDa20Storage(page);
  await page.evaluate((s) => {
    localStorage.setItem("da20_theme_v1", s);
    localStorage.setItem("da20_locale_v1", "Es");
  }, storage);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  const themeSelect = page.locator("label.theme-picker select.palette-select");
  await themeSelect.waitFor({ state: "visible", timeout: 30_000 });
  await themeSelect.selectOption(theme);
  await page.waitForFunction(
    (t) => document.documentElement.getAttribute("data-theme") === t,
    theme,
    { timeout: 15_000 },
  );
  await page.waitForTimeout(800);
}

async function scrollMethodToNewCopy(page) {
  const list = page.locator(".method-list");
  await list.waitFor({ state: "visible", timeout: 30_000 });
  const entry = page
    .locator(".method-list li")
    .filter({ hasText: /Crucero \(Tabla 3\)|Cruise \(Table 3\)/i })
    .first();
  await entry.waitFor({ state: "attached", timeout: 30_000 });
  const interp = entry.locator("em").first();
  await interp.waitFor({ state: "attached", timeout: 30_000 });
  await interp.evaluate((el) => {
    const root = el.closest(".method-list");
    if (root) {
      const top = el.offsetTop - root.clientHeight / 4;
      root.scrollTop = Math.max(0, top);
    }
    el.scrollIntoView({ block: "start", behavior: "instant" });
  });
  await page.waitForTimeout(800);
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
await page.goto(`${baseUrl}/?v=${Date.now()}`, {
  waitUntil: "networkidle",
  timeout: 120_000,
});
await clearDa20Storage(page);
await page.reload({ waitUntil: "networkidle", timeout: 120_000 });

const md5Changes = [];

for (const theme of themes) {
  await applyThemeEs(page, theme);

  for (const tab of tabs) {
    const btn = page.getByRole("button", { name: tab.btn, exact: false });
    await btn.first().click();
    await page.waitForFunction(
      (slug) => window.location.hash.includes(slug),
      tab.slug,
      { timeout: 15_000 },
    );
    await page.waitForSelector(tab.wait, { timeout: 30_000 }).catch(() => {});

    if (tab.key === "takeoff" && label === "after") {
      const lbBtn = page.getByRole("button", { name: /^lb$/i });
      if ((await lbBtn.count()) > 0) {
        await lbBtn.first().click();
      }
      const w = page.locator(".unit-row input.mono").first();
      if ((await w.count()) > 0) {
        await w.click();
        await w.fill("1763");
        await w.press("Tab");
        await page.waitForTimeout(400);
      }
    }

    if (tab.key === "landing" && label === "after") {
      const offBtn = page.getByRole("button", { name: "No", exact: true });
      if ((await offBtn.count()) > 0) {
        await offBtn.click();
        await page.waitForTimeout(400);
      }
    }

    if (tab.key === "cruise" && label === "after") {
      const mt = page.getByRole("button", { name: /MT 175|175 R/i });
      if ((await mt.count()) > 0) {
        await mt.first().click();
        await page.waitForTimeout(400);
      }
      const bhp = page.getByRole("button", { name: "Fijar %BHP", exact: true });
      if ((await bhp.count()) > 0) {
        await bhp.click();
        await page.waitForTimeout(300);
      }
    }

    if (tab.key === "method" && label === "after") {
      await scrollMethodToNewCopy(page);
    }

    const file = path.join(outDir, `${label}_${theme}_${tab.key}.png`);
    const beforePath = path.join(outDir, `before_light_${tab.key}.png`);
    let beforeMd5 = null;
    try {
      beforeMd5 = await md5File(beforePath);
    } catch {
      /* no before baseline */
    }
    const prevMd5 = await md5File(file).catch(() => null);
    const fullPage = tab.key === "method" && label === "after";
    await page.screenshot({ path: file, fullPage });
    const afterMd5 = await md5File(file);
    md5Changes.push({
      file: path.basename(file),
      md5: afterMd5,
      changedFromPrevious: prevMd5 !== null && prevMd5 !== afterMd5,
      changedFromBeforeLight:
        beforeMd5 !== null && tab.key === "method" && theme === "light"
          ? beforeMd5 !== afterMd5
          : null,
    });
    console.log("wrote", file, afterMd5);
  }
}

await browser.close();
console.log("md5 summary", JSON.stringify(md5Changes, null, 2));

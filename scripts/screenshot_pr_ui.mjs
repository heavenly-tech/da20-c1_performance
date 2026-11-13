import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
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
  { key: "landing", slug: "landing", wait: ".plot-svg", btn: "Aterrizaje" },
  { key: "method", slug: "method", wait: ".method-list", btn: "Método" },
];

async function applyThemeEs(page, theme) {
  const storage = themeStorage[theme] ?? '"Light"';
  await page.evaluate((s) => {
    localStorage.setItem("da20_theme_v1", s);
    localStorage.setItem("da20_locale_v1", "Es");
  }, storage);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  await page.waitForTimeout(600);
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
await page.goto(`${baseUrl}/?v=${Date.now()}`, {
  waitUntil: "networkidle",
  timeout: 120_000,
});

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

    if (tab.key === "landing" && label === "after") {
      const onBtn = page.getByRole("button", { name: "Sí", exact: true });
      if ((await onBtn.count()) > 0) {
        await onBtn.click();
        await page.waitForTimeout(400);
      }
    }

    if (tab.key === "cruise" && label === "after") {
      const bhp = page.getByRole("button", { name: "Fijar %BHP", exact: true });
      if ((await bhp.count()) > 0) {
        await bhp.click();
        await page.waitForTimeout(300);
      }
    }

    if (tab.key === "method" && label === "after") {
      await page
        .locator(".method-list")
        .evaluate((el) => {
          el.scrollTop = el.scrollHeight;
        })
        .catch(() => {});
      await page.waitForTimeout(400);
    }

    const file = path.join(outDir, `${label}_${theme}_${tab.key}.png`);
    await page.screenshot({ path: file, fullPage: false });
    console.log("wrote", file);
  }
}

await browser.close();

import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:8080";
const outDir = process.env.OUT_DIR ?? "docs/pr11-screenshots";
const label = process.env.LABEL ?? "after";

const tabs = [
  { key: "takeoff", slug: "takeoff", wait: ".strip-svg", btn: "Despegue" },
  { key: "cruise", slug: "cruise", wait: ".cruise-table-full, .plot-svg", btn: "Crucero" },
  { key: "landing", slug: "landing", wait: ".plot-svg", btn: "Aterrizaje" },
  { key: "method", slug: "method", wait: ".method-list", btn: "Método" },
];

async function applyLightEs(page) {
  await page.evaluate(() => {
    localStorage.setItem("da20_theme_v1", '"Light"');
    localStorage.setItem("da20_locale_v1", "Es");
  });
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
await applyLightEs(page);

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

  const file = path.join(outDir, `${label}_light_${tab.key}.png`);
  await page.screenshot({ path: file, fullPage: false });
  console.log("wrote", file);
}

await browser.close();

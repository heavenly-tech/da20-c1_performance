import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:8080";
const outDir = process.env.OUT_DIR ?? "docs/pr-climb-calculator-screenshots";

const localeStorage = { Es: '"Es"', En: '"En"' };

async function setup(page, locale) {
  await page.goto(baseUrl, { waitUntil: "networkidle", timeout: 120_000 });
  await page.evaluate(
    (loc) => {
      localStorage.setItem("da20_theme_v1", '"Light"');
      localStorage.setItem("da20_locale_v1", loc);
    },
    localeStorage[locale],
  );
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
}

async function shotClimbCalc(page, locale) {
  const tab = locale === "Es" ? "Ascenso" : "Climb";
  await page.getByRole("button", { name: tab }).click();
  await page.waitForTimeout(600);
  const calc = page.locator(".sweep-card").first();
  await calc.waitFor({ state: "visible", timeout: 30_000 });
  await calc.getByRole("button", { name: /Calcular tabla|Build table/i }).click();
  await page.waitForSelector(".sweep-table tbody tr", { timeout: 30_000 });
  await page.waitForTimeout(500);
  await calc.screenshot({
    path: path.join(outDir, `climb_calculator_${locale.toLowerCase()}.png`),
  });
}

async function main() {
  await mkdir(outDir, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1400, height: 1200 } });
  for (const loc of ["Es", "En"]) {
    await setup(page, loc);
    await shotClimbCalc(page, loc);
  }
  await browser.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
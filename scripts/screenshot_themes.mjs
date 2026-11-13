import { chromium } from "playwright";
import { createHash } from "node:crypto";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:8080";
const outDir = process.env.OUT_DIR ?? "/opt/cursor/artifacts";
const label = process.env.LABEL ?? "after";
const themes = (process.env.THEMES ?? "light,dark,night").split(",").filter(Boolean);

const TAB_LABELS_ES = {
  takeoff: "Despegue",
  cruise: "Crucero y potencia",
  landing: "Aterrizaje",
};

const TAB_WAIT = {
  takeoff: ".strip-svg",
  cruise: ".cruise-table-full, .plot-svg",
  landing: ".plot-svg",
};

const themeStorage = {
  light: '"Light"',
  dark: '"Dark"',
  night: '"Night"',
};

async function md5File(file) {
  const buf = await readFile(file);
  return createHash("md5").update(buf).digest("hex");
}

async function applyTheme(page, theme) {
  const storage = themeStorage[theme] ?? '"Light"';
  await page.evaluate(
    ([t, s]) => {
      localStorage.setItem("da20_theme_v1", s);
      document.documentElement.setAttribute("data-theme", t);
      document.documentElement.style.colorScheme =
        t === "light" ? "light only" : "dark only";
    },
    [theme, storage],
  );
}

async function setTakeoffWeightLb(page, lbs) {
  const weightRow = page.locator(".f").filter({ hasText: /Peso|Weight/i }).first();
  const input = weightRow.locator("input[type='number']").first();
  if ((await input.count()) > 0) {
    await input.fill(String(lbs));
    await input.press("Enter");
    await page.waitForTimeout(500);
  }
}

async function clickTab(page, tabKey) {
  const name = TAB_LABELS_ES[tabKey];
  const btn = page.getByRole("button", { name, exact: true });
  await btn.click();
  await page.waitForFunction(
    (slug) => window.location.hash.includes(slug),
    `/${tabKey}`,
    { timeout: 15_000 },
  );
  await page.waitForSelector(TAB_WAIT[tabKey], { timeout: 30_000 });
  await page.waitForTimeout(1200);
}

await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });

const saved = [];
for (const theme of themes) {
  await page.goto(`${baseUrl}/`, { waitUntil: "networkidle", timeout: 120_000 });
  await applyTheme(page, theme);

  for (const tab of ["takeoff", "cruise", "landing"]) {
    await clickTab(page, tab);

    if (tab === "takeoff") {
      if (theme === "night") {
        await setTakeoffWeightLb(page, 1764);
      } else {
        await setTakeoffWeightLb(page, 1598);
      }
      await page.waitForTimeout(800);
    }

    const file = path.join(outDir, `${label}_${theme}_${tab}.png`);
    await page.screenshot({ path: file, fullPage: false });
    const hash = await md5File(file);
    const hashLine = `${path.basename(file)} ${hash}`;
    console.log(hashLine);
    saved.push({ file, tab, theme, hash });
  }
}

await browser.close();

const byHash = new Map();
for (const s of saved) {
  const list = byHash.get(s.hash) ?? [];
  list.push(s.file);
  byHash.set(s.hash, list);
}
for (const [hash, files] of byHash) {
  if (files.length > 1) {
    console.error("DUPLICATE SCREENSHOTS (same md5):", hash, files);
    process.exit(1);
  }
}

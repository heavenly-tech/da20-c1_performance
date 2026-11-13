import { chromium } from "playwright";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:8080";
const outDir = process.env.OUT_DIR ?? "/opt/cursor/artifacts";
const label = process.env.LABEL ?? "after";
const themes = (process.env.THEMES ?? "light,dark,night").split(",").filter(Boolean);
const verifyOldMd5Path = process.env.VERIFY_OLD_MD5 ?? "";
const onlyShots = (process.env.ONLY_SHOTS ?? "")
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);

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

async function loadOldMd5Map(filePath) {
  if (!filePath) return new Map();
  const text = await readFile(filePath, "utf8");
  const map = new Map();
  for (const line of text.split("\n")) {
    const parts = line.trim().split(/\s+/);
    if (parts.length >= 2) {
      map.set(parts[parts.length - 1], parts[0]);
    }
  }
  return map;
}

async function applyTheme(page, theme) {
  const storage = themeStorage[theme] ?? '"Light"';
  await page.evaluate((s) => {
    localStorage.setItem("da20_theme_v1", s);
  }, storage);
  await page.reload({ waitUntil: "networkidle", timeout: 120_000 });
  const select = page.locator("label.theme-picker select.palette-select");
  await select.waitFor({ state: "visible", timeout: 15_000 });
  let value = await select.inputValue();
  if (value !== theme) {
    await select.selectOption(theme);
    await page.waitForTimeout(400);
    value = await select.inputValue();
  }
  const domTheme = await page.evaluate(
    () => document.documentElement.getAttribute("data-theme"),
  );
  if (value !== theme || domTheme !== theme) {
    throw new Error(
      `theme mismatch want=${theme} select=${value} data-theme=${domTheme}`,
    );
  }
}

async function setTakeoffWeightLb(page, lbs) {
  const input = page.locator(".unit-row input[type='text']").first();
  await input.waitFor({ state: "visible", timeout: 15_000 });
  await input.click();
  await input.fill(String(lbs));
  await input.press("Tab");
  await page.waitForTimeout(2000);
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
  await page.waitForTimeout(800);
}

async function assertLightPrimaryButton(page) {
  const bg = await page.evaluate(() => {
    const el = document.querySelector(".results-block ~ button.btn, .save-row .btn");
    const btn =
      document.querySelector('button.btn:not(.g)') ??
      document.querySelector(".btn");
    if (!btn) return null;
    return getComputedStyle(btn).backgroundColor;
  });
  if (bg && !bg.includes("138") && !bg.includes("133")) {
    console.warn("warn: primary button bg unexpected:", bg);
  }
}

await mkdir(outDir, { recursive: true });
const oldMd5 = await loadOldMd5Map(verifyOldMd5Path);

const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: { width: 1400, height: 1000 },
});
await context.route("**/*", (route) => {
  const headers = {
    ...route.request().headers(),
    "Cache-Control": "no-cache, no-store, must-revalidate",
    Pragma: "no-cache",
  };
  route.continue({ headers });
});
const page = await context.newPage();

const bust = Date.now();
const saved = [];
for (const theme of themes) {
  await page.goto(`${baseUrl}/?v=${bust}-${theme}`, {
    waitUntil: "networkidle",
    timeout: 120_000,
  });
  await applyTheme(page, theme);

  for (const tab of ["takeoff", "cruise", "landing"]) {
    await clickTab(page, tab);

    if (tab === "cruise") {
      const isaBtn = page.getByRole("button", { name: "ISA", exact: true });
      if ((await isaBtn.count()) > 0) {
        await isaBtn.click();
        await page.waitForTimeout(500);
      }
    }

    if (tab === "takeoff") {
      const lbs = theme === "night" ? 1764 : 1598;
      await setTakeoffWeightLb(page, lbs);
      if (theme === "light") {
        await assertLightPrimaryButton(page);
      }
      if (theme !== "night") {
        const ooc = page.locator("p.error.warn");
        if ((await ooc.count()) > 0) {
          const txt = await ooc.first().textContent();
          console.error(`unexpected OOC on ${theme} takeoff: ${txt}`);
          process.exit(1);
        }
      }
    }

    const shotName = `${label}_${theme}_${tab}.png`;
    if (onlyShots.length > 0 && !onlyShots.includes(shotName)) {
      continue;
    }
    const file = path.join(outDir, shotName);
    await page.screenshot({ path: file, fullPage: false });
    const hash = await md5File(file);
    const base = path.basename(file);
    const prev = oldMd5.get(base);
    if (prev && prev === hash) {
      console.error(`STALE: ${base} md5 unchanged from previous (${hash})`);
      process.exit(1);
    }
    console.log(`${base} ${hash}${prev ? ` (was ${prev})` : ""}`);
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

await writeFile(
  path.join(outDir, `${label}_manifest.json`),
  JSON.stringify(saved, null, 2),
);

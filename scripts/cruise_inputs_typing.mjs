/**
 * Playwright: multi-digit numeric fields keep focus and DOM node identity while typing.
 *
 * Prereqs:
 *   cd crates/da20-ui && trunk build --release
 *   cargo build --release -p da20-server
 *   (dist at repo-root dist/ — Dockerfile layout)
 *   npx --yes playwright install chromium
 *
 * Run (starts da20-server when MANAGE_SERVER is unset or "1"):
 *   node scripts/cruise_inputs_typing.mjs
 */
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18081";
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manageServer = process.env.MANAGE_SERVER !== "0";

const TAB = {
  takeoff: { es: "Despegue", en: "Takeoff" },
  climb: { es: "Ascenso", en: "Climb" },
  cruise: { es: "Crucero y potencia", en: "Cruise & power" },
  endurance: { es: "Autonomía y alcance", en: "Endurance & range" },
  landing: { es: "Aterrizaje", en: "Landing" },
};

async function localeIsEn(page) {
  const locale = await page.evaluate(() => document.documentElement.lang);
  return locale === "en" || locale.startsWith("en");
}

async function clickTab(page, key) {
  const en = await localeIsEn(page);
  const name = en ? TAB[key].en : TAB[key].es;
  const btn = page.getByRole("button", { name, exact: true });
  await btn.click();
  await page.waitForFunction(
    (slug) => window.location.hash.includes(`/${slug}`),
    key,
    { timeout: 15_000 },
  );
  await page.waitForTimeout(600);
}

async function openApp(page) {
  await page.goto(baseUrl, { waitUntil: "networkidle", timeout: 120_000 });
}

function fieldByLabel(page, labelText) {
  const box = page
    .locator(".card .f")
    .filter({ has: page.locator("label", { hasText: labelText }) });
  return box.locator("input.mono").first();
}

function fieldByLabelPattern(page, pattern) {
  const box = page
    .locator(".card .f")
    .filter({ has: page.locator("label", { hasText: pattern }) });
  return box.locator("input.mono").first();
}

async function assertStableTyping(page, input, typed, expectedValue) {
  await input.waitFor({ state: "visible", timeout: 15_000 });
  const handle = await input.elementHandle();
  if (!handle) {
    throw new Error("numeric input not found");
  }
  await handle.click();
  await page.keyboard.press("Control+a");
  const probe = await handle.evaluate((el) => {
    const token = `nf-${Math.random().toString(36).slice(2)}`;
    el.dataset.numFieldProbe = token;
    return token;
  });
  if (!(await handle.evaluate((el) => document.activeElement === el))) {
    throw new Error("focus not on input after click");
  }
  for (const ch of typed) {
    await page.keyboard.type(ch, { delay: 40 });
    const sameNode = await page.evaluate((token) => {
      const el = document.activeElement;
      return (
        el instanceof HTMLInputElement && el.dataset.numFieldProbe === token
      );
    }, probe);
    if (!sameNode) {
      throw new Error(`input node replaced after typing "${ch}"`);
    }
  }
  await handle.press("Tab");
  await page.waitForTimeout(300);
  const finalValue = await handle.inputValue();
  if (finalValue !== expectedValue) {
    throw new Error(`expected value "${expectedValue}", got "${finalValue}"`);
  }
}

async function runCruiseChecks(page, en) {
  await clickTab(page, "cruise");
  await page.waitForSelector(".cruise-table-full, .plot-svg", { timeout: 30_000 });

  const paLabel = en ? "Cruise PA (ft)" : "PA crucero (ft)";
  const oatPattern = en
    ? /Cruise OAT|Estimated OAT/i
    : /OAT crucero|OAT estimada/i;
  const rpmLabel = en ? "Cruise RPM" : "RPM crucero";
  const bhpLabel = en ? "Cruise % BHP" : "% BHP crucero";

  await assertStableTyping(page, fieldByLabel(page, paLabel), "5000", "5000");
  await assertStableTyping(page, fieldByLabelPattern(page, oatPattern), "15", "15.0");

  const bhpMode = page.getByRole("button", {
    name: en ? "Set %BHP" : "Fijar %BHP",
    exact: true,
  });
  await bhpMode.click();
  await page.waitForTimeout(200);
  await assertStableTyping(page, fieldByLabel(page, bhpLabel), "65", "65");

  const rpmMode = page.getByRole("button", {
    name: en ? "Set RPM" : "Fijar RPM",
    exact: true,
  });
  await rpmMode.click();
  await page.waitForTimeout(200);
  await assertStableTyping(page, fieldByLabel(page, rpmLabel), "2450", "2450");
}

async function runTakeoffAirfieldChecks(page, en) {
  await clickTab(page, "takeoff");
  const paLabel = en ? "Pressure altitude (ft)" : "Altitud de presión (ft)";
  const oatLabel = "OAT (°C)";
  await assertStableTyping(page, fieldByLabel(page, paLabel), "3100", "3100");
  await assertStableTyping(page, fieldByLabel(page, oatLabel), "18", "18.0");
}

async function runClimbChecks(page, en) {
  await clickTab(page, "climb");
  const paLabel = en ? "Pressure altitude (ft)" : "Altitud de presión (ft)";
  const oatLabel = "OAT (°C)";
  const targetLabel = en
    ? "Climb target PA (ft)"
    : "Altitud objetivo de trepada (ft)";
  await assertStableTyping(page, fieldByLabel(page, paLabel), "3500", "3500");
  await assertStableTyping(page, fieldByLabel(page, oatLabel), "12", "12.0");
  await assertStableTyping(page, fieldByLabel(page, targetLabel), "4500", "4500");
}

async function runLandingChecks(page, en) {
  await clickTab(page, "landing");
  const paLabel = en ? "Pressure altitude (ft)" : "Altitud de presión (ft)";
  await assertStableTyping(page, fieldByLabel(page, paLabel), "2500", "2500");
}

async function runEnduranceChecks(page, en) {
  await clickTab(page, "endurance");
  await page.waitForSelector(".results-block .big, .cruise-table-full", {
    timeout: 60_000,
  });

  const paLabel = en ? "Cruise PA (ft)" : "PA crucero (ft)";
  const oatPattern = en
    ? /Cruise OAT|Estimated OAT/i
    : /OAT crucero|OAT estimada/i;
  const groundElev = en
    ? "Aerodrome elevation (ft)"
    : "Elevación del aeródromo (ft)";
  const fuelLabel = en ? "Usable fuel (US gal)" : "Combustible utilizable (US gal)";
  const reserveLabel = en ? "Reserve (US gal)" : "Reserva (US gal)";

  await assertStableTyping(page, fieldByLabel(page, paLabel), "5000", "5000");
  await assertStableTyping(page, fieldByLabelPattern(page, oatPattern), "15", "15.0");
  await assertStableTyping(page, fieldByLabel(page, groundElev), "2200", "2200");
  await assertStableTyping(page, fieldByLabel(page, fuelLabel), "24", "24");
  await assertStableTyping(page, fieldByLabel(page, reserveLabel), "5", "5");
}

function startServer() {
  const port = new URL(baseUrl).port || "8080";
  return spawn(path.join(repoRoot, "target/release/da20-performance"), [], {
    env: { ...process.env, PORT: port },
    stdio: ["ignore", "pipe", "pipe"],
  });
}

async function main() {
  let server;
  if (manageServer) {
    server = startServer();
    await new Promise((r) => setTimeout(r, 800));
  }
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await openApp(page);
    const en = await localeIsEn(page);
    await runCruiseChecks(page, en);
    await runTakeoffAirfieldChecks(page, en);
    await runClimbChecks(page, en);
    await runLandingChecks(page, en);
    await runEnduranceChecks(page, en);
    console.log("numeric_inputs_typing: OK");
  } finally {
    await browser.close();
    if (server) {
      server.kill("SIGTERM");
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

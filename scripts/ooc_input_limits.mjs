/**
 * Playwright: out-of-chart values are not clamped; UI shows Fuera de carta.
 *
 * Prereqs:
 *   cd crates/da20-ui && trunk build --release
 *   cargo build --release -p da20-server
 *   npx --yes playwright install chromium
 *
 *   node scripts/ooc_input_limits.mjs
 */
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const baseUrl = process.env.BASE_URL ?? "http://127.0.0.1:18081";
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manageServer = process.env.MANAGE_SERVER !== "0";

let serverProc;

function startServer() {
  return new Promise((resolve, reject) => {
    serverProc = spawn(
      "cargo",
      ["run", "--release", "-p", "da20-server"],
      {
        cwd: repoRoot,
        stdio: "inherit",
        env: { ...process.env, PORT: "18081" },
      },
    );
    serverProc.on("error", reject);
    setTimeout(resolve, 2500);
  });
}

async function openTakeoff(page) {
  await page.goto(baseUrl, { waitUntil: "networkidle", timeout: 120_000 });
  const esBtn = page.getByRole("button", { name: /Español|English/i });
  if (await esBtn.count()) {
    await esBtn.click();
  }
  const tab = page.getByRole("button", { name: /Despegue|Takeoff/i });
  await tab.click();
  await page.waitForFunction(() => window.location.hash.includes("/takeoff"), null, {
    timeout: 15_000,
  });
}

function fieldByLabel(page, pattern) {
  const box = page
    .locator(".card .f")
    .filter({ has: page.locator("label", { hasText: pattern }) });
  return box.locator("input.mono").first();
}

async function commitField(page, input, value) {
  await input.click();
  await page.keyboard.press("Control+a");
  await page.keyboard.type(value);
  await page.keyboard.press("Tab");
  await page.waitForTimeout(400);
}

async function assertOocVisible(page) {
  const ooc = page.locator("p.warn, p.error.warn").filter({ hasText: /Fuera de carta|Out of chart/i });
  await ooc.first().waitFor({ state: "visible", timeout: 10_000 });
}

async function assertComputes(page) {
  const ooc = page.locator("p.warn, p.error.warn").filter({ hasText: /Fuera de carta|Out of chart/i });
  await ooc.first().waitFor({ state: "hidden", timeout: 10_000 }).catch(() => {});
  const dist = page.locator(".readout-text, .mono").filter({ hasText: /\d+\s*m/ });
  if ((await dist.count()) === 0) {
    throw new Error("expected numeric distance readout");
  }
}

async function main() {
  if (manageServer) {
    await startServer();
  }
  const browser = await chromium.launch();
  const page = await browser.newPage();
  try {
    await openTakeoff(page);
    await page.locator('input[maxlength="3"]').fill("360");

    const signedWind = fieldByLabel(page, /Viento en pista|Along-runway wind/i);
    await commitField(page, signedWind, "-5.1");
    await assertOocVisible(page);
    const limits = page.locator(".field-error").filter({ hasText: /máx\. 5 kt|max tailwind 5 kt/i });
    await limits.first().waitFor({ state: "visible", timeout: 5000 });
    await page.locator(".f.out-of-chart input.mono").first().waitFor({ state: "visible" });

    await commitField(page, signedWind, "-5.0");
    await assertComputes(page);

    await commitField(page, signedWind, "20.1");
    await assertOocVisible(page);
    await commitField(page, signedWind, "20.0");
    await assertComputes(page);

    const kgBtn = page.getByRole("button", { name: /^kg$/i });
    if (await kgBtn.count()) {
      await kgBtn.click();
    }
    const weight = fieldByLabel(page, /Peso \(kg\)|Weight \(kg\)|^Peso$|^Weight$/i);
    await commitField(page, weight, "800.1");
    await assertOocVisible(page);
    await commitField(page, weight, "800.0");
    await assertComputes(page);

    console.log("ooc_input_limits: ok");
  } finally {
    await browser.close();
    serverProc?.kill("SIGTERM");
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
// Spike 02 (throwaway): browser driver. `bun driver.mjs <job.mjs>`; the job
// module's default export receives { page, spike, shot, out }.
import { createRequire } from "node:module";
import { mkdirSync, writeFileSync } from "node:fs";
import { spawn } from "node:child_process";
const require = createRequire("/Users/david/dev/battlegame/web/package.json");
const { chromium } = require("playwright");
const { PNG } = require("pngjs");

const HERE = new URL(".", import.meta.url).pathname;
const OUT = new URL("../evidence/spike02/", import.meta.url).pathname;
mkdirSync(OUT, { recursive: true });
const port = 5802 + Math.floor(Math.random() * 100);
const server = spawn("bun", [HERE + "serve.ts"], { env: { ...process.env, PORT: String(port) }, stdio: "inherit" });
await new Promise((r) => setTimeout(r, 800));
const browser = await chromium.launch({
  channel: "chromium",
  args: ["--enable-unsafe-webgpu", "--enable-features=WebGPU", "--enable-dawn-features=allow_unsafe_apis",
    "--disable-dawn-features=timestamp_quantization"],
});
try {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  page.on("console", (m) => { const t = m.text(); if (!t.startsWith("[spike02]") || process.env.VERBOSE) console.log("page:", t); });
  page.on("pageerror", (e) => console.log("pageerror:", e.message));
  await page.goto(`http://localhost:${port}/`);
  await page.waitForFunction(() => window.spikeReady);
  await page.evaluate(() => window.spike.init());
  const spike = new Proxy({}, { get: (_, fn) => (...args) => page.evaluate(({ fn, args }) => window.spike[fn](...args), { fn, args }) });
  const shot = async (name) => {
    const b64 = await spike.capture();
    const raw = Buffer.from(b64, "base64");
    const png = new PNG({ width: 1920, height: 1080 });
    raw.copy(png.data);
    const path = OUT + name + ".png";
    writeFileSync(path, PNG.sync.write(png));
    return { path, raw };
  };
  const job = await import(new URL(process.argv[2], `file://${process.cwd()}/`).href);
  await job.default({ page, spike, shot, out: OUT });
} finally {
  await browser.close();
  server.kill();
}

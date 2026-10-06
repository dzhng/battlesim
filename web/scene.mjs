// Deterministic browser verification for registered lab fixtures.
//
//   bun run --cwd web scene -- <fixture-id>...   run those fixtures' scenes
//   bun run --cwd web scene                      run every registered scene
//   bun run --cwd web scene -- --list            list fixture ids
//
// The registry is apps/battle-lab/src/fixtures.json; every fixture id must have
// exactly one scene at web/scenes/<id>.mjs and vice versa. Without VERIFY_URL
// the runner starts its own Vite server; a fixture registered with
// `"build": "production"` (a timing verdict) runs against a production build
// served by Vite's preview instead. A scene longer than its fixture's
// `timeout_s` (900 by default; SCENE_TIMEOUT_S overrides every fixture's)
// fails. Evidence goes to throwaway/evidence/.
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const HERE = new URL(".", import.meta.url);
const REGISTRY = new URL("../apps/battle-lab/src/fixtures.json", HERE);
const SCENES_DIR = new URL("./scenes/", HERE);
const EVIDENCE_DIR = new URL("../throwaway/evidence/", HERE);

export const WEBGPU_FLAGS = ["--enable-unsafe-webgpu", "--enable-features=WebGPU"];

/** React's development build logs each commit's changed props into a
 *  `performance.measure` detail, walking two levels into objects, typed
 *  arrays element by element. A per-publication GPU buffer passed as a prop
 *  once made each detail tens of megabytes and ran the page out of memory.
 *  Every scene page fails on a detail this long. */
export const MEASURE_ENTRIES_MAX = 5000;

/** In the page: report an oversized measure detail as a console error. */
export function guardMeasures(max) {
  const measure = performance.measure.bind(performance);
  performance.measure = (name, options) => {
    const entries = options?.detail?.devtools?.properties?.length ?? 0;
    if (entries > max)
      console.error(
        `oversized performance.measure detail: "${name}" logs ${entries} entries (${JSON.stringify(options.detail.devtools.properties.slice(0, 3))})`,
      );
    return measure(name, options);
  };
}

export function parseArgs(argv) {
  const ids = [];
  let list = false;
  for (const arg of argv) {
    if (arg === "--") continue;
    if (arg === "--list") list = true;
    else if (arg.startsWith("-")) throw new Error(`unknown flag ${arg}`);
    else ids.push(arg);
  }
  return { ids, list };
}

export async function loadRegistry() {
  const fixtures = JSON.parse(await readFile(REGISTRY, "utf8"));
  const files = (await readdir(SCENES_DIR)).filter((f) => f.endsWith(".mjs") && !f.startsWith("_"));
  const sceneIds = new Set(files.map((f) => f.slice(0, -4)));
  const fixtureIds = new Set(fixtures.map((f) => f.id));
  const missing = [...fixtureIds].filter((id) => !sceneIds.has(id));
  const orphan = [...sceneIds].filter((id) => !fixtureIds.has(id));
  if (missing.length || orphan.length) {
    throw new Error(
      `registry/scene mismatch: fixtures without scenes [${missing}], scenes without fixtures [${orphan}]`,
    );
  }
  return fixtures;
}

export function selectFixtures(fixtures, ids) {
  if (ids.length === 0) return fixtures;
  const unknown = ids.filter((id) => !fixtures.some((f) => f.id === id));
  if (unknown.length) {
    throw new Error(
      `unknown fixture id(s) ${unknown}. Known: ${fixtures.map((f) => f.id).join(", ")}`,
    );
  }
  return ids.map((id) => fixtures.find((f) => f.id === id));
}

export async function startServer(production = false) {
  if (process.env.VERIFY_URL) {
    // A timing verdict must not silently run on whatever VERIFY_URL serves.
    if (production) throw new Error("a production-build fixture cannot run against VERIFY_URL");
    return { url: process.env.VERIFY_URL, close: async () => {} };
  }
  if (production) {
    const { build, preview } = await import("vite");
    const outDir = new URL("../throwaway/lab-build/", HERE).pathname;
    const configFile = new URL("./vite.config.ts", HERE).pathname;
    // A dev server started earlier in this process sets NODE_ENV to
    // development, which would build development React.
    const env = process.env.NODE_ENV;
    process.env.NODE_ENV = "production";
    try {
      await build({
        configFile,
        root: HERE.pathname,
        logLevel: "error",
        build: { outDir, emptyOutDir: true },
      });
    } finally {
      if (env === undefined) delete process.env.NODE_ENV;
      else process.env.NODE_ENV = env;
    }
    const server = await preview({
      configFile,
      root: HERE.pathname,
      logLevel: "error",
      build: { outDir },
      preview: { port: 0, host: "127.0.0.1" },
    });
    const address = server.httpServer.address();
    return { url: `http://127.0.0.1:${address.port}`, close: () => server.close() };
  }
  const { createServer } = await import("vite");
  const server = await createServer({
    configFile: new URL("./vite.config.ts", HERE).pathname,
    root: HERE.pathname,
    logLevel: "error",
    server: { port: 0, host: "127.0.0.1" },
  });
  await server.listen();
  const address = server.httpServer.address();
  return { url: `http://127.0.0.1:${address.port}`, close: () => server.close() };
}

export async function run(fixtures) {
  const { chromium } = await import("playwright");
  const servers = new Map();
  const serverFor = async (fixture) => {
    const production = fixture.build === "production";
    if (!servers.has(production)) servers.set(production, await startServer(production));
    return servers.get(production);
  };
  const browser = await chromium.launch({
    channel: "chromium",
    args: [...WEBGPU_FLAGS, "--mute-audio"],
    ignoreDefaultArgs: ["--hide-scrollbars"],
  });
  const failures = [];
  const pageErrors = [];
  const seconds = [];
  try {
    for (const fixture of fixtures) {
      const started = performance.now();
      const scene = await import(new URL(`${fixture.id}.mjs`, SCENES_DIR));
      const evidenceDir = new URL(`${fixture.id}/`, EVIDENCE_DIR);
      await mkdir(evidenceDir, { recursive: true });
      let server;
      try {
        server = await serverFor(fixture);
      } catch (error) {
        console.log(`FAIL  ${fixture.id}: server did not start  (${error?.message ?? error})`);
        failures.push(`${fixture.id}: server did not start`);
        continue;
      }
      const ctx = {
        fixture,
        url: `${server.url}${fixture.route}`,
        browser: browser.version(),
        check(name, ok, detail) {
          console.log(
            `${ok ? "PASS" : "FAIL"}  ${fixture.id}: ${name}${detail ? `  (${detail})` : ""}`,
          );
          if (!ok) failures.push(`${fixture.id}: ${name}`);
        },
        async newPage({ viewport = { width: 1280, height: 800 }, allowErrors = false } = {}) {
          const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
          const page = await context.newPage();
          await page.addInitScript(guardMeasures, MEASURE_ENTRIES_MAX);
          page.on("crash", () => pageErrors.push(`${fixture.id}: the page crashed`));
          if (!allowErrors) {
            page.on("pageerror", (e) => pageErrors.push(`${fixture.id}: ${e.message}`));
            page.on("console", (m) => {
              if (m.type() === "error") pageErrors.push(`${fixture.id}: ${m.text()}`);
            });
          }
          return page;
        },
        async openLab(page, url = ctx.url, timeout = 30000) {
          await page.goto(url);
          await page.waitForFunction(
            () =>
              window.__lab?.ready ||
              window.__lab?.error ||
              document.querySelector("[data-testid=error]"),
            undefined,
            {
              timeout,
            },
          );
          let error = await page.evaluate(() => window.__lab?.error);
          if (!error && (await page.getByTestId("error").isVisible())) {
            const details = page.getByRole("button", { name: "Details", exact: true });
            if (await details.isVisible()) await details.click();
            error = await page.getByTestId("error").textContent();
            await page.screenshot({ path: ctx.evidencePath("startup-refusal.png") });
          }
          if (error) throw new Error(`lab failed: ${error}`);
          await page.evaluate(() => window.__lab.frame());
        },
        evidencePath: (name) => new URL(name, evidenceDir).pathname,
        async writeEvidence(name, data) {
          await writeFile(new URL(name, evidenceDir), JSON.stringify(data, null, 2));
        },
      };
      // No scene may hang the gate: a stalled page fails its fixture instead.
      let timer;
      const limit = Number(process.env.SCENE_TIMEOUT_S ?? fixture.timeout_s ?? 900);
      const timedOut = new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`scene exceeded ${limit} s`)), limit * 1000);
      });
      try {
        await Promise.race([scene.run(ctx), timedOut]);
      } catch (error) {
        ctx.check("scene completed without throwing", false, error?.stack ?? String(error));
      } finally {
        clearTimeout(timer);
        await Promise.allSettled(browser.contexts().map((c) => c.close()));
      }
      seconds.push([fixture.id, (performance.now() - started) / 1000]);
    }
  } finally {
    await browser.close();
    await Promise.allSettled([...servers.values()].map((server) => server.close()));
  }
  // Where the gate's time goes, slowest first.
  const slowest = seconds.sort((a, b) => b[1] - a[1]).slice(0, 8);
  console.log(
    `\nslowest scenes: ${slowest.map(([id, s]) => `${id} ${s.toFixed(0)} s`).join(", ")}`,
  );
  if (pageErrors.length) failures.push(`page errors: ${pageErrors.slice(0, 5).join(" | ")}`);
  console.log(
    failures.length
      ? `\n${failures.length} FAILURE(S)\n${failures.join("\n")}`
      : "\nALL CHECKS PASSED",
  );
  return failures.length ? 1 : 0;
}

export async function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const fixtures = await loadRegistry();
  if (args.list) {
    for (const f of fixtures) console.log(`${f.id}\t${f.route}\t${f.describe}`);
    return 0;
  }
  return run(selectFixtures(fixtures, args.ids));
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  main()
    .then((code) => process.exit(code))
    .catch((error) => {
      console.error(error.message);
      process.exit(2);
    });
}

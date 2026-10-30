// Deterministic browser verification for registered lab fixtures.
//
//   bun run --cwd web scene -- <fixture-id>...   run those fixtures' scenes
//   bun run --cwd web scene                      run every registered test scene
//   bun run --cwd web scene -- --list            list fixture ids and what each scene proves
//   bun run --cwd web scene -- --visual          run every scene with approved pictures
//
// Fixtures run two at a time (SCENE_CONCURRENCY sets the overlap, 1 serializes);
// a fixture marked `"serial"` always runs alone. A lab must become ready within
// 30 s alone or 60 s overlapped (SCENE_READINESS_TIMEOUT_MS overrides).
// SCENE_GRAPHICS_QUALITY=minimum|low|medium|high moves the scenes without
// approved pictures to that profile. Serial timing/parity fixtures keep their own.
// `bun run --cwd web verify` runs every registered scene; tools are skipped
// unless named explicitly. Scenes default to minimum, while scenes with an
// approved baseline directory use the player low profile.
// A fixture marked `"tool": true` is a lab page for humans and a runnable
// measurement, not a test; it is skipped without ids and runs by explicit id.
//
// The registry is apps/battle-lab/src/fixtures.json; every fixture id must have
// exactly one scene at web/scenes/<id>.mjs and vice versa. Without VERIFY_URL
// the runner starts its own Vite server; a fixture registered with
// `"build": "production"` (a timing verdict) runs against a production build
// served by Vite's preview instead. A scene longer than its fixture's
// `timeout_s` (900 by default; SCENE_TIMEOUT_S overrides every fixture's)
// fails. Evidence goes to throwaway/evidence/. A scene's approved pictures
// live in scenes/baselines/<fixture-id>/ (see scenes/_baseline.mjs);
// UPDATE_BASELINES=1, or =<fixture-id>,…, blesses this run's captures.
// The browser is browser.mjs's: WEBGPU_BROWSER chooses it.
//
//   UPDATE_BASELINES=panels bun run --cwd web scene -- panels
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { BASELINES, baselines } from "./scenes/_baseline.mjs";
import { launchBrowser } from "./browser.mjs";

const HERE = new URL(".", import.meta.url);
const REGISTRY = new URL("../apps/battle-lab/src/fixtures.json", HERE);
const SCENES_DIR = new URL("./scenes/", HERE);
const EVIDENCE_DIR = new URL("../throwaway/evidence/", HERE);
const ROOT = new URL("..", HERE).pathname;
const execFileAsync = promisify(execFile);

/** Errors that make a lab page unable to become ready.  Catching these at the
 * runner boundary avoids turning one adapter-limit failure into a chain of
 * readiness timeouts. Scenes that intentionally exercise validation pass
 * `allowErrors` and keep their existing assertions. */
export function fatalGpuMessage(message) {
  return /(?:WebGPU (?:uncaptured|validation|internal|out[- ]of[- ]memory) error|GPU(?:Validation|Internal|OutOfMemory)Error|GPUPipelineError|Invalid Texture|device lost|WebGPU is unavailable|requestAdapter\(\).*null|requestDevice.*failed)/i.test(
    message,
  );
}

const nativeTools = [
  {
    name: "mechanics validator",
    path: "debug/examples/mechanics_validate",
    build: "build:mechanics",
  },
  {
    name: "map workbench report",
    path: "release/examples/map_workbench_report",
    build: "build:map-workbench",
  },
];

export function nativeToolPath(relativePath) {
  const target = process.env.CARGO_TARGET_DIR ?? "target";
  return resolve(ROOT, target, relativePath);
}

/** Browser scenes may be run directly. Build the native fixture helpers every
 * run: cargo rebuilds them only when their sources changed (a no-op costs about
 * a tenth of a second), so a rules or catalog change never leaves a stale tool
 * to fail later as an opaque Vite `/__mechanics` 500. */
export async function ensureNativeTools() {
  for (const tool of nativeTools) {
    try {
      await execFileAsync("bun", ["run", tool.build], {
        cwd: ROOT,
        env: process.env,
        maxBuffer: 8 * 1024 * 1024,
      });
    } catch (error) {
      const detail = error?.stderr?.trim() || error?.message || String(error);
      throw new Error(`Could not prepare ${tool.name}. bun run ${tool.build} failed: ${detail}`);
    }
  }
}

/** React's development build logs each commit's changed props into a
 *  `performance.measure` detail, walking two levels into objects, typed
 *  arrays element by element. A per-publication GPU buffer passed as a prop
 *  once made each detail tens of megabytes and ran the page out of memory.
 *  Every scene page fails on a detail this long. */
const MEASURE_ENTRIES_MAX = 5000;

/** In the page: report an oversized measure detail as a console error. */
function guardMeasures(max) {
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
  let visual = false;
  for (const arg of argv) {
    if (arg === "--") continue;
    if (arg === "--list") list = true;
    else if (arg === "--visual") visual = true;
    else if (arg.startsWith("-")) throw new Error(`unknown flag ${arg}`);
    else ids.push(arg);
  }
  if (visual && ids.length) throw new Error("--visual selects its own fixtures; name none");
  return { ids, list, visual };
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

/** The fixtures whose scenes have approved pictures: the visual regression
 *  suite, `--visual`. */
export async function pinnedFixtures(fixtures) {
  const pinned = new Set(await readdir(BASELINES).catch(() => []));
  return fixtures.filter((f) => !f.tool && pinned.has(f.id));
}

export function selectFixtures(fixtures, ids) {
  if (ids.length === 0) return fixtures.filter((fixture) => !fixture.tool);
  const unknown = ids.filter((id) => !fixtures.some((f) => f.id === id));
  if (unknown.length) {
    throw new Error(
      `unknown fixture id(s) ${unknown}. Known: ${fixtures.map((f) => f.id).join(", ")}`,
    );
  }
  return ids.map((id) => fixtures.find((f) => f.id === id));
}

export function sceneUrlFor(url, quality) {
  const sceneUrl = new URL(url);
  if (quality && sceneUrl.pathname.startsWith("/labs/"))
    sceneUrl.searchParams.set("test-quality", quality);
  return sceneUrl;
}

export function mergeSceneUrl(url, params = {}) {
  const target = new URL(url);
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null) target.searchParams.delete(key);
    else target.searchParams.set(key, String(value));
  }
  return target.href;
}

/** Run independent scene jobs with bounded overlap while returning results in
 * input order. Browser contexts are isolated per scene, so this keeps the
 * verification gate moving without allowing a slow fixture to serialize the
 * whole suite or an unbounded burst to exhaust the GPU. */
export async function runConcurrent(items, limit, worker) {
  if (items.length === 0) return [];
  const workers = Math.min(items.length, Math.max(1, Math.floor(limit) || 1));
  const results = Array.from({ length: items.length });
  let next = 0;
  await Promise.all(
    Array.from({ length: workers }, async () => {
      while (true) {
        const index = next++;
        if (index >= items.length) return;
        results[index] = await worker(items[index], index);
      }
    }),
  );
  return results;
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

/** A page in a browser context of its own, closed with it. The context holds
 *  what the page stored: an incognito context keeps its Cache Storage (the
 *  appearance prefetch, every runtime asset) in the browser process, so a
 *  context left open after its page grows the browser until it crashes. */
export async function openScenePage(
  browser,
  { viewport = { width: 1280, height: 800 }, deviceScaleFactor = 1 } = {},
) {
  const context = await browser.newContext({ viewport, deviceScaleFactor });
  // The app declares an empty icon (index.html); a bare document a scene
  // opens (a JSON file, for its origin) has none, so the browser asks for
  // /favicon.ico itself, and its 404 would read as a page error.
  await context.route("**/favicon.ico", (route) => route.fulfill({ status: 204 }));
  const page = await context.newPage();
  page.close = () => context.close();
  return page;
}

/** Resolve the runner's quality contract from the baseline directory set: a
 * scene with approved pictures is judged at the player's low setting, whatever
 * the run asks for; SCENE_GRAPHICS_QUALITY only moves the other scenes off the
 * test-only minimum. */
export function fixtureGraphicsQuality(fixture, { pinned = false, env } = {}) {
  if (fixture.serial) return undefined;
  if (pinned) return "low";
  return env || process.env.SCENE_GRAPHICS_QUALITY || "minimum";
}

export async function run(fixtures, { pinnedIds = new Set() } = {}) {
  await ensureNativeTools();
  const servers = new Map();
  const serverFor = async (fixture) => {
    const production = fixture.build === "production";
    if (!servers.has(production)) {
      const starting = startServer(production).catch((error) => {
        servers.delete(production);
        throw error;
      });
      servers.set(production, starting);
    }
    return servers.get(production);
  };
  const browser = await launchBrowser({
    args: ["--mute-audio"],
    launch: { ignoreDefaultArgs: ["--hide-scrollbars"] },
  });
  const failures = [];
  const pageErrors = [];
  const pageErrorKeys = new Set();
  const recordPageError = (message) => {
    if (pageErrorKeys.has(message)) return;
    pageErrorKeys.add(message);
    pageErrors.push(message);
    console.log(`PAGE ERROR ${message}`);
  };
  const seconds = [];
  try {
    const concurrency = Number(process.env.SCENE_CONCURRENCY ?? 2);
    const readinessTimeout = Number(
      process.env.SCENE_READINESS_TIMEOUT_MS ?? (concurrency > 1 ? 60_000 : 30_000),
    );
    const runFixture = async (fixture) => {
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
        return;
      }
      let sceneFailure;
      const scenePages = new Set();
      let rejectSceneFailure;
      const sceneFailurePromise = new Promise((_, reject) => {
        rejectSceneFailure = reject;
      });
      const recordSceneFailure = (message) => {
        if (sceneFailure) return;
        sceneFailure = new Error(message);
        rejectSceneFailure(sceneFailure);
      };
      const quality = fixtureGraphicsQuality(fixture, { pinned: pinnedIds.has(fixture.id) });
      const sceneUrl = sceneUrlFor(`${server.url}${fixture.route}`, quality);
      const ctx = {
        fixture,
        url: sceneUrl.href,
        urlFor(params = {}) {
          return mergeSceneUrl(sceneUrl.href, params);
        },
        browser: browser.version(),
        readinessTimeout,
        graphicsQuality: quality,
        check(name, ok, detail) {
          console.log(
            `${ok ? "PASS" : "FAIL"}  ${fixture.id}: ${name}${detail ? `  (${detail})` : ""}`,
          );
          if (!ok) failures.push(`${fixture.id}: ${name}`);
        },
        async newPage({
          viewport = { width: 1280, height: 800 },
          deviceScaleFactor = 1,
          allowErrors = false,
        } = {}) {
          const page = await openScenePage(browser, { viewport, deviceScaleFactor });
          scenePages.add(page);
          page.setDefaultTimeout(readinessTimeout);
          await page.addInitScript(guardMeasures, MEASURE_ENTRIES_MAX);
          const failForGpu = (message) => {
            recordSceneFailure(`fatal WebGPU error: ${message}`);
            void page
              .evaluate(() => {
                console.error = () => {};
              })
              .catch(() => {});
            // Stop a poisoned page from flooding the terminal with follow-on
            // invalid-resource errors. The first error is already retained in
            // sceneFailure, so closing cannot hide the useful diagnosis.
            void page.close().catch(() => {});
          };
          page.on("crash", () => {
            const message = `${fixture.id}: the page crashed`;
            recordPageError(message);
            if (!allowErrors) recordSceneFailure(message);
          });
          if (!allowErrors) {
            page.on("pageerror", (e) => {
              recordPageError(`${fixture.id}: ${e.stack ?? e.message}`);
              if (fatalGpuMessage(e.message)) failForGpu(e.message);
            });
            page.on("console", (m) => {
              if (m.type() !== "error") return;
              const text = m.text();
              recordPageError(`${fixture.id}: ${text}`);
              if (fatalGpuMessage(text)) failForGpu(text);
            });
          }
          return page;
        },
        async openLab(page, url = ctx.url, timeout = readinessTimeout) {
          await page.goto(url);
          try {
            await Promise.race([
              page.waitForFunction(
                () =>
                  window.__lab?.ready ||
                  window.__lab?.error ||
                  document.querySelector("[data-testid=error]"),
                undefined,
                { timeout },
              ),
              sceneFailurePromise,
            ]);
          } catch (error) {
            if (/Timeout/i.test(error?.message ?? "")) {
              const state = await page
                .evaluate(() => ({
                  ready: Boolean(window.__lab?.ready),
                  error: window.__lab?.error ?? null,
                  visibleError: document.querySelector("[data-testid=error]")?.textContent ?? null,
                  document: document.readyState,
                }))
                .catch(() => null);
              throw new Error(
                `lab readiness timed out after ${timeout} ms (${JSON.stringify(state)})`,
                { cause: error },
              );
            }
            throw error;
          }
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
        /** Capture a page or locator and match it against its approved picture
         *  (`scenes/_baseline.mjs`). */
        matchBaseline: (target, name, options) => approved.match(target, name, options),
      };
      const approved = baselines(fixture.id, ctx);
      // No scene may hang the gate: a stalled page fails its fixture instead.
      let timer;
      const limit = Number(process.env.SCENE_TIMEOUT_S ?? fixture.timeout_s ?? 900);
      const timedOut = new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`scene exceeded ${limit} s`)), limit * 1000);
      });
      const scenePromise = scene.run(ctx);
      try {
        await Promise.race([scenePromise, timedOut, sceneFailurePromise]);
        await approved.finish();
      } catch (error) {
        let reportedError = error;
        if (sceneFailure) {
          // Preserve the first renderer/environment error while closing pages
          // to release any scene-owned waits. This avoids reporting only a
          // secondary "Target page ... closed" rejection.
          await Promise.allSettled([...scenePages].map((page) => page.close()));
          await Promise.allSettled([scenePromise]);
          reportedError = sceneFailure;
        }
        ctx.check(
          "scene completed without throwing",
          false,
          reportedError?.stack ?? String(reportedError),
        );
      } finally {
        clearTimeout(timer);
        await Promise.allSettled([...scenePages].map((page) => page.close()));
      }
      seconds.push([fixture.id, (performance.now() - started) / 1000]);
    };
    let parallel = [];
    for (const fixture of fixtures) {
      if (!fixture.serial) {
        parallel.push(fixture);
        continue;
      }
      await runConcurrent(parallel, concurrency, runFixture);
      parallel = [];
      await runFixture(fixture);
    }
    await runConcurrent(parallel, concurrency, runFixture);
  } finally {
    await browser.close();
    const startedServers = await Promise.allSettled(servers.values());
    await Promise.allSettled(
      startedServers
        .filter((result) => result.status === "fulfilled")
        .map((result) => result.value.close()),
    );
  }
  // Where the gate's time goes, slowest first.
  const slowest = seconds.sort((a, b) => b[1] - a[1]).slice(0, 8);
  console.log(
    `\nslowest scenes: ${slowest.map(([id, s]) => `${id} ${s.toFixed(0)} s`).join(", ")}`,
  );
  const failedFixtures = new Set(failures.map((failure) => failure.slice(0, failure.indexOf(":"))));
  const unreportedPageErrors = pageErrors.filter(
    (error) => !failedFixtures.has(error.slice(0, error.indexOf(":"))),
  );
  if (unreportedPageErrors.length)
    failures.push(`page errors: ${unreportedPageErrors.slice(0, 5).join(" | ")}`);
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
  const pinned = new Set((await pinnedFixtures(fixtures)).map((fixture) => fixture.id));
  const selected = args.visual
    ? fixtures.filter((fixture) => pinned.has(fixture.id))
    : selectFixtures(fixtures, args.ids);
  return run(selected, { pinnedIds: pinned });
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  main()
    .then((code) => process.exit(code))
    .catch((error) => {
      console.error(error.message);
      process.exit(2);
    });
}

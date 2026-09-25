// Deterministic browser verification for registered lab fixtures.
//
//   bun run --cwd web scene -- <fixture-id>...   run those fixtures' scenes
//   bun run --cwd web scene                      run every registered scene
//   bun run --cwd web scene -- --list            list fixture ids
//
// The registry is apps/battle-lab/src/fixtures.json; every fixture id must have
// exactly one scene at web/scenes/<id>.mjs and vice versa. Without VERIFY_URL
// the runner starts its own Vite server. Evidence goes to throwaway/evidence/.
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const HERE = new URL(".", import.meta.url);
const REGISTRY = new URL("../apps/battle-lab/src/fixtures.json", HERE);
const SCENES_DIR = new URL("./scenes/", HERE);
const EVIDENCE_DIR = new URL("../throwaway/evidence/", HERE);

export const WEBGPU_FLAGS = ["--enable-unsafe-webgpu", "--enable-features=WebGPU"];

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

async function startServer() {
  if (process.env.VERIFY_URL) return { url: process.env.VERIFY_URL, close: async () => {} };
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
  const server = await startServer();
  const browser = await chromium.launch({ channel: "chromium", args: WEBGPU_FLAGS });
  const failures = [];
  const pageErrors = [];
  try {
    for (const fixture of fixtures) {
      const scene = await import(new URL(`${fixture.id}.mjs`, SCENES_DIR));
      const evidenceDir = new URL(`${fixture.id}/`, EVIDENCE_DIR);
      await mkdir(evidenceDir, { recursive: true });
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
          if (!allowErrors) {
            page.on("pageerror", (e) => pageErrors.push(`${fixture.id}: ${e.message}`));
            page.on("console", (m) => {
              if (m.type() === "error") pageErrors.push(`${fixture.id}: ${m.text()}`);
            });
          }
          return page;
        },
        async openLab(page, url = ctx.url) {
          await page.goto(url);
          await page.waitForFunction(() => window.__lab?.ready || window.__lab?.error, undefined, {
            timeout: 30000,
          });
          const error = await page.evaluate(() => window.__lab.error);
          if (error) throw new Error(`lab failed: ${error}`);
          await page.evaluate(() => window.__lab.frame());
        },
        evidencePath: (name) => new URL(name, evidenceDir).pathname,
        async writeEvidence(name, data) {
          await writeFile(new URL(name, evidenceDir), JSON.stringify(data, null, 2));
        },
      };
      try {
        await scene.run(ctx);
      } catch (error) {
        ctx.check("scene completed without throwing", false, error?.stack ?? String(error));
      } finally {
        await Promise.allSettled(browser.contexts().map((c) => c.close()));
      }
    }
  } finally {
    await browser.close();
    await server.close();
  }
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

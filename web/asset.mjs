// The appearance asset CLI: `bun run --cwd web asset -- <command>`.
//
//   validate <glb> [--unit U] [--type T] [--yaw DEG] [--clips GLB] [--loop a,b] [--json]
//                          stats and findings for one GLB (catalog settings when it is a catalog source;
//                          a vehicle fitted to unit type T, or to every type that draws it)
//   bake                   bake the catalog into assets/runtime/<hash>/bundle.bin and assets/runtime/catalog.json
//   check                  re-bake in memory; fail if anything on disk is stale, missing or orphaned
//   pull [name...] [--sources]
//                          git lfs pull exactly the runtime bundles (and sources) of the named entries
//   blender <script.py> [args...]
//                          run a Blender script headless on the pinned Blender
//   sheet <appearance|glb> [--out DIR] [--accept] [--unit U] [--type T] [--yaw DEG] [--side blue|red]
//                          the workbench's contact sheet, strips, surface (close views
//                          and each texture channel's part), texture preview, stats and impostor
//                          atlas, rendered headless by the production renderer;
//                          --accept copies them to assets/review/<name>/
//   icons                  write the generated icons (assets/icons/): every weapon row's,
//                          every role's symbol and every unit type's silhouette
//   grass [name...]        write each generated grass kind's GLB from its catalog spec (then bake)
//   prototypes             write the prototype city set (its kit and templates.json): stand-in rows for
//                          every template of the physical catalogue no other set dresses (then bake)
//   catalogue              rewrite the physical template catalogue's rows from the city sets' descriptors
//
// bake and check pack the city sets into the template art library, which is
// judged by the simulation's own contract: build the WebAssembly first.
//
// Everything asset-specific lives in packages/scene-assets; this file is IO.

import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { registerHooks } from "node:module";
import { basename, dirname, join, relative, resolve } from "node:path";
import { parseArgs } from "node:util";

// packages/* own no node_modules: resolve their bare imports (the `math`
// package and its subpaths, such as `math/shapes`) from web/, as
// web/vite.config.ts's aliases and tsconfig's paths do for the browser.
registerHooks({
  resolve(specifier, context, next) {
    if (
      /^[a-z@]/.test(specifier) &&
      !specifier.startsWith("node:") &&
      context.parentURL?.includes("/packages/")
    )
      return next(specifier, { ...context, parentURL: import.meta.url });
    return next(specifier, context);
  },
});
const { bakeCatalog, runtimeCatalogText } = await import("../packages/scene-assets/src/bake.ts");
const { contentSha256, lfsPointerOid, lfsPullCommand } =
  await import("../packages/scene-assets/src/glb.ts");
const { bundlePath, templateLibraryPath } = await import("../packages/scene-assets/src/schema.ts");
const { TEMPLATE_TIER_TRIANGLES, catalogueRows, catalogueText, readTemplateSet } =
  await import("../packages/scene-assets/src/templateSource.ts");
const { PROTOTYPE_SET, prototypeKitGlb, prototypeTemplates, templateSetText } =
  await import("../packages/scene-assets/src/prototypeSet.ts");
const { hasErrors } = await import("../packages/scene-assets/src/validate.ts");
const { validateLoose } = await import("../packages/scene-assets/src/loose.ts");
const { fixtureAuthority } = await import("../packages/scene-assets/src/authority.ts");
const { UnitCatalog } = await import("../packages/scene-assets/src/units.ts");
const { grassClumpGlb } = await import("../packages/scene-assets/src/grass.ts");
const { iconFiles } = await import("../packages/scene-assets/src/icons.ts");
const { runtimeLookup, unitSolids } = await import("../packages/scene-assets/src/silhouette.ts");

const ROOT = resolve(dirname(new URL(import.meta.url).pathname), "..");
const CATALOG = join(ROOT, "assets/catalog.json");
const RUNTIME = join(ROOT, "assets/runtime");
const FIXTURE = join(ROOT, "fixtures/game.json");
const ICONS = join(ROOT, "assets/icons");
const UNIT_CATALOG = join(ROOT, "fixtures/catalog.json");
/** The physical template catalogue the map generator reads, which the city sets dress. */
const TEMPLATES = "fixtures/prototype-building-templates.json";
const BLENDER_VERSION = "5.2.1";
const BLENDER = process.env.BLENDER ?? "/Applications/Blender.app/Contents/MacOS/Blender";

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const catalog = () => readJson(CATALOG);
const authority = () =>
  fixtureAuthority(readJson(FIXTURE), new UnitCatalog(readJson(UNIT_CATALOG)));
const repoPath = (path) => relative(ROOT, resolve(path));
const readSource = async (path) => new Uint8Array(readFileSync(join(ROOT, path)));
/** The physical template contract, judged by the simulation's own code. Loaded
 *  on demand: only the template commands need the WebAssembly built. */
const physicalTemplates = async () => (await import("./src/maps/node.ts")).physicalTemplates();

function printFindings(findings) {
  for (const f of findings)
    console.log(
      `  ${f.severity === "error" ? "ERROR" : "warn "} ${f.code}: ${f.message}\n        fix: ${f.fix}`,
    );
}

function printStats(stats) {
  if (!stats) return;
  const kb = (n) => `${(n / 1024).toFixed(1)} KiB`;
  if (stats.tiers.length)
    console.log(`  triangles per tier: ${stats.tiers.map((t) => t.triangles).join(" / ")}`);
  if (stats.joints !== undefined) console.log(`  bones: ${stats.joints}`);
  if (stats.nodes !== undefined) console.log(`  nodes: ${stats.nodes}`);
  for (const module of stats.modules ?? [])
    console.log(`  module ${module.name}: ${module.triangles.join(" / ")} triangles`);
  if (stats.clips)
    console.log(
      `  clips: ${stats.clips.map((c) => `${c.name}${c.loop ? " (loop)" : ""} ${c.duration.toFixed(2)} s`).join(", ")}`,
    );
  console.log(`  source bytes: ${kb(stats.source_bytes)}`);
  if (stats.bounds)
    console.log(
      `  bounds: [${stats.bounds.min.map((v) => v.toFixed(3)).join(", ")}] – [${stats.bounds.max.map((v) => v.toFixed(3)).join(", ")}] m`,
    );
}

async function validate(args) {
  const { values, positionals } = parseArgs({
    args,
    allowPositionals: true,
    options: {
      unit: { type: "string" },
      type: { type: "string" },
      yaw: { type: "string" },
      clips: { type: "string" },
      loop: { type: "string" },
      json: { type: "boolean" },
    },
  });
  if (!positionals.length)
    throw new Error(
      "validate <glb> [--unit soldier|vehicle|building|scenery] [--type <unit type id>] [--yaw deg] [--clips glb] [--loop a,b]",
    );
  const context = { authority: authority(), tolerances: catalog().tolerances };
  let failed = false;
  for (const file of positionals) {
    const path = repoPath(file);
    const bytes = new Uint8Array(readFileSync(file));
    const oid = lfsPointerOid(bytes);
    if (oid) {
      console.log(`${path}: Git LFS pointer (sha256 ${oid}); run: ${lfsPullCommand(path)}`);
      failed = true;
      continue;
    }
    const result = await validateLoose(
      path,
      bytes,
      catalog(),
      context,
      {
        unit: values.unit,
        type: values.type,
        yaw: values.yaw !== undefined ? Number(values.yaw) : undefined,
        loops: values.loop?.split(","),
        clips: values.clips
          ? { path: repoPath(values.clips), bytes: new Uint8Array(readFileSync(values.clips)) }
          : undefined,
      },
      readSource,
    );
    if (result.clips) {
      console.log(
        result.appearance ? `${result.clips.path}: clips` : `${path}: skeleton ${result.clips.id}`,
      );
      if (values.json && !result.appearance)
        console.log(
          JSON.stringify({ stats: result.clips.stats, findings: result.clips.findings }, null, 2),
        );
      else {
        printStats(result.clips.stats);
        printFindings(result.clips.findings);
      }
      failed ||= hasErrors(result.clips.findings);
    }
    const judged = result.appearance;
    if (!judged) continue;
    console.log(
      `${path}: ${result.unit} (${result.kind}), basis yaw ${result.yaw}°${result.entryName ? `, catalog entry ${result.entryName}` : ""}`,
    );
    if (values.json)
      console.log(JSON.stringify({ stats: judged.stats, findings: judged.findings }, null, 2));
    else {
      printStats(judged.stats);
      printFindings(judged.findings);
      if (!judged.findings.length) console.log("  no findings");
    }
    failed ||= hasErrors(judged.findings);
  }
  return failed ? 1 : 0;
}

async function bakeAll() {
  const cat = catalog();
  const templates = Object.keys(cat.city_sets ?? {}).length
    ? { catalogue: readJson(join(ROOT, TEMPLATES)), physical: await physicalTemplates() }
    : undefined;
  return bakeCatalog(cat, readSource, { authority: authority(), templates });
}

function report(result) {
  for (const r of result.reports) {
    console.log(
      `${r.what} ${r.name}: ${r.hash ? `${r.hash} (${(r.bytes / 1024).toFixed(1)} KiB)` : "not baked"}`,
    );
    printStats(r.stats);
    if (r.templates) {
      const { templates, rows, modules } = r.templates;
      const standIns = templates.filter((t) => t.status === "prototype").length;
      console.log(
        `  ${templates.length} template(s), ${templates.length - standIns} release and ${standIns} prototype; ${rows} row(s) of ${modules} module(s)`,
      );
      for (const t of templates)
        console.log(
          `  ${t.id} (${t.set}, ${t.status}): ${t.rows} row(s), ${t.triangles.join(" / ")} triangles${
            t.triangles.some((n, tier) => n > TEMPLATE_TIER_TRIANGLES[tier])
              ? ` — over its budget of ${TEMPLATE_TIER_TRIANGLES.join(" / ")}`
              : ""
          }`,
        );
    }
    printFindings(r.findings);
  }
}

function runtimeHashDirs() {
  return existsSync(RUNTIME)
    ? readdirSync(RUNTIME, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => d.name)
    : [];
}

async function bake() {
  const result = await bakeAll();
  report(result);
  if (!result.ok) {
    console.log("bake failed; the runtime directory is unchanged");
    return 1;
  }
  mkdirSync(RUNTIME, { recursive: true });
  for (const [path, bytes] of result.files) {
    mkdirSync(dirname(join(RUNTIME, path)), { recursive: true });
    writeFileSync(join(RUNTIME, path), bytes);
  }
  const live = new Set([...result.files.keys()].map((p) => p.split("/")[0]));
  for (const dir of runtimeHashDirs())
    if (!live.has(dir)) rmSync(join(RUNTIME, dir), { recursive: true });
  writeFileSync(join(RUNTIME, "catalog.json"), runtimeCatalogText(result.runtime));
  console.log(`wrote ${result.files.size} runtime file(s) and assets/runtime/catalog.json`);
  return 0;
}

async function check() {
  const result = await bakeAll();
  const problems = [];
  for (const r of result.reports) {
    if (hasErrors(r.findings) || !r.hash) {
      problems.push(`${r.what} ${r.name} does not bake:`);
      printFindings(r.findings);
    }
  }
  const catalogText = runtimeCatalogText(result.runtime);
  const onDisk = existsSync(join(RUNTIME, "catalog.json"))
    ? readFileSync(join(RUNTIME, "catalog.json"), "utf8")
    : null;
  if (onDisk !== catalogText) problems.push("assets/runtime/catalog.json is stale; run bake");
  for (const [path, bytes] of result.files) {
    const file = join(RUNTIME, path);
    if (!existsSync(file)) {
      problems.push(`missing ${path}; run bake`);
      continue;
    }
    const disk = new Uint8Array(readFileSync(file));
    const hash = path.split("/")[0];
    if ((await contentSha256(disk)) !== hash)
      problems.push(`${path} does not hash to its name; run bake`);
    else if (lfsPointerOid(disk) === null && Buffer.compare(disk, bytes) !== 0)
      problems.push(`${path} differs from a fresh bake`);
  }
  const live = new Set([...result.files.keys()].map((p) => p.split("/")[0]));
  for (const dir of runtimeHashDirs())
    if (!live.has(dir)) problems.push(`orphan assets/runtime/${dir}; run bake`);
  let generated = null;
  try {
    generated = prototypeFiles();
  } catch (e) {
    problems.push(`the prototype city set cannot be generated: ${e.message}`);
  }
  for (const [path, bytes] of generated ?? []) {
    const file = join(ROOT, path);
    const stale =
      !existsSync(file) ||
      (await contentSha256(new Uint8Array(readFileSync(file)))) !== (await contentSha256(bytes));
    if (stale) problems.push(`${path} is missing or stale; run prototypes, then bake`);
  }
  const icons = generatedIcons();
  for (const [path, svg] of icons) {
    const file = join(ICONS, path);
    if (!existsSync(file) || readFileSync(file, "utf8") !== svg)
      problems.push(`assets/icons/${path} is missing or stale; run icons`);
  }
  for (const path of iconsOnDisk())
    if (!icons.has(path)) problems.push(`orphan assets/icons/${path}; run icons`);
  for (const p of problems) console.log(p);
  console.log(
    problems.length
      ? "check failed"
      : `check passed: ${result.files.size} runtime file(s) match the catalog`,
  );
  return problems.length ? 1 : 0;
}

function pull(args) {
  const { values, positionals } = parseArgs({
    args,
    allowPositionals: true,
    options: { sources: { type: "boolean" } },
  });
  const cat = catalog();
  const runtime = existsSync(join(RUNTIME, "catalog.json"))
    ? readJson(join(RUNTIME, "catalog.json"))
    : { skeletons: {}, appearances: {} };
  // A city set is pulled by its name: its kit, its templates.json and the
  // template art library every set is packed into.
  const sets = Object.entries(cat.city_sets ?? {}).filter(
    ([set, entry]) =>
      !positionals.length || positionals.includes(set) || positionals.includes(entry.kit),
  );
  const names = positionals.length
    ? positionals.map((name) => cat.city_sets?.[name]?.kit ?? name)
    : Object.keys(cat.appearances);
  const include = new Set();
  for (const [, entry] of sets) {
    if (runtime.templates)
      include.add(`assets/runtime/${templateLibraryPath(runtime.templates.library)}`);
    if (values.sources) include.add(entry.templates);
  }
  for (const name of names) {
    const entry = cat.appearances[name] ?? null;
    const skeleton = cat.skeletons[name] ? name : entry?.skeleton;
    if (!entry && !cat.skeletons[name]) throw new Error(`no catalog entry ${name}`);
    const hashes = [
      runtime.appearances[name]?.bundle,
      skeleton && runtime.skeletons[skeleton],
    ].filter(Boolean);
    for (const hash of hashes) include.add(`assets/runtime/${bundlePath(hash)}`);
    if (values.sources) {
      for (const path of [
        entry?.source,
        ...Object.values(entry?.states ?? {}),
        skeleton && cat.skeletons[skeleton]?.source,
      ])
        if (path) include.add(path);
    }
  }
  if (!include.size) {
    console.log("nothing to pull");
    return 0;
  }
  const command = lfsPullCommand([...include].sort().join(","));
  console.log(command);
  const run = spawnSync("git", ["lfs", "pull", `--include=${[...include].sort().join(",")}`], {
    cwd: ROOT,
    stdio: "inherit",
  });
  return run.status ?? 1;
}

function blender(args) {
  if (!args.length) throw new Error("blender <script.py> [args...]");
  if (!existsSync(BLENDER)) {
    console.log(
      `Blender not found at ${BLENDER}; install Blender ${BLENDER_VERSION} or set BLENDER`,
    );
    return 1;
  }
  const version =
    spawnSync(BLENDER, ["--version"], { encoding: "utf8" }).stdout?.split("\n")[0] ?? "";
  if (!version.includes(`Blender ${BLENDER_VERSION}`)) {
    console.log(
      `found "${version.trim()}"; asset scripts are pinned to Blender ${BLENDER_VERSION}`,
    );
    return 1;
  }
  const [script, ...rest] = args;
  const run = spawnSync(
    BLENDER,
    [
      "-b",
      "--factory-startup",
      "--python-exit-code",
      "1",
      "--python",
      resolve(script),
      "--",
      ...rest,
    ],
    { stdio: "inherit" },
  );
  return run.status ?? 1;
}

async function sheet(args) {
  const { values, positionals } = parseArgs({
    args,
    allowPositionals: true,
    options: {
      out: { type: "string" },
      accept: { type: "boolean" },
      unit: { type: "string" },
      type: { type: "string" },
      yaw: { type: "string" },
      side: { type: "string" },
    },
  });
  const [target] = positionals;
  if (!target)
    throw new Error(
      "sheet <appearance|glb> [--out DIR] [--accept] [--unit U] [--type T] [--yaw DEG] [--side blue|red]",
    );
  const file = target.endsWith(".glb") && existsSync(target) ? target : null;
  const side = values.side ?? "blue";
  if (side !== "blue" && side !== "red") throw new Error(`--side ${side}: blue or red`);
  const name =
    (file ? basename(file, ".glb") : target).replace(/[^\w.-]+/g, "_") +
    (side === "blue" ? "" : `-${side}`);
  const out = resolve(values.out ?? join(ROOT, "throwaway/sheets", name));
  const { startServer, WEBGPU_FLAGS } = await import("./scene.mjs");
  const { chromium } = await import("playwright");
  const { PNG } = await import("pngjs");
  const server = await startServer();
  const browser = await chromium.launch({ channel: "chromium", args: WEBGPU_FLAGS });
  try {
    const page = await browser.newPage({
      viewport: { width: 1280, height: 800 },
      deviceScaleFactor: 1,
    });
    page.on("pageerror", (e) => console.error(`page: ${e.message}`));
    await page.goto(`${server.url}/workbench`);
    await page.waitForFunction(() => window.__lab?.ready && window.__workbench, undefined, {
      timeout: 60000,
    });
    if (file)
      await page.evaluate(
        ([n, bytes, opts]) => window.__workbench.drop(n, new Uint8Array(bytes), opts),
        [
          basename(file),
          Array.from(readFileSync(file)),
          {
            ...(values.unit ? { unit: values.unit } : {}),
            ...(values.type ? { type: values.type } : {}),
            ...(values.yaw !== undefined ? { yaw: Number(values.yaw) } : {}),
          },
        ],
      );
    else await page.evaluate((n) => window.__workbench.select(n), target);
    await page.evaluate((s) => window.__workbench.setSide(s), side);
    const adapter = await page.evaluate(() => window.__lab.adapter);
    const impostor = await page.evaluate(() => window.__workbench.bakeImpostor());
    const result = await page.evaluate(() => window.__workbench.sheet());
    mkdirSync(out, { recursive: true });
    const png = (dataUrl) => Buffer.from(dataUrl.split(",")[1], "base64");
    writeFileSync(join(out, "contact.png"), png(result.contact));
    for (const strip of result.strips)
      writeFileSync(join(out, `strip-${strip.name}.png`), png(strip.png));
    writeFileSync(join(out, "surface.png"), png(result.surface));
    if (result.textures) writeFileSync(join(out, "textures.png"), png(result.textures));
    for (const layer of ["albedo", "normal"]) {
      const image = new PNG({ width: impostor.width, height: impostor.height });
      Buffer.from(impostor[layer], "base64").copy(image.data);
      writeFileSync(join(out, `impostor-${layer}.png`), PNG.sync.write(image));
      // For eyes: the same atlas over mid grey, since viewers show coverage 0 as white.
      const shown = new PNG({ width: impostor.width, height: impostor.height });
      for (let i = 0; i < image.data.length; i += 4) {
        const a = image.data[i + 3] / 255;
        for (let c = 0; c < 3; c++)
          shown.data[i + c] = Math.round(image.data[i + c] * a + 96 * (1 - a));
        shown.data[i + 3] = 255;
      }
      writeFileSync(join(out, `impostor-${layer}-on-grey.png`), PNG.sync.write(shown));
    }
    writeFileSync(
      join(out, "stats.json"),
      `${JSON.stringify({ ...result.stats, adapter }, null, 2)}\n`,
    );
    console.log(
      `wrote ${out} (contact, ${result.strips.length} strips, stats, impostor ${impostor.hash.slice(0, 12)})`,
    );
    const findings = await page.evaluate(() => window.__workbench.state().findings);
    for (const f of findings)
      console.log(`  ${f.severity === "error" ? "ERROR" : "warn "} ${f.code}: ${f.message}`);
    if (values.accept) {
      const review = join(ROOT, "assets/review", name);
      rmSync(review, { recursive: true, force: true });
      cpSync(out, review, { recursive: true });
      console.log(`accepted into ${relative(ROOT, review)}`);
    }
    return 0;
  } finally {
    await browser.close();
    await server.close();
  }
}

/** Generated grass kinds: each catalog entry with a `grass` spec gets its
 *  season state's GLB written from the spec. */
async function grass(names) {
  const cat = catalog();
  const entries = Object.entries(cat.appearances).filter(
    ([name, e]) => e.grass && (!names.length || names.includes(name)),
  );
  if (!entries.length) {
    console.log("no generated grass kinds in the catalog");
    return 1;
  }
  for (const [name, entry] of entries) {
    const path = Object.values(entry.states ?? {})[0];
    if (!path) throw new Error(`${name}: a grass kind needs its season's state`);
    const bytes = grassClumpGlb(name, entry.grass);
    mkdirSync(dirname(join(ROOT, path)), { recursive: true });
    writeFileSync(join(ROOT, path), bytes);
    console.log(`${path}: ${(bytes.byteLength / 1024).toFixed(1)} KiB`);
  }
  console.log("now run bake");
  return 0;
}

/** A city set's `templates.json` as it is on disk, read as a set. */
function readSet(name, entry) {
  const { set, findings } = readTemplateSet(
    new Uint8Array(readFileSync(join(ROOT, entry.templates))),
    entry.templates,
  );
  if (!set || findings.length) {
    printFindings(findings);
    throw new Error(`city set ${name} (${entry.templates}) does not read`);
  }
  return set;
}

/** The prototype set's two files by repo path, generated: stand-in rows for
 *  every template of the physical catalogue that no other set dresses, tinted
 *  by the fixture's prototype tints. Null when the catalog has no such set. */
function prototypeFiles() {
  const cat = catalog();
  const entry = cat.city_sets?.[PROTOTYPE_SET];
  const kit = entry && cat.appearances[entry.kit]?.source;
  if (!entry || !kit) return null;
  const dressed = new Set(
    Object.entries(cat.city_sets)
      .filter(([name]) => name !== PROTOTYPE_SET)
      .flatMap(([name, other]) => readSet(name, other).templates.map((t) => t.descriptor.id)),
  );
  const set = prototypeTemplates(
    readJson(join(ROOT, TEMPLATES)).filter((descriptor) => !dressed.has(descriptor.id)),
    readJson(FIXTURE).presentation.buildings.prototype_tints,
  );
  return new Map([
    [kit, prototypeKitGlb()],
    [entry.templates, new TextEncoder().encode(templateSetText(set))],
  ]);
}

async function prototypes() {
  const files = prototypeFiles();
  if (!files) {
    console.log(`the catalog has no "${PROTOTYPE_SET}" city set with a kit appearance`);
    return 1;
  }
  for (const [path, bytes] of files) {
    mkdirSync(dirname(join(ROOT, path)), { recursive: true });
    writeFileSync(join(ROOT, path), bytes);
    console.log(`${path}: ${(bytes.byteLength / 1024).toFixed(1)} KiB`);
  }
  console.log("now run bake");
  return 0;
}

/** The physical catalogue's rows, rewritten from the city sets' descriptors:
 *  what a Blender script's new templates reach the map generator through. */
async function catalogue() {
  const physical = await physicalTemplates();
  const sets = Object.entries(catalog().city_sets ?? {}).map(([name, entry]) =>
    readSet(name, entry),
  );
  const before = readFileSync(join(ROOT, TEMPLATES), "utf8");
  const rows = catalogueRows(sets, JSON.parse(before), physical);
  const text = catalogueText(rows);
  const hash = physical.complete(rows).hash;
  if (text === before) {
    console.log(`${TEMPLATES} is in step with the sets: ${rows.length} template(s), hash ${hash}`);
    return 0;
  }
  writeFileSync(join(ROOT, TEMPLATES), text);
  console.log(
    `${TEMPLATES}: ${rows.length} template(s); hash ${physical.complete(JSON.parse(before)).hash} → ${hash}`,
  );
  console.log(
    "the map generator builds from these rows: maps and their identities move with them. Now run prototypes, then bake",
  );
  return 0;
}

/** Every generated icon for the fixture's weapon rows and the unit catalog,
 *  each type's silhouette rendered from its baked model in assets/runtime. */
function generatedIcons() {
  const units = new UnitCatalog(readJson(UNIT_CATALOG));
  const lookup = runtimeLookup(
    readJson(join(RUNTIME, "catalog.json")),
    (path) => new Uint8Array(readFileSync(join(RUNTIME, path))),
  );
  return iconFiles(readJson(FIXTURE).weapons, units, (id) => unitSolids(units, id, lookup));
}

/** Every .svg under assets/icons/, by its path there. */
function iconsOnDisk() {
  return existsSync(ICONS)
    ? readdirSync(ICONS, { recursive: true })
        .filter((p) => p.endsWith(".svg"))
        .map((p) => p.split("\\").join("/"))
    : [];
}

async function icons() {
  const files = generatedIcons();
  for (const [path, svg] of files) {
    const file = join(ICONS, path);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, svg);
    console.log(`wrote assets/icons/${path}`);
  }
  for (const path of iconsOnDisk())
    if (!files.has(path)) {
      rmSync(join(ICONS, path));
      console.log(`removed assets/icons/${path}`);
    }
  return 0;
}

const [command, ...rest] = process.argv.slice(2);
const commands = {
  validate,
  bake,
  check,
  pull,
  blender,
  sheet,
  icons,
  grass,
  prototypes,
  catalogue,
};
if (!commands[command]) {
  console.log(`usage: asset ${Object.keys(commands).join(" | ")}`);
  process.exit(2);
}
process.exit(await commands[command](rest));

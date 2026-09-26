// The appearance asset CLI: `bun run --cwd web asset -- <command>`.
//
//   validate <glb> [--unit U] [--yaw DEG] [--clips GLB] [--loop a,b] [--json]
//                          stats and findings for one GLB (catalog settings when it is a catalog source)
//   bake                   bake the catalog into assets/runtime/<hash>/bundle.bin and assets/runtime/catalog.json
//   check                  re-bake in memory; fail if anything on disk is stale, missing or orphaned
//   provenance <file...>   content hash, manifest entry and licence of each file
//   pull [name...] [--sources]
//                          git lfs pull exactly the runtime bundles (and sources) of the named entries
//   blender <script.py> [args...]
//                          run a Blender script headless on the pinned Blender
//
// Everything asset-specific lives in packages/scene-assets; this file is IO.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
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
const { contentSha256, lfsPointerOid, lfsPullCommand, parseGlb } =
  await import("../packages/scene-assets/src/glb.ts");
const { bundlePath, UNIT_BUNDLE_KIND } = await import("../packages/scene-assets/src/schema.ts");
const { hasErrors, validateAppearance, validateProvenance, validateSkeleton } =
  await import("../packages/scene-assets/src/validate.ts");

const ROOT = resolve(dirname(new URL(import.meta.url).pathname), "..");
const CATALOG = join(ROOT, "assets/catalog.json");
const RUNTIME = join(ROOT, "assets/runtime");
const MANIFEST = join(ROOT, "specs/battle-look/assets/reuse-manifest.json");
const FIXTURE = join(ROOT, "fixtures/village.json");
const BLENDER_VERSION = "5.2.1";
const BLENDER = process.env.BLENDER ?? "/Applications/Blender.app/Contents/MacOS/Blender";

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const catalog = () => readJson(CATALOG);
const authority = () => readJson(FIXTURE).physics;
const provenance = () => readJson(MANIFEST).third_party;
const repoPath = (path) => relative(ROOT, resolve(path));
const readSource = async (path) => new Uint8Array(readFileSync(join(ROOT, path)));

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

function inferUnit(bytes) {
  const { json } = parseGlb(bytes);
  const names = new Set((json.nodes ?? []).map((n) => n.name));
  if ((json.skins ?? []).length) return "rifle";
  if (names.has("turret")) return "tank";
  if ([...names].some((n) => n?.startsWith("deploy_"))) return "supply";
  return "building";
}

async function validate(args) {
  const { values, positionals } = parseArgs({
    args,
    allowPositionals: true,
    options: {
      unit: { type: "string" },
      yaw: { type: "string" },
      clips: { type: "string" },
      loop: { type: "string" },
      json: { type: "boolean" },
    },
  });
  if (!positionals.length)
    throw new Error(
      "validate <glb> [--unit rifle|recon|at|tank|supply|building] [--yaw deg] [--clips glb] [--loop a,b]",
    );
  const cat = catalog();
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
    const named = Object.entries(cat.appearances).find(
      ([, e]) => e.source === path || Object.values(e.states ?? {}).includes(path),
    );
    const skeletonNamed = Object.entries(cat.skeletons).find(([, s]) => s.source === path);
    const context = {
      authority: authority(),
      tolerances: cat.tolerances,
      provenance: provenance(),
    };
    let result;
    if (skeletonNamed && !values.unit) {
      const [id, entry] = skeletonNamed;
      console.log(`${path}: skeleton ${id}`);
      result = await validateSkeleton(id, entry, bytes, context);
    } else {
      const unit = values.unit ?? named?.[1].unit ?? inferUnit(bytes);
      const kind = UNIT_BUNDLE_KIND[unit];
      if (!kind) throw new Error(`unknown unit ${unit}`);
      const yaw = values.yaw !== undefined ? Number(values.yaw) : (named?.[1].basis_yaw_deg ?? 0);
      const entry =
        named?.[1] ??
        (kind === "static"
          ? { unit, states: { intact: path }, basis_yaw_deg: yaw }
          : { unit, source: path, basis_yaw_deg: yaw });
      const files = { [path]: bytes };
      let skeleton;
      if (kind === "skinned") {
        const skeletonEntry = named?.[1].skeleton ? cat.skeletons[named[1].skeleton] : null;
        const clipsPath = values.clips ? repoPath(values.clips) : (skeletonEntry?.source ?? path);
        const clipBytes =
          clipsPath === path ? bytes : new Uint8Array(readFileSync(join(ROOT, clipsPath)));
        let declared = skeletonEntry && !values.clips ? skeletonEntry : null;
        if (!declared) {
          // Outside the catalog, loop flags come from --loop; every other clip is one-shot.
          const loops = values.loop?.split(",") ?? [];
          const names = (parseGlb(clipBytes).json.animations ?? []).map((a) => a.name);
          declared = {
            source: clipsPath,
            basis_yaw_deg: yaw,
            sample_hz: 30,
            aim_reference: { clip: "stand_aim", phase: 0 },
            clips: Object.fromEntries(names.map((n) => [n, { loop: loops.includes(n) }])),
          };
        }
        const clips = await validateSkeleton(
          named?.[1].skeleton ?? "adhoc",
          declared,
          clipBytes,
          {},
        );
        console.log(`${clipsPath}: clips`);
        printStats(clips.stats);
        printFindings(clips.findings);
        failed ||= hasErrors(clips.findings);
        if (!clips.built) continue;
        // Fit is measured on whatever clips built, so body findings print even when the clips have errors.
        skeleton = { clips: clips.built, aim_reference: declared.aim_reference };
      }
      console.log(
        `${path}: ${unit} (${kind}), basis yaw ${yaw}°${named ? `, catalog entry ${named[0]}` : ""}`,
      );
      result = await validateAppearance(
        { name: named?.[0] ?? path, entry, files, skeleton },
        context,
      );
    }
    if (values.json)
      console.log(JSON.stringify({ stats: result.stats, findings: result.findings }, null, 2));
    else {
      printStats(result.stats);
      printFindings(result.findings);
      if (!result.findings.length) console.log("  no findings");
    }
    failed ||= hasErrors(result.findings);
  }
  return failed ? 1 : 0;
}

async function bakeAll() {
  return bakeCatalog(catalog(), readSource, { authority: authority(), provenance: provenance() });
}

function report(result) {
  for (const r of result.reports) {
    console.log(
      `${r.what} ${r.name}: ${r.hash ? `${r.hash} (${(r.bytes / 1024).toFixed(1)} KiB)` : "not baked"}`,
    );
    printStats(r.stats);
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
  console.log(`wrote ${result.files.size} bundle(s) and assets/runtime/catalog.json`);
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
  for (const p of problems) console.log(p);
  console.log(
    problems.length
      ? "check failed"
      : `check passed: ${result.files.size} bundle(s) match the catalog`,
  );
  return problems.length ? 1 : 0;
}

async function provenanceCommand(files) {
  if (!files.length) throw new Error("provenance <file...>");
  let failed = false;
  for (const file of files) {
    const path = repoPath(file);
    const bytes = new Uint8Array(readFileSync(file));
    const hash = await contentSha256(bytes);
    const entry = provenance().find((e) => e.sha256 === hash);
    console.log(`${path}: sha256 ${hash}${lfsPointerOid(bytes) ? " (from its LFS pointer)" : ""}`);
    if (entry)
      console.log(
        `  manifest: ${entry.path}, licence ${entry.licence}, accepted by ${entry.accepted_by}`,
      );
    const findings = await validateProvenance(path, bytes, provenance());
    printFindings(findings);
    failed ||= hasErrors(findings);
  }
  return failed ? 1 : 0;
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
  const names = positionals.length ? positionals : Object.keys(cat.appearances);
  const include = new Set();
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

const [command, ...rest] = process.argv.slice(2);
const commands = { validate, bake, check, provenance: provenanceCommand, pull, blender };
if (!commands[command]) {
  console.log(`usage: asset ${Object.keys(commands).join(" | ")}`);
  process.exit(2);
}
process.exit(await commands[command](rest));

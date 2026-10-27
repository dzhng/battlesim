#!/usr/bin/env node

/**
 * Repeatable accounting for the one-renderer spike.
 *
 * This deliberately reports source surface and generated payload separately:
 * a small spike can otherwise look comparable to the production renderer just
 * because it does not implement production passes. Run from the repository
 * root. Add --write to save the JSON under throwaway evidence.
 */
import { readdir, readFile, stat, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const extensions = new Set([".mjs", ".rs", ".ts", ".tsx", ".wgsl"]);

const groups = {
  "production-browser-typescript": ["packages/battle-renderer/src"],
  "production-browser-wgsl": ["packages/battle-renderer/pipelines"],
  "renderer-core-typescript": ["packages/renderer-core/src"],
  "shared-rust-renderer-core": ["crates/renderer-core/src"],
  "shared-rust-browser-shell": ["crates/renderer-browser/src"],
  "native-renderer-modules": [
    "native/client/src/frame.rs",
    "native/client/src/world.rs",
    "native/client/src/ground.rs",
    "native/client/src/models.rs",
    "native/client/src/scenery.rs",
    "native/client/src/grass.rs",
    "native/client/src/shadow.rs",
    "native/client/src/environment.rs",
    "native/client/src/fog.rs",
    "native/client/src/fog_mask.rs",
    "native/client/src/effects.rs",
    "native/client/src/post.rs",
    "native/client/src/sky.rs",
    "native/client/src/statics.rs",
    "native/client/src/scars.rs",
  ],
  "native-platform-shell": [
    "native/client/src/main.rs",
    "native/client/src/gpu.rs",
    "native/client/src/pipelines.rs",
    "native/client/src/bundle.rs",
    "native/client/src/capture.rs",
    "native/client/src/bench.rs",
    "native/client/src/audio.rs",
  ],
  "spike-rust": ["crates/renderer-wasm-spike/src"],
  "spike-browser-typescript": [
    "apps/battle-lab/src/routes/oneRenderer.tsx",
    "apps/battle-lab/src/routes/oneRendererBattle.ts",
    "apps/battle-lab/src/routes/oneRendererModel.ts",
  ],
  "spike-scene": ["web/scenes/one-renderer.mjs"],
};

async function filesIn(target) {
  const full = path.join(root, target);
  const info = await stat(full);
  if (info.isFile()) return [full];
  const entries = await readdir(full, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const child = path.join(target, entry.name);
    if (entry.isDirectory()) files.push(...(await filesIn(child)));
    else if (extensions.has(path.extname(entry.name))) files.push(path.join(root, child));
  }
  return files;
}

function lineMetrics(source) {
  const lines = source.split(/\r?\n/);
  let nonblank = 0;
  let comments = 0;
  let blockComment = false;
  for (const raw of lines) {
    const line = raw.trim();
    if (!line) continue;
    nonblank++;
    if (blockComment) {
      comments++;
      if (line.includes("*/")) blockComment = false;
    } else if (line.startsWith("//") || line.startsWith("#") || line.startsWith("*")) {
      comments++;
    } else if (line.startsWith("/*")) {
      comments++;
      if (!line.includes("*/")) blockComment = true;
    }
  }
  return {
    lines: lines.length - 1,
    nonblank,
    comments,
    code: nonblank - comments,
    bytes: Buffer.byteLength(source),
  };
}

function constructMetrics(source, extension) {
  const names =
    extension === ".rs"
      ? {
          pipelines: /create_(?:render|compute)_pipeline\s*\(/g,
          renderPasses: /begin_render_pass\s*\(/g,
          computePasses: /begin_compute_pass\s*\(/g,
          textures: /create_texture\s*\(/g,
          buffers: /create_buffer\s*\(/g,
          bindGroups: /create_bind_group\s*\(/g,
        }
      : {
          pipelines: /create(?:Render|Compute)Pipeline\s*\(/g,
          renderPasses: /beginRenderPass\s*\(/g,
          computePasses: /beginComputePass\s*\(/g,
          textures: /createTexture\s*\(/g,
          buffers: /createBuffer\s*\(/g,
          bindGroups: /createBindGroup\s*\(/g,
        };
  return Object.fromEntries(
    Object.entries(names).map(([name, pattern]) => [name, source.match(pattern)?.length ?? 0]),
  );
}

async function sourceGroup(roots) {
  const files = (await Promise.all(roots.map(filesIn))).flat().sort();
  const totals = { files: files.length, lines: 0, nonblank: 0, comments: 0, code: 0, bytes: 0 };
  const constructs = {
    pipelines: 0,
    renderPasses: 0,
    computePasses: 0,
    textures: 0,
    buffers: 0,
    bindGroups: 0,
  };
  for (const file of files) {
    const contents = await readFile(file, "utf8");
    const metrics = lineMetrics(contents);
    for (const key of Object.keys(totals).slice(1)) totals[key] += metrics[key];
    const counts = constructMetrics(contents, path.extname(file));
    for (const key of Object.keys(constructs)) constructs[key] += counts[key];
  }
  return { ...totals, constructs };
}

async function artifact(globPrefix, suffix = null) {
  const directory = path.dirname(path.join(root, globPrefix));
  const prefix = path.basename(globPrefix);
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch {
    return [];
  }
  const result = [];
  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.startsWith(prefix)) continue;
    if (suffix && !entry.name.endsWith(suffix)) continue;
    const file = path.join(directory, entry.name);
    result.push({ path: path.relative(root, file), bytes: (await stat(file)).size });
  }
  return result.sort((a, b) => a.path.localeCompare(b.path));
}

async function jsonFile(relative) {
  try {
    return JSON.parse(await readFile(path.join(root, relative), "utf8"));
  } catch {
    return null;
  }
}

const source = {};
for (const [name, roots] of Object.entries(groups)) source[name] = await sourceGroup(roots);

const result = await jsonFile("throwaway/evidence/one-renderer/result.json");
const visual = await jsonFile("throwaway/evidence/one-renderer/visual.json");
const report = {
  generatedAt: new Date().toISOString(),
  definitions: {
    lines: "physical lines including blank lines",
    nonblank: "non-empty physical lines",
    code: "non-empty lines excluding full-line comments",
    bytes: "UTF-8 source bytes or raw generated artifact bytes",
    runtime: "the latest scene result.json when present; queue drain is a submission/compositor boundary, not GPU timestamp time",
  },
  source,
  generatedArtifacts: {
    wasmSpike: await artifact("web/src/wasm/renderer_spike/renderer_wasm_spike_bg.wasm", ".wasm"),
    wasmSpikeJs: await artifact("web/src/wasm/renderer_spike/renderer_wasm_spike.js", ".js"),
    wasmShared: await artifact("web/src/wasm/renderer_browser/renderer_browser_bg.wasm", ".wasm"),
    wasmSharedJs: await artifact("web/src/wasm/renderer_browser/renderer_browser.js", ".js"),
    routeChunk: await artifact("web/dist/assets/oneRenderer-"),
  },
  runtime: result
    ? {
        rows: result,
        visualDiff: visual,
      }
    : null,
  ownership: [
    {
      area: "canvas/device/surface",
      browser: "BattleFrame caller + TypeGPU root/device",
      rust: "renderer-browser WASM shell over renderer-core",
      status: "shared target/pipeline/pass seam runs in browser; surface shell remains platform-specific",
    },
    {
      area: "terrain and authored tank",
      browser: "worldPass/modelLayer and oneRendererModel.ts",
      rust: "renderer-browser shared asset parser/upload/indexed draw; renderer-wasm-spike remains transport probe",
      status: "shared browser core parses, uploads, and draws the real tank through renderer-core; production material parity remains open",
    },
    {
      area: "reverse-Z depth",
      browser: "SizedTargets / worldDepth / BattleFrame",
      rust: "renderer-core::depth::DepthPrepass called by both native and renderer-browser",
      status: "shared depth pass executes in browser WASM fixture; native adapter compiles",
    },
    {
      area: "shadows, fog, effects, post, overlays",
      browser: "BattleFrame world/fog/effects/post/overlay owners",
      rust: "native renderer modules, moving into shared core",
      status: "depth pass shared; world/material and remaining pass owners open",
    },
    {
      area: "resource registry/generation/reset/device loss",
      browser: "GpuRegistry + SizedTargets + frame disposal",
      rust: "ad hoc renderer-owned resources",
      status: "full lifecycle parity open",
    },
  ],
};

const output = JSON.stringify(report, null, 2) + "\n";
if (process.argv.includes("--write")) {
  const directory = path.join(root, "throwaway/evidence/one-renderer");
  await mkdir(directory, { recursive: true });
  await writeFile(path.join(directory, "accounting.json"), output);
}
process.stdout.write(output);

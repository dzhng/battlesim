#!/usr/bin/env node

/**
 * Renderer source and generated artifact accounting.
 *
 * This deliberately reports source surface and generated payload separately:
 * generated shader expansion must not disguise the maintained authoring cost.
 * Run from the repository root; --write saves ignored evidence.
 */
import { readdir, readFile, stat, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { gzipSync, brotliCompressSync } from "node:zlib";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const distFlag = process.argv.indexOf("--dist");
const dist = distFlag < 0 ? "web/dist" : process.argv[distFlag + 1];
if (!dist) throw new Error("--dist needs a build directory");
const extensions = new Set([".mjs", ".rs", ".ts", ".tsx", ".wgsl"]);

const groups = {
  "browser-cpu-contracts": ["packages/battle-renderer/src"],
  "browser-camera-primitives": ["packages/renderer-core/src"],
  "shared-rust-renderer": ["crates/renderer-core/src", "crates/renderer-core/build.rs"],
  "shared-rust-browser-shell": ["crates/renderer-browser/src"],
  "browser-platform-transport": ["web/src/battle/render"],
  "native-client": ["native/client/src"],
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
    } else if (line.startsWith("//") || (line.startsWith("#!") && !line.startsWith("#!["))) {
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

async function sourceGroup(roots, include = () => true) {
  const files = (await Promise.all(roots.map(filesIn))).flat().filter(include).sort();
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
  const directory = globPrefix.endsWith("/")
    ? path.join(root, globPrefix)
    : path.dirname(path.join(root, globPrefix));
  const prefix = globPrefix.endsWith("/") ? "" : path.basename(globPrefix);
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
    const bytes = await readFile(file);
    result.push({
      path: path.relative(root, file),
      bytes: bytes.length,
      gzip: gzipSync(bytes).length,
      brotli: brotliCompressSync(bytes).length,
    });
  }
  return result.sort((a, b) => a.path.localeCompare(b.path));
}

const source = {};
for (const [name, roots] of Object.entries(groups)) {
  source[name] = await sourceGroup(
    roots,
    (file) => !file.endsWith(".wgsl") && !file.endsWith(".gen.ts"),
  );
}
source["authored-wgsl"] = await sourceGroup(["crates/renderer-core/src"], (file) =>
  file.endsWith(".wgsl"),
);
source["shader-composition"] = await sourceGroup([
  "crates/renderer-core/src/shaders/pipeline_sources.txt",
]);
source["generated-record-views"] = await sourceGroup([
  "packages/battle-renderer/src/records.gen.ts",
]);

const report = {
  generatedAt: new Date().toISOString(),
  definitions: {
    lines: "physical lines including blank lines",
    nonblank: "non-empty physical lines",
    code: "non-empty lines excluding full-line comments",
    bytes: "UTF-8 source bytes or raw generated artifact bytes",
    compressedBytes:
      "individually compressed artifacts; actual route transfer also depends on requested resources and HTTP content encoding",
    nativeClient: "includes replay/benchmark/audio support; not solely the platform GPU shell",
  },
  source,
  generatedArtifacts: {
    wasmShared: await artifact("web/src/wasm/renderer_browser/renderer_browser_bg.wasm", ".wasm"),
    wasmSharedJs: await artifact("web/src/wasm/renderer_browser/renderer_browser.js", ".js"),
    simulationWasm: await artifact("web/src/wasm/game_wasm_bg.wasm", ".wasm"),
    javascript: await artifact(`${dist}/assets/`, ".js"),
  },
};

const output = JSON.stringify(report, null, 2) + "\n";
if (process.argv.includes("--write")) {
  const directory = path.join(root, "throwaway/evidence/renderer");
  await mkdir(directory, { recursive: true });
  await writeFile(path.join(directory, "accounting.json"), output);
}
process.stdout.write(output);

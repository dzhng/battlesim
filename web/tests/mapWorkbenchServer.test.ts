// @vitest-environment node
import { mkdtemp, mkdir, writeFile, readFile, readdir, chmod, rm } from "node:fs/promises";
import { createServer as createViteServer, type ViteDevServer } from "vite";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join } from "node:path";
import * as filesystem from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { afterEach, expect, test, vi } from "vitest";
import {
  WorkbenchStore,
  nativeReporter,
  mapWorkbenchPlugin,
  type NativeReporter,
} from "../../apps/map-workbench/server";
import type { NativeValidation } from "../../apps/map-workbench/src/protocol";

vi.mock("node:fs/promises", async (original) => ({
  ...(await original<typeof import("node:fs/promises")>()),
  rename: vi.fn((await original<typeof import("node:fs/promises")>()).rename),
}));
const roots: string[] = [];
const stores: WorkbenchStore[] = [];
const servers: ViteDevServer[] = [];
afterEach(async () => {
  vi.restoreAllMocks();
  vi.mocked(filesystem.rename).mockImplementation(
    (await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises")).rename,
  );
  await Promise.all(servers.splice(0).map((server) => server.close()));
  await Promise.all(stores.splice(0).map((store) => store.close()));
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});
const validation: NativeValidation = {
  status: "valid",
  diagnostics: [],
  fields: [
    {
      id: "density",
      document: "presets",
      path: ["density"],
      group: "towns",
      label: "Density",
      description: "Building density",
      unit: "share",
      role: "construction",
      editable: true,
    },
    {
      id: "analysis.target",
      document: "defaults",
      path: ["analysis", "target"],
      group: "analysis",
      label: "Target",
      description: "Report target",
      unit: "share",
      role: "analysis",
      editable: true,
    },
  ],
};
const reporter: NativeReporter = async (request) => {
  if (request.operation === "validate") return validation;
  throw new Error("Unexpected operation");
};
async function fixture(run: NativeReporter = reporter) {
  const root = await mkdtemp(join(tmpdir(), "map-workbench-"));
  roots.push(root);
  await mkdir(join(root, "fixtures"));
  const files: Record<string, unknown> = {
    "map-presets.json": { revision: "initial", density: 0.5 },
    "generated-battle.json": { analysis: { target: 0.5 } },
    "prototype-building-templates.json": [],
    "game.json": { seed: 1 },
    "catalog.json": { units: [], props: [] },
    "encounters.json": { recipes: {} },
  };
  for (const [name, value] of Object.entries(files))
    await writeFile(join(root, "fixtures", name), JSON.stringify(value));
  const store = new WorkbenchStore(root, run);
  stores.push(store);
  return { root, store };
}

test("preview is read-only and save publishes the exact reviewed source replacements", async () => {
  const { root, store } = await fixture();
  const initial = await store.snapshot();
  const draft = structuredClone(initial);
  draft.documents.presets.density = 0.6;
  const reviewed = await store.preview(draft);
  expect(await readFile(join(root, "fixtures/map-presets.json"), "utf8")).toBe(
    '{"revision":"initial","density":0.5}',
  );
  await store.save(reviewed);
  const replacement = reviewed.files.find((file) => file.path === "fixtures/map-presets.json")!;
  expect(await readFile(join(root, replacement.path), "utf8")).toBe(replacement.after);
  expect(JSON.parse(replacement.after)).toMatchObject({ density: 0.6 });
  expect(JSON.parse(replacement.after).revision).not.toBe("initial");
});

test("unknown or read-only draft fields cannot be previewed or generated", async () => {
  const { store } = await fixture();
  const initial = await store.snapshot();
  const unknown = structuredClone(initial);
  unknown.documents.defaults.camera = { overview_pitch: 2 };
  await expect(store.preview(unknown)).rejects.toMatchObject({ status: 400 });
  const managed = structuredClone(initial);
  managed.documents.presets.revision = "forged";
  await expect(store.preview(managed)).rejects.toMatchObject({ status: 400 });
});

test("analysis-only edits and no-op retry preserve the authored preset bytes and revision", async () => {
  const { root, store } = await fixture();
  const initial = await store.snapshot();
  const presets = await readFile(join(root, "fixtures/map-presets.json"), "utf8");
  const draft = structuredClone(initial);
  draft.documents.defaults.analysis = { target: 0.6 };
  const review = await store.preview(draft);
  expect(review.files.map((file) => file.path)).toEqual(["fixtures/generated-battle.json"]);
  const saved = await store.save(review);
  expect(await store.save(review)).toEqual(saved);
  expect(await readFile(join(root, "fixtures/map-presets.json"), "utf8")).toBe(presets);
  const noOp = await store.preview(saved);
  expect(noOp.files).toEqual([]);
  expect(await store.save(noOp)).toEqual(saved);
});

test("retained refused reports export the exact inputs and remain saveable when settings are valid", async () => {
  const refused: NativeReporter = async (request) =>
    request.operation === "validate"
      ? validation
      : {
          status: "refused",
          choice: { type: "mixed", size: "small", seed: "4" },
          diagnostics: [
            {
              code: "generation_failed",
              feature: "towns",
              location: "$.presets",
              message: "No layout fits this seed",
            },
          ],
        };
  const { root, store } = await fixture(refused);
  const initial = await store.snapshot();
  const draft = structuredClone(initial);
  draft.documents.presets.density = 0.7;
  const run = await store.generate({
    purpose: "preview",
    draft,
    choice: { type: "mixed", size: "small", seed: "4" },
  });
  expect(run.status).toBe("refused");
  expect(run.diagnostics[0].message).toBe("No layout fits this seed");
  const exported = await store.export(run.artifactId!);
  expect(exported.report).toEqual(run);
  expect(JSON.parse((exported.inputs as Record<string, string>).presets)).toMatchObject({
    density: 0.7,
  });
  const review = await store.preview(draft);
  await store.save(review);
  expect(JSON.parse(await readFile(join(root, "fixtures/map-presets.json"), "utf8"))).toMatchObject(
    { density: 0.7 },
  );
});

test("a changed read-only receipt makes a reviewed save stale without touching draft sources", async () => {
  const { root, store } = await fixture();
  const draft = structuredClone(await store.snapshot());
  draft.documents.presets.density = 0.8;
  const reviewed = await store.preview(draft);
  const original = await readFile(join(root, "fixtures/map-presets.json"), "utf8");
  const outside = '{"seed":2}\n';
  await writeFile(join(root, "fixtures/game.json"), outside);
  await expect(store.save(reviewed)).rejects.toMatchObject({ status: 409 });
  expect(await readFile(join(root, "fixtures/game.json"), "utf8")).toBe(outside);
  expect(await readFile(join(root, "fixtures/map-presets.json"), "utf8")).toBe(original);
});

test("the HTTP plugin serves only local same-origin operations and rejects browser paths", async () => {
  const { root } = await fixture();
  const server = await createViteServer({
    configFile: false,
    plugins: [mapWorkbenchPlugin(root, reporter)],
    server: { host: "127.0.0.1", port: 0 },
  });
  servers.push(server);
  await server.listen();
  const address = server.httpServer!.address();
  if (!address || typeof address === "string") throw new Error("No server port");
  const origin = `http://127.0.0.1:${address.port}`;
  const initial = await fetch(`${origin}/__map-workbench/snapshot`).then((response) =>
    response.json(),
  );
  const wrongOrigin = await fetch(`${origin}/__map-workbench/preview`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Origin: "https://outside.example" },
    body: JSON.stringify(initial),
  });
  expect(wrongOrigin.status).toBe(403);
  const path = await fetch(`${origin}/__map-workbench/inspect`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Origin: origin },
    body: JSON.stringify({ artifactDir: root }),
  });
  expect(path.status).toBe(400);
  const preview = await fetch(`${origin}/__map-workbench/preview`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Origin: origin },
    body: JSON.stringify(initial),
  });
  expect(preview.status).toBe(200);
  expect((await preview.json()).files).toEqual([]);
});

test("aborting native work terminates its actual process before resolving cancellation", async () => {
  const { root } = await fixture();
  const binary = join(root, "throwaway/target/release/examples/map_workbench_report");
  await mkdir(join(root, "throwaway/target/release/examples"), { recursive: true });
  await writeFile(
    binary,
    `#!/usr/bin/env node
require("node:fs").writeFileSync(${JSON.stringify(join(root, "pid"))}, String(process.pid));
setInterval(() => {}, 1000);
`,
  );
  await chmod(binary, 0o755);
  const controller = new AbortController();
  const pending = nativeReporter(root)(
    {
      operation: "validate",
      inputs: {
        presets: "{}",
        defaults: "{}",
        rules: "{}",
        catalog: "{}",
        recipes: "{}",
        templates: "[]",
      },
    },
    controller.signal,
  );
  let pid: number | null = null;
  try {
    await expect
      .poll(async () => {
        pid = Number(await readFile(join(root, "pid"), "utf8").catch(() => "0")) || null;
        return pid;
      })
      .not.toBeNull();
    process.kill(pid!, 0);
    controller.abort();
    await expect(pending).rejects.toMatchObject({ status: 499 });
    expect(() => process.kill(pid!, 0)).toThrow();
  } finally {
    if (pid) {
      try {
        process.kill(pid, "SIGKILL");
      } catch {}
    }
  }
});

test("retained artifact receipts describe tested draft bytes, and superseded drafts release their artifacts", async () => {
  const run: NativeReporter = async (request) =>
    request.operation === "validate"
      ? validation
      : {
          status: "ok",
          choice: { type: "mixed", size: "small", seed: "4" },
          diagnostics: [],
          svg: "<svg/>",
        };
  const { root, store } = await fixture(run);
  const initial = await store.snapshot();
  const baseline = await store.generate({
    purpose: "preview",
    draft: initial,
    choice: { type: "mixed", size: "small", seed: "4" },
  });
  const draft = structuredClone(initial);
  draft.documents.presets.density = 0.6;
  const first = await store.generate({ purpose: "preview", draft, choice: baseline.choice });
  const exported = await store.export(first.artifactId!);
  const input = (exported.inputs as Record<string, string>).presets;
  expect((exported.receipts as Record<string, string>).presets).toBe(
    createHash("sha256").update(input).digest("hex"),
  );
  draft.documents.presets.density = 0.7;
  const latest = await store.generate({ purpose: "preview", draft, choice: baseline.choice });
  await expect(store.export(first.artifactId!)).rejects.toMatchObject({ status: 410 });
  expect((await store.export(baseline.artifactId!)).report).toEqual(baseline);
  expect((await store.export(latest.artifactId!)).report).toEqual(latest);
  expect((await readdir(join(root, "throwaway/map-workbench"))).sort()).toEqual(
    [baseline.artifactId, latest.artifactId].sort(),
  );
  await store.close();
  expect(await readdir(join(root, "throwaway/map-workbench"))).toEqual([]);
});

test("outside edits during publication survive while the editor rolls back its own replacements", async () => {
  const { root, store } = await fixture();
  const initial = await store.snapshot();
  const draft = structuredClone(initial);
  draft.documents.presets.density = 0.7;
  draft.documents.defaults.analysis = { target: 0.6 };
  const review = await store.preview(draft);
  const outside = '{"analysis":{"target":0.9}}\n';
  const rename = (await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises"))
    .rename;
  vi.mocked(filesystem.rename).mockImplementation(async (from, to) => {
    await rename(from, to);
    if (String(to) === join(root, "fixtures/map-presets.json"))
      await writeFile(join(root, "fixtures/generated-battle.json"), outside);
  });
  await expect(store.save(review)).rejects.toMatchObject({ status: 409 });
  expect(await readFile(join(root, "fixtures/generated-battle.json"), "utf8")).toBe(outside);
  expect(await readFile(join(root, "fixtures/map-presets.json"), "utf8")).toBe(
    review.files.find((file) => file.path.endsWith("map-presets.json"))!.before,
  );
});

test("a refused latest draft keeps the last admitted plan available for inspection", async () => {
  const run: NativeReporter = async (request) => {
    if (request.operation === "validate") return validation;
    if (request.operation === "inspect") return { svg: "<svg>older admitted</svg>", features: [] };
    if (request.operation !== "generate") throw new Error("Unexpected sight");
    const status = JSON.parse(request.inputs.presets).density === 0.9 ? "refused" : "ok";
    return {
      status,
      choice: request.choice,
      diagnostics: [],
      svg: status === "ok" ? "<svg/>" : undefined,
    };
  };
  const { root, store } = await fixture(run);
  const draft = structuredClone(await store.snapshot());
  draft.documents.presets.density = 0.6;
  const accepted = await store.generate({
    purpose: "preview",
    draft,
    choice: { type: "mixed", size: "small", seed: "4" },
  });
  draft.documents.presets.density = 0.9;
  const refused = await store.generate({ purpose: "preview", draft, choice: accepted.choice });
  expect(refused.status).toBe("refused");
  expect(await store.inspect(accepted.artifactId!)).toMatchObject({
    svg: "<svg>older admitted</svg>",
  });
  expect((await store.export(refused.artifactId!)).report).toEqual(refused);
  expect(await readdir(join(root, "throwaway/map-workbench"))).toEqual([accepted.artifactId]);
});

test("the next reader recovers an interrupted source publication before exposing a snapshot", async () => {
  const { root, store } = await fixture();
  const initial = await store.snapshot();
  const draft = structuredClone(initial);
  draft.documents.presets.density = 0.6;
  draft.documents.defaults.analysis = { target: 0.7 };
  const review = await store.preview(draft);
  const departed = spawnSync(process.execPath, ["-e", ""]);
  expect(departed.status).toBe(0);
  await writeFile(
    join(root, "throwaway/map-workbench-transaction.json"),
    JSON.stringify({ pid: departed.pid, files: review.files }),
  );
  await writeFile(join(root, review.files[0].path), review.files[0].after);
  const recovered = new WorkbenchStore(root, reporter);
  stores.push(recovered);
  expect(await recovered.snapshot()).toEqual(initial);
});

test("disconnecting an HTTP generation terminates the process and removes its unpublished artifact", async () => {
  const { root, store } = await fixture();
  const draft = await store.snapshot();
  const binary = join(root, "throwaway/target/release/examples/map_workbench_report");
  await mkdir(join(root, "throwaway/target/release/examples"), { recursive: true });
  await writeFile(
    binary,
    `#!/usr/bin/env node\nlet input = ""; process.stdin.on("data", data => input += data); process.stdin.on("end", () => { const request = JSON.parse(input); if (request.operation === "validate") { process.stdout.write(${JSON.stringify(JSON.stringify(validation))}); } else { require("node:fs").writeFileSync(${JSON.stringify(join(root, "http-pid"))}, String(process.pid)); setInterval(() => {}, 1000); } });\n`,
  );
  await chmod(binary, 0o755);
  const server = await createViteServer({
    configFile: false,
    plugins: [mapWorkbenchPlugin(root)],
    server: { host: "127.0.0.1", port: 0 },
  });
  servers.push(server);
  await server.listen();
  const address = server.httpServer!.address();
  if (!address || typeof address === "string") throw new Error("No server port");
  const origin = `http://127.0.0.1:${address.port}`;
  const controller = new AbortController();
  const pending = fetch(`${origin}/__map-workbench/generate`, {
    method: "POST",
    signal: controller.signal,
    headers: { "Content-Type": "application/json", Origin: origin },
    body: JSON.stringify({
      purpose: "preview",
      draft,
      choice: { type: "mixed", size: "small", seed: "4" },
    }),
  });
  let pid: number | null = null;
  try {
    await expect
      .poll(async () => {
        pid = Number(await readFile(join(root, "http-pid"), "utf8").catch(() => "0")) || null;
        return pid;
      })
      .not.toBeNull();
    controller.abort();
    await expect(pending).rejects.toThrow();
    await expect
      .poll(() => {
        try {
          process.kill(pid!, 0);
          return false;
        } catch {
          return true;
        }
      })
      .toBe(true);
    await expect.poll(() => readdir(join(root, "throwaway/map-workbench"))).toEqual([]);
  } finally {
    if (pid) {
      try {
        process.kill(pid, "SIGKILL");
      } catch {}
    }
  }
});

test("save revalidates the reviewed settings before publishing any replacement", async () => {
  let refuse = false;
  const run: NativeReporter = async () =>
    refuse
      ? {
          status: "invalid",
          diagnostics: [
            {
              code: "invalid_presets",
              feature: null,
              location: "$.presets",
              message: "Native settings admission failed",
            },
          ],
          fields: validation.fields,
        }
      : validation;
  const { root, store } = await fixture(run);
  const draft = structuredClone(await store.snapshot());
  draft.documents.presets.density = 0.6;
  const review = await store.preview(draft);
  refuse = true;
  await expect(store.save(review)).rejects.toThrow("Native settings admission failed");
  expect(await readFile(join(root, review.files[0].path), "utf8")).toBe(review.files[0].before);
});

test("sample export and cancellation preserve pinned preview geometry for crop and sight", async () => {
  const run: NativeReporter = async (request, signal) => {
    if (request.operation === "validate") return validation;
    if (request.operation === "generate") {
      if (request.choice.seed === "99")
        return new Promise<never>((finish) => {
          if (signal!.aborted) return finish(Promise.reject(new Error("Sample cancelled")));
          signal!.addEventListener(
            "abort",
            () => finish(Promise.reject(new Error("Sample cancelled"))),
            { once: true },
          );
        });
      await writeFile(join(request.artifactDir, "seed"), request.choice.seed);
      return {
        status: "ok",
        choice: request.choice,
        diagnostics: [],
        svg: `<svg>${request.choice.seed}</svg>`,
      };
    }
    const seed = await readFile(join(request.artifactDir, "seed"), "utf8");
    if (request.operation === "inspect") return { svg: `<svg>${seed}</svg>`, features: [] };
    return {
      status: "measured",
      samples: Number(seed),
      target: 0.5,
      step_m: 300,
      bearings: 64,
      elapsed_ms: 1,
    };
  };
  const { root, store } = await fixture(run);
  const initial = await store.snapshot();
  const baseline = await store.generate({
    purpose: "preview",
    draft: initial,
    choice: { type: "mixed", size: "small", seed: "4" },
  });
  const draft = structuredClone(initial);
  draft.documents.presets.density = 0.6;
  const preview = await store.generate({ purpose: "preview", draft, choice: baseline.choice });
  const sample = await store.generate({
    purpose: "sample",
    draft,
    choice: { ...baseline.choice, seed: "5" },
  });
  expect((await store.export(sample.artifactId!)).report).toEqual(sample);
  expect((await readdir(join(root, "throwaway/map-workbench"))).sort()).toEqual(
    [baseline.artifactId, preview.artifactId].sort(),
  );
  const controller = new AbortController();
  const cancelled = store.generate(
    { purpose: "sample", draft, choice: { ...baseline.choice, seed: "99" } },
    controller.signal,
  );
  await expect
    .poll(async () => (await readdir(join(root, "throwaway/map-workbench"))).length)
    .toBe(3);
  controller.abort();
  await expect(cancelled).rejects.toThrow();
  expect(await store.inspect(preview.artifactId!, "district-1")).toMatchObject({
    svg: "<svg>4</svg>",
  });
  expect(await store.sight(preview.artifactId!)).toMatchObject({ status: "measured", samples: 4 });
  expect(await store.inspect(baseline.artifactId!)).toMatchObject({ svg: "<svg>4</svg>" });
});

test("invalid policy previews retain native diagnostic locations without writes and cannot be saved", async () => {
  const diagnostics = [
    {
      code: "invalid_presets",
      feature: "fairness",
      location: "$.presets.density",
      message: "Density is outside the admitted policy range",
    },
  ];
  const run: NativeReporter = async (request) =>
    request.operation === "validate" && JSON.parse(request.inputs.presets).density === 0.8
      ? { status: "invalid", diagnostics, fields: validation.fields }
      : validation;
  const { root, store } = await fixture(run);
  const draft = structuredClone(await store.snapshot());
  draft.documents.presets.density = 0.8;
  const preview = await store.preview(draft);
  expect(preview.diagnostics).toEqual(diagnostics);
  expect(preview.files[0].after).toContain('"density": 0.8');
  expect(await readFile(join(root, preview.files[0].path), "utf8")).toBe(preview.files[0].before);
  await expect(store.save(preview)).rejects.toMatchObject({ status: 400, diagnostics });
  expect(await readFile(join(root, preview.files[0].path), "utf8")).toBe(preview.files[0].before);
});

test("native inspection refusal becomes an HTTP error with its structured diagnostic location", async () => {
  const { root, store } = await fixture();
  const draft = await store.snapshot();
  const diagnostics = [
    {
      code: "invalid_request",
      feature: null,
      location: "$.crop",
      message: "Unknown inspection crop",
    },
  ];
  const binary = join(root, "throwaway/target/release/examples/map_workbench_report");
  await mkdir(join(root, "throwaway/target/release/examples"), { recursive: true });
  await writeFile(
    binary,
    `#!/usr/bin/env node\nlet input = ""; process.stdin.on("data", data => input += data); process.stdin.on("end", () => { const request = JSON.parse(input); const output = request.operation === "validate" ? ${JSON.stringify(validation)} : request.operation === "generate" ? {status:"ok",choice:request.choice,diagnostics:[],svg:"<svg/>"} : {status:"invalid",fields:[],diagnostics:${JSON.stringify(diagnostics)}}; process.stdout.write(JSON.stringify(output)); });\n`,
  );
  await chmod(binary, 0o755);
  const server = await createViteServer({
    configFile: false,
    plugins: [mapWorkbenchPlugin(root)],
    server: { host: "127.0.0.1", port: 0 },
  });
  servers.push(server);
  await server.listen();
  const address = server.httpServer!.address();
  if (!address || typeof address === "string") throw new Error("No server port");
  const origin = `http://127.0.0.1:${address.port}`;
  const post = (operation: string, body: unknown) =>
    fetch(`${origin}/__map-workbench/${operation}`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Origin: origin },
      body: JSON.stringify(body),
    });
  const generated = await post("generate", {
    purpose: "preview",
    draft,
    choice: { type: "mixed", size: "small", seed: "4" },
  }).then((response) => response.json());
  const response = await post("inspect", {
    artifactId: generated.artifactId,
    crop: "nonexistent-district",
  });
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchObject({ error: "Unknown inspection crop", diagnostics });
});

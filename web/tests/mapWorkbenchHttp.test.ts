// @vitest-environment node
import { mkdir, writeFile, readFile, readdir, chmod, watch } from "node:fs/promises";
import { createServer as createViteServer, type ViteDevServer } from "vite";
import { join } from "node:path";
import { afterEach, expect, test, vi } from "vitest";
import { mapWorkbenchPlugin } from "../../apps/map-workbench/server";
import { fixture, validation, reporter, closeWorkbenchFixtures } from "./support/mapWorkbench";

const servers: ViteDevServer[] = [];
afterEach(async () => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  await Promise.all(servers.splice(0).map((server) => server.close()));
  await closeWorkbenchFixtures();
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

test("disconnecting an HTTP generation terminates the process and removes its unpublished artifact", async ({
  signal,
}) => {
  vi.stubEnv("CARGO_TARGET_DIR", "throwaway/target");
  const { root, store } = await fixture();
  const draft = await store.snapshot();
  const binary = join(root, "throwaway/target/release/examples/map_workbench_report");
  await mkdir(join(root, "throwaway/target/release/examples"), { recursive: true });
  await writeFile(
    binary,
    `#!/usr/bin/env node\nlet input = ""; process.stdin.on("data", data => input += data); process.stdin.on("end", () => { const request = JSON.parse(input); if (request.operation === "validate") { process.stdout.write(${JSON.stringify(JSON.stringify(validation))}); } else { const fs = require("node:fs"); fs.writeFileSync(${JSON.stringify(join(root, "http-pid.tmp"))}, String(process.pid)); fs.renameSync(${JSON.stringify(join(root, "http-pid.tmp"))}, ${JSON.stringify(join(root, "http-pid"))}); setInterval(() => {}, 1000); } });\n`,
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
  const lifetime = AbortSignal.any([signal, controller.signal]);
  // Generation starts after native validation; startup is not a timing assertion.
  const ready = (async () => {
    for await (const event of watch(root, { signal: lifetime })) {
      if (event.filename === "http-pid")
        return Number(await readFile(join(root, "http-pid"), "utf8"));
    }
    throw new Error("Native generation did not announce readiness");
  })();
  const pending = fetch(`${origin}/__map-workbench/generate`, {
    method: "POST",
    signal: lifetime,
    headers: { "Content-Type": "application/json", Origin: origin },
    body: JSON.stringify({
      purpose: "preview",
      retainedArtifactIds: [],
      draft,
      choice: { type: "mixed", size: "small", seed: "4" },
    }),
  });
  let pid: number | null = null;
  try {
    pid = await Promise.race([
      ready,
      pending.then(() => {
        throw new Error("HTTP generation completed before cancellation");
      }),
    ]);
    process.kill(pid!, 0);
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
    controller.abort();
    await pending.catch(() => {});
    if (pid) {
      try {
        process.kill(pid, "SIGKILL");
      } catch {}
    }
  }
});

test("native inspection refusal becomes an HTTP error with its structured diagnostic location", async () => {
  vi.stubEnv("CARGO_TARGET_DIR", "throwaway/target");
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
  const generation = await post("generate", {
    purpose: "preview",
    retainedArtifactIds: [],
    draft,
    choice: { type: "mixed", size: "small", seed: "4" },
  });
  expect(generation.status).toBe(200);
  const generated = await generation.json();
  const response = await post("inspect", {
    artifactId: generated.artifactId,
    crop: "nonexistent-district",
  });
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchObject({ error: "Unknown inspection crop", diagnostics });
});

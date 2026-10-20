// @vitest-environment node
import { mkdtemp, mkdir, writeFile, readFile, chmod, rm, watch } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test, vi } from "vitest";
import { nativeReporter } from "../../apps/map-workbench/native";

const roots: string[] = [];
afterEach(async () => {
  vi.unstubAllEnvs();
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

test("aborting native work terminates its actual process before resolving cancellation", async ({
  signal,
}) => {
  vi.stubEnv("CARGO_TARGET_DIR", "throwaway/target");
  const root = await mkdtemp(join(tmpdir(), "native-reporter-"));
  roots.push(root);
  const binary = join(root, "throwaway/target/release/examples/map_workbench_report");
  await mkdir(join(root, "throwaway/target/release/examples"), { recursive: true });
  await writeFile(
    binary,
    `#!/usr/bin/env node
const fs = require("node:fs");
fs.writeFileSync(${JSON.stringify(join(root, "pid.tmp"))}, String(process.pid));
fs.renameSync(${JSON.stringify(join(root, "pid.tmp"))}, ${JSON.stringify(join(root, "pid"))});
setInterval(() => {}, 1000);
`,
  );
  await chmod(binary, 0o755);
  const controller = new AbortController();
  const lifetime = AbortSignal.any([signal, controller.signal]);
  // Process startup depends on machine load; wait for its explicit readiness signal.
  const ready = (async () => {
    for await (const event of watch(root, { signal: lifetime })) {
      if (event.filename === "pid") return Number(await readFile(join(root, "pid"), "utf8"));
    }
    throw new Error("Native fixture did not announce readiness");
  })();
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
    lifetime,
  );
  let pid: number | null = null;
  try {
    pid = await Promise.race([
      ready,
      pending.then(() => {
        throw new Error("Native fixture exited before cancellation");
      }),
    ]);
    process.kill(pid!, 0);
    controller.abort();
    await expect(pending).rejects.toMatchObject({ status: 499 });
    expect(() => process.kill(pid!, 0)).toThrow();
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

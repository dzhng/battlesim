// @vitest-environment node
import { mkdtemp, mkdir, writeFile, readFile, chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test, vi } from "vitest";
import { nativeReporter } from "../../apps/map-workbench/native";

const roots: string[] = [];
afterEach(async () => {
  vi.unstubAllEnvs();
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

test("aborting native work terminates its actual process before resolving cancellation", async () => {
  vi.stubEnv("CARGO_TARGET_DIR", "throwaway/target");
  const root = await mkdtemp(join(tmpdir(), "native-reporter-"));
  roots.push(root);
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

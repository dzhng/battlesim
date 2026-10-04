// @vitest-environment node
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { expect, test } from "vitest";

function runBatch(exitCode: number, output: string) {
  const root = mkdtempSync(resolve(tmpdir(), "blender sources "));
  try {
    const scripts = resolve(root, "packages/scene-assets/blender");
    const bin = resolve(root, "bin");
    mkdirSync(scripts, { recursive: true });
    mkdirSync(resolve(root, "web"));
    mkdirSync(bin);
    const script = resolve(scripts, "build_sources.sh");
    copyFileSync(
      resolve(import.meta.dirname, "../../packages/scene-assets/blender/build_sources.sh"),
      script,
    );
    const log = resolve(root, "calls");
    writeFileSync(
      resolve(bin, "node"),
      `#!/bin/sh\nprintf '%s\\n' "$*" >> "$CALL_LOG"\nprintf '%s\\n' "$EXPORT_OUTPUT"\nexit "$EXPORT_EXIT"\n`,
      { mode: 0o755 },
    );
    const result = spawnSync("/bin/sh", [script], {
      cwd: tmpdir(),
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH}`,
        CALL_LOG: log,
        EXPORT_OUTPUT: output,
        EXPORT_EXIT: String(exitCode),
      },
      encoding: "utf8",
    });
    return { ...result, calls: readFileSync(log, "utf8").trim().split("\n") };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test("the source batch stops at an exporter failure even after matching output", () => {
  const result = runBatch(42, "wrote partial-source");
  expect(result.status).toBe(42);
  expect(result.calls).toHaveLength(1);
  expect(result.stdout).toContain("wrote partial-source");
});

test.each(["wrote complete-source", "export finished"])(
  "a successful batch completes with output: %s",
  (output) => {
    const result = runBatch(0, output);
    expect(result.status).toBe(0);
    expect(result.calls[0]).toContain("infantry_kit.py at_carried a");
    expect(result.calls.at(-1)).toContain("forest_floor.py");
  },
);

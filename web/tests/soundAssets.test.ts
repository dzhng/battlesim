// @vitest-environment node
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { spawnSync } from "node:child_process";
import { expect, test } from "vitest";

test("audio rebuild refuses a changed pinned source before replacing a clip", () => {
  const root = mkdtempSync(join(tmpdir(), "battle-audio-assets-"));
  try {
    mkdirSync(join(root, "fixtures"));
    mkdirSync(join(root, "assets/third-party/audio"), { recursive: true });
    mkdirSync(join(root, "assets/runtime/audio/clips"), { recursive: true });
    writeFileSync(join(root, "assets/third-party/audio/source.wav"), "changed source");
    writeFileSync(join(root, "assets/runtime/audio/clips/report.wav"), "retained clip");
    writeFileSync(
      join(root, "fixtures/sounds.json"),
      JSON.stringify({
        sources: {
          source: { path: "assets/third-party/audio/source.wav", sha256: "a".repeat(64) },
        },
        clips: {
          report: {
            source: "source",
            source_rate: 48000,
            source_frames: [0, 100],
            processing: "weighted-shot",
            url: "/audio/clips/report.wav",
          },
        },
      }),
    );
    const run = spawnSync(
      "python3",
      [resolve("../packages/battle-audio/tools/assets.py"), "build", "--root", root],
      { encoding: "utf8" },
    );
    expect(run.status).not.toBe(0);
    expect(run.stderr).toMatch(/source.*hash/i);
    expect(readFileSync(join(root, "assets/runtime/audio/clips/report.wav"), "utf8")).toBe(
      "retained clip",
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

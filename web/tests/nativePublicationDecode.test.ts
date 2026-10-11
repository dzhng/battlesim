// @vitest-environment node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import type { ObservationLayout, ObservationView } from "@web/battle/sim/observationView";
import { ObservationDecoder, loadPresentation } from "@web/battle/sim/presentation";

await loadPresentation();

// The page decodes every sample of native's fixture captures to the committed
// expectation, the last decode of the TS decoder the Rust reader replaced.
// Native's decoder is compared against the same file (`publication_decode_test.gd`).
const fixtures = new URL("../../native/godot/tests/fixtures/", import.meta.url);
const CAPTURES = {
  "paris-corner": "run/paris-corner/capture.json",
  // Seven samples of the market-town control, the first re-encoded as a fresh
  // record, for the groups paris-corner never publishes.
  "market-town": "publication/market-town-capture.json",
};

/** The fog field's words as a digest: the market-town field is 17,579 words a sample. */
const fogDigest = (bits: Uint32Array) =>
  "sha256:" +
  createHash("sha256")
    .update(new Uint8Array(bits.buffer, bits.byteOffset, bits.byteLength))
    .digest("hex");

/** The shortest decimal that reads back as the same float32: every decoded
 * number is a float32, so native compares them at that precision. */
function f32Text(value: number): number {
  for (let digits = 1; digits < 17; digits++) {
    const short = Number(value.toPrecision(digits));
    if (Math.fround(short) === value) return short;
  }
  return value;
}

/** An observation as JSON: typed arrays become arrays, NaN null, and every
 * number its shortest float32 text. */
function exported(view: ObservationView): unknown {
  return JSON.parse(
    JSON.stringify(
      { ...view, fog: { ...view.fog, bits: fogDigest(view.fog.bits) } },
      (_key, value) =>
        ArrayBuffer.isView(value)
          ? Array.from(value as Float32Array, f32Text)
          : typeof value === "number"
            ? f32Text(value)
            : value,
    ),
  );
}

test("every sample of native's fixture captures decodes to the committed expectation", () => {
  const seen = new Map<string, number>();
  const count = (name: string, n: number) => seen.set(name, (seen.get(name) ?? 0) + n);
  for (const [scene, path] of Object.entries(CAPTURES)) {
    const capture = JSON.parse(readFileSync(new URL(path, fixtures), "utf8"));
    const decoder = new ObservationDecoder(JSON.parse(capture.layout) as ObservationLayout);
    const observations = capture.samples.map((sample: { tick: number; publication: number[] }) => {
      const view = decoder.decode(new Float32Array(new Uint32Array(sample.publication).buffer));
      expect(view, `${scene} tick ${sample.tick}`).not.toBeNull();
      for (const group of [
        "own",
        "identified",
        "contacts",
        "audible",
        "knownProps",
        "projectiles",
        "blasts",
        "corpses",
        "fallenBodies",
      ] as const)
        count(group, view![group].length);
      count(
        "ricochets",
        view!.projectiles.reduce((n, p) => n + p.ricochets.length, 0),
      );
      count("groundRuns", view!.groundPatch.runs.length / 4);
      count(
        "clearedRuns",
        view!.groundPatch.runs.filter((v, i) => i % 4 === 3 && v >= 65536).length,
      );
      return exported(view!);
    });
    const expected = JSON.parse(
      readFileSync(new URL(`publication/${scene}-observations.json`, fixtures), "utf8"),
    );
    expect({ capture: path, observations }).toEqual(expected);
  }
  // The fixtures exercise every group the menu reel publishes.
  for (const [group, n] of seen) expect(n, group).toBeGreaterThan(0);
});

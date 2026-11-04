// @vitest-environment node
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { expect, test } from "vitest";
import type { ObservationLayout, ObservationView } from "../src/battle/sim/observationView";
import { loadPresentation, ScenePresentation } from "../src/battle/sim/presentation";

await loadPresentation();

const captures = [
  ["paris-corner", "../../fixtures/parity/capture/run/paris-corner/capture.json"],
  ["market-town", "../../fixtures/parity/capture/publication/market-town-capture.json"],
] as const;

type FixtureObservation = Omit<ObservationView, "fog" | "groundPatch"> & {
  fog: Omit<ObservationView["fog"], "bits"> & { bits: string };
  groundPatch: ObservationView["groundPatch"] & { runs: number[] };
};

function sha256Words(words: readonly number[]): string {
  const bytes = Buffer.allocUnsafe(words.length * 4);
  words.forEach((word, i) => bytes.writeUInt32LE(word >>> 0, i * 4));
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function decodeCapture(capturePath: string): FixtureObservation[] {
  const capture = JSON.parse(readFileSync(new URL(capturePath, import.meta.url), "utf8")) as {
    layout: string;
    side: string;
    tickHz: number;
    samples: { publication: number[] }[];
  };
  const decoder = new ScenePresentation(
    JSON.parse(capture.layout) as ObservationLayout,
    capture.tickHz,
  );
  decoder.invalidate(capture.side);
  try {
    return capture.samples.map((sample) => {
      const view = decoder.decode(Uint32Array.from(sample.publication));
      if (!view) throw new Error("capture publication was stale");
      const fixture = structuredClone(view) as unknown as FixtureObservation;
      fixture.fog.bits = sha256Words(Array.from(view.fog.bits));
      fixture.groundPatch = {
        ...fixture.groundPatch,
        runs: Array.from(decoder.groundRuns()),
      };
      return fixture;
    });
  } finally {
    decoder.free();
  }
}

for (const [scene, capturePath] of captures) {
  test(`browser decoder reproduces ${scene} observation fixture`, () => {
    const fixtureUrl = new URL(
      `../../fixtures/parity/capture/publication/${scene}-observations.json`,
      import.meta.url,
    );
    const observations = { observations: decodeCapture(capturePath) };
    if (process.env.BLESS_PARITY === "1") {
      writeFileSync(fixtureUrl, `${JSON.stringify(observations)}\n`);
    } else {
      expect(observations).toEqual(JSON.parse(readFileSync(fixtureUrl, "utf8")));
    }
  });
}

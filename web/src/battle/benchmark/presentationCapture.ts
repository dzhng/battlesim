import type { CameraPose } from "@packages/renderer-core/src/cameraController";

/** A renderer-only playback artifact. It contains observed authority output,
 * not rules or a second simulation. Native clients may decode the packed
 * publication; browser tooling may retain the decoded observation beside it. */
export interface PresentationCapture {
  schema: "battle-presentation-capture/v1";
  workload: { id: string; fingerprint: string; scene: string; map: string; encounter: string; seed: number };
  tickHz: number;
  warmTick: number;
  side: "blue" | "red";
  samples: readonly PresentationSample[];
}

export interface PresentationSample {
  tick: number;
  digest: string;
  publication: readonly number[];
  camera: CameraPose;
}

/** Reject malformed or reordered captures before a renderer can benchmark it. */
export function validatePresentationCapture(capture: PresentationCapture): PresentationCapture {
  if (capture.schema !== "battle-presentation-capture/v1") throw new Error("unsupported presentation capture schema");
  if (!Number.isInteger(capture.tickHz) || capture.tickHz <= 0) throw new Error("capture tickHz must be positive");
  if (!Number.isInteger(capture.warmTick) || capture.warmTick < 0) throw new Error("capture warmTick must be non-negative");
  let previous = capture.warmTick - 1;
  for (const sample of capture.samples) {
    if (!Number.isInteger(sample.tick) || sample.tick <= previous) throw new Error("capture samples must increase by tick");
    if (!/^[0-9a-f]{16}$/i.test(sample.digest)) throw new Error("capture digest must be a 16-digit hex value");
    if (!sample.publication.every(Number.isFinite)) throw new Error("capture publication contains a non-finite value");
    previous = sample.tick;
  }
  return capture;
}

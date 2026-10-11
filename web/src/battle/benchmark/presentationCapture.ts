import type { CameraPose } from "@packages/renderer-core/src/cameraController";

/** A renderer-only playback artifact. It contains observed authority output,
 * not rules or a second simulation. Native clients may decode the packed
 * publication; browser tooling may retain the decoded observation beside it. */
export interface PresentationCapture {
  schema: "battle-presentation-capture/v1";
  workload: {
    id: string;
    fingerprint: string;
    scene: string;
    map: string;
    encounter: string;
    seed: number;
  };
  tickHz: number;
  warmTick: number;
  side: "blue" | "red";
  /** Exact Rust publication layout JSON for a fresh decoder. */
  layout: string;
  samples: readonly PresentationSample[];
  frames: readonly PresentationFrame[];
}

export interface PresentationSample {
  tick: number;
  digest: string;
  /** Raw u32 carriers from the authority publication, never decoded as f32. */
  publication: readonly number[];
  camera: CameraPose;
}

export interface PresentationFrame {
  elapsedMs: number;
  tick: number;
  camera: CameraPose;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

function isCameraPose(value: unknown): value is CameraPose {
  if (!isRecord(value) || !Array.isArray(value.target) || value.target.length !== 2)
    return false;
  return (
    value.target.every((coordinate) => typeof coordinate === "number" && Number.isFinite(coordinate)) &&
    typeof value.distance === "number" &&
    Number.isFinite(value.distance) &&
    typeof value.yaw === "number" &&
    Number.isFinite(value.yaw) &&
    typeof value.pitch === "number" &&
    Number.isFinite(value.pitch)
  );
}

/** Reject malformed or reordered captures before a renderer can benchmark it. */
export function validatePresentationCapture(capture: PresentationCapture): PresentationCapture {
  if (capture.schema !== "battle-presentation-capture/v1")
    throw new Error("unsupported presentation capture schema");
  if (!Number.isInteger(capture.tickHz) || capture.tickHz <= 0)
    throw new Error("capture tickHz must be positive");
  if (!Number.isInteger(capture.warmTick) || capture.warmTick < 0)
    throw new Error("capture warmTick must be non-negative");
  if (typeof capture.layout !== "string" || capture.layout.length === 0)
    throw new Error("capture layout must be non-empty JSON");
  if (!Array.isArray(capture.frames)) throw new Error("capture frames must be an array");
  let previousFrame = -1;
  for (const frame of capture.frames) {
    if (!isRecord(frame)) throw new Error("capture frame is malformed");
    const elapsedMs = frame.elapsedMs;
    if (
      typeof elapsedMs !== "number" ||
      !Number.isFinite(elapsedMs) ||
      elapsedMs < previousFrame
    )
      throw new Error("capture frame times must be increasing");
    const tick = frame.tick;
    if (typeof tick !== "number" || !Number.isInteger(tick) || tick < 0)
      throw new Error("capture frame tick must be non-negative");
    if (!isCameraPose(frame.camera))
      throw new Error("capture frame camera is malformed");
    previousFrame = elapsedMs;
  }
  if (!Array.isArray(capture.samples)) throw new Error("capture samples must be an array");
  let previous = -1;
  for (const sample of capture.samples) {
    if (!isRecord(sample)) throw new Error("capture sample is malformed");
    const tick = sample.tick;
    if (typeof tick !== "number" || !Number.isInteger(tick) || tick < 0 || tick <= previous)
      throw new Error("capture samples must increase by non-negative tick");
    const digest = sample.digest;
    if (typeof digest !== "string" || !/^[0-9a-f]{16}$/i.test(digest))
      throw new Error("capture digest must be a 16-digit hex value");
    const publication = sample.publication;
    if (
      !Array.isArray(publication) ||
      !publication.every(
        (word) => typeof word === "number" && Number.isInteger(word) && word >= 0 && word <= 0xffffffff,
      )
    )
      throw new Error("capture publication contains an invalid u32 word");
    if (!isCameraPose(sample.camera)) throw new Error("capture sample camera is malformed");
    previous = tick;
  }
  return capture;
}

/** Stable JSON boundary used by ignored benchmark evidence and native tools. */
export function encodePresentationCapture(capture: PresentationCapture): string {
  return JSON.stringify(validatePresentationCapture(capture));
}

export function decodePresentationCapture(text: string): PresentationCapture {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error("presentation capture is not valid JSON");
  }
  return validatePresentationCapture(value as PresentationCapture);
}

/** A renderer-only playback artifact. It contains observed authority output,
 * not rules or a second simulation. Native clients may decode the packed
 * publication; browser tooling may retain the decoded observation beside it.
 * The format changes in place: a stale file is refused, never migrated. */
export interface PresentationCapture {
  schema: "battle-presentation-capture";
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
}

type Point = readonly [number, number, number];

/** One drawn frame, as a second client must draw it again: the view after
 *  the rig placed it (clamped, held clear of obstacles, lagging its subject),
 *  in sim space and metres. */
export interface PresentationFrame {
  /** Its position in `frames`. */
  index: number;
  /** Wall time into the scene's reel. */
  reelMs: number;
  /** The latest simulation tick when the frame was drawn. */
  tick: number;
  /** The interpolated sim time the frame drew with; the wind reads it too. */
  simTime: number;
  eye: Point;
  target: Point;
  /** The screen's up in the world. */
  up: Point;
  /** The drawn canvas in pixels (one per CSS pixel). */
  canvas: readonly [number, number];
  /** The control plate read back from this frame (`plateFile`), or null. */
  plate: string | null;
}

/** The file a frame's control plate is written to. */
export const plateFile = (index: number) => `plate-${index}.png`;

const SAMPLE_KEYS = ["tick", "digest", "publication"];
const FRAME_KEYS = ["index", "reelMs", "tick", "simTime", "eye", "target", "up", "canvas", "plate"];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

/** Exactly `keys`: an old or foreign shape is refused, not read in part. */
const hasKeys = (value: unknown, keys: readonly string[]): value is Record<string, unknown> =>
  isRecord(value) && Object.keys(value).length === keys.length && keys.every((key) => key in value);

const finite = (value: unknown): value is number =>
  typeof value === "number" && Number.isFinite(value);

const isPoint = (value: unknown): boolean =>
  Array.isArray(value) && value.length === 3 && value.every(finite);

const isPixels = (value: unknown): boolean =>
  Array.isArray(value) &&
  value.length === 2 &&
  value.every((n) => Number.isInteger(n) && (n as number) > 0);

function validateFrames(frames: unknown) {
  if (!Array.isArray(frames)) throw new Error("capture frames must be an array");
  let previous: PresentationFrame | null = null;
  for (const [position, frame] of frames.entries()) {
    if (!hasKeys(frame, FRAME_KEYS)) throw new Error(`capture frame ${position} is malformed`);
    const f = frame as unknown as PresentationFrame;
    if (f.index !== position) throw new Error(`capture frame ${position} has index ${f.index}`);
    if (!finite(f.reelMs) || (previous && f.reelMs < previous.reelMs))
      throw new Error(`capture frame ${position} reelMs goes back`);
    if (!Number.isInteger(f.tick) || f.tick < 0 || (previous && f.tick < previous.tick))
      throw new Error(`capture frame ${position} tick goes back`);
    if (!finite(f.simTime) || (previous && f.simTime < previous.simTime))
      throw new Error(`capture frame ${position} simTime goes back`);
    for (const key of ["eye", "target", "up"] as const)
      if (!isPoint(f[key])) throw new Error(`capture frame ${position} ${key} is not a point`);
    if (!isPixels(f.canvas)) throw new Error(`capture frame ${position} canvas is not pixels`);
    if (f.plate !== null && f.plate !== plateFile(position))
      throw new Error(`capture frame ${position} plate must be ${plateFile(position)}`);
    previous = f;
  }
}

function validateSamples(samples: unknown) {
  if (!Array.isArray(samples)) throw new Error("capture samples must be an array");
  let previous = -1;
  for (const sample of samples) {
    if (!hasKeys(sample, SAMPLE_KEYS)) throw new Error("capture sample is malformed");
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
        (word) =>
          typeof word === "number" && Number.isInteger(word) && word >= 0 && word <= 0xffffffff,
      )
    )
      throw new Error("capture publication contains an invalid u32 word");
    previous = tick;
  }
}

/** Reject malformed, stale or reordered captures before a renderer reads one. */
export function validatePresentationCapture(capture: PresentationCapture): PresentationCapture {
  if (capture.schema !== "battle-presentation-capture")
    throw new Error(`unsupported presentation capture schema ${JSON.stringify(capture.schema)}`);
  if (!Number.isInteger(capture.tickHz) || capture.tickHz <= 0)
    throw new Error("capture tickHz must be positive");
  if (!Number.isInteger(capture.warmTick) || capture.warmTick < 0)
    throw new Error("capture warmTick must be non-negative");
  if (typeof capture.layout !== "string" || capture.layout.length === 0)
    throw new Error("capture layout must be non-empty JSON");
  validateFrames(capture.frames);
  validateSamples(capture.samples);
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

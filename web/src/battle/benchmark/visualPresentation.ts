// The `native-visual-presentation` manifest: what the browser drew for one
// captured scene, beside its `battle-presentation-capture`, so a second client
// draws the same battle through the same lens and the same layers. The
// browser run is its one writer; it grows only by what native reads, and
// changes in place (a stale file is refused, never migrated).
import { decodePresentationCapture } from "./presentationCapture.ts";

/** The presentation layers a frame can draw. */
export const PRESENTATION_LAYERS = [
  "sky",
  "terrain",
  "water",
  "grass",
  "scenery",
  "props",
  "structures",
  "buildings",
  "models",
  "effects",
  "scars",
  "fog",
  "overlays",
] as const;
export type PresentationLayer = (typeof PRESENTATION_LAYERS)[number];

/** The drawn lens: vertical field of view (radians) and near plane (metres),
 *  through the engine-wide reverse-Z depth with no far plane. */
export interface Lens {
  fovY: number;
  near: number;
  depth: "reverse-z";
  far: "infinite";
}

/** Something the browser meant to draw and did not. */
export interface Omission {
  owner: string;
  reason: string;
  sourceId: string;
}

/** What the page knows of its own drawing; the files give the rest. */
export interface DrawnPresentation {
  lens: Lens;
  layers: readonly PresentationLayer[];
  omissions: readonly Omission[];
}

export interface VisualPresentation extends DrawnPresentation {
  schema: "native-visual-presentation";
  identity: {
    scene: string;
    map: string;
    encounter: string;
    seed: number;
    /** sha256 of the saved map's `map.json` bytes. */
    mapHash: string;
    /** sha256 of the capture file's bytes. */
    captureHash: string;
  };
  /** The canvas every frame drew, at one pixel per CSS pixel. */
  viewport: { width: number; height: number; scale: 1 };
}

const IDENTITY_KEYS = ["scene", "map", "encounter", "seed", "mapHash", "captureHash"];
const MANIFEST_KEYS = ["schema", "identity", "viewport", "lens", "layers", "omissions"];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const hasKeys = (value: unknown, keys: readonly string[]): value is Record<string, unknown> =>
  isRecord(value) && Object.keys(value).length === keys.length && keys.every((k) => k in value);

const text = (value: unknown) => typeof value === "string" && value.length > 0;
const isHash = (value: unknown) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

/** `value` as a manifest, or a refusal naming the part at fault. */
export function validateVisualPresentation(value: unknown): VisualPresentation {
  if (!hasKeys(value, MANIFEST_KEYS)) throw new Error("visual presentation is malformed");
  if (value.schema !== "native-visual-presentation")
    throw new Error(`unsupported visual presentation schema ${JSON.stringify(value.schema)}`);
  const id = value.identity;
  if (
    !hasKeys(id, IDENTITY_KEYS) ||
    ![id.scene, id.map, id.encounter].every(text) ||
    !Number.isInteger(id.seed) ||
    !isHash(id.mapHash) ||
    !isHash(id.captureHash)
  )
    throw new Error("visual presentation identity is malformed");
  const view = value.viewport;
  if (
    !hasKeys(view, ["width", "height", "scale"]) ||
    !Number.isInteger(view.width) ||
    !Number.isInteger(view.height) ||
    (view.width as number) <= 0 ||
    (view.height as number) <= 0 ||
    view.scale !== 1
  )
    throw new Error("visual presentation viewport is malformed");
  const lens = value.lens;
  if (
    !hasKeys(lens, ["fovY", "near", "depth", "far"]) ||
    !(typeof lens.fovY === "number" && lens.fovY > 0 && lens.fovY < Math.PI) ||
    !(typeof lens.near === "number" && lens.near > 0) ||
    lens.depth !== "reverse-z" ||
    lens.far !== "infinite"
  )
    throw new Error("visual presentation lens is malformed");
  const layers = value.layers;
  if (
    !Array.isArray(layers) ||
    !layers.every((l) => (PRESENTATION_LAYERS as readonly unknown[]).includes(l)) ||
    new Set(layers).size !== layers.length
  )
    throw new Error("visual presentation layers must be distinct known layer names");
  const omissions = value.omissions;
  if (
    !Array.isArray(omissions) ||
    !omissions.every(
      (o) =>
        hasKeys(o, ["owner", "reason", "sourceId"]) && [o.owner, o.reason, o.sourceId].every(text),
    )
  )
    throw new Error("visual presentation omission rows need an owner, reason and sourceId");
  return value as unknown as VisualPresentation;
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as BufferSource));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** The manifest JSON for a capture file's text `captureJson` drawn on the
 *  map whose `map.json` is `mapBytes`: the identity is the capture's
 *  workload and both files' hashes, the viewport the canvas every frame drew. */
export async function writeVisualPresentation({
  captureJson,
  mapBytes,
  drawn,
}: {
  captureJson: string;
  mapBytes: Uint8Array;
  drawn: DrawnPresentation;
}): Promise<string> {
  const capture = decodePresentationCapture(captureJson);
  const [first] = capture.frames;
  if (!first) throw new Error("a capture with no frame has no viewport");
  const [width, height] = first.canvas;
  const resized = capture.frames.find((f) => f.canvas[0] !== width || f.canvas[1] !== height);
  if (resized)
    throw new Error(
      `the canvas changed from ${width}×${height} to ${resized.canvas.join("×")} at frame ${resized.index}`,
    );
  const { scene, map, encounter, seed } = capture.workload;
  const manifest: VisualPresentation = {
    schema: "native-visual-presentation",
    identity: {
      scene,
      map,
      encounter,
      seed,
      mapHash: await sha256(mapBytes),
      captureHash: await sha256(new TextEncoder().encode(captureJson)),
    },
    viewport: { width, height, scale: 1 },
    lens: drawn.lens,
    layers: drawn.layers,
    omissions: drawn.omissions,
  };
  return JSON.stringify(validateVisualPresentation(manifest));
}

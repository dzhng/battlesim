/** Canonical appearance catalog records and external model pose requests. */
import type { ModelInstance, ModelPose } from "@packages/battle-renderer/src/models/modelInstances";
import { type Presentation, type Renderer, type PreparedAppearances } from "@wasm/renderer_browser";
import { contentIdKey } from "../render/contentIdentity";
import type { ContentId } from "@packages/scene-assets/src/loader";
import type { ModelAsset } from "../render/modelAssets";
import { releaseTransport } from "../transport";
import {
  APPEARANCE_WORDS,
  Appearance,
  worldViews,
} from "@packages/battle-renderer/src/records.gen";
import { STUMP } from "@packages/battle-renderer/src/scenery/walk";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import type { Articulation } from "@packages/scene-assets/src/articulation";

export interface AdmissionTransportWork {
  bundlesEncoded: number;
  bundleBytesSent: number;
}

const preparedTransportWork = new WeakMap<object, AdmissionTransportWork>();

export function preparedAdmissionWork(prepared: PreparedAppearances): AdmissionTransportWork {
  return preparedTransportWork.get(prepared) ?? { bundlesEncoded: 0, bundleBytesSent: 0 };
}

export interface ModelDetail {
  lod_px: readonly [number, number, number];
  impostor_px: number;
  shadow_caster_px?: number;
}

export type LabModel = Omit<ModelInstance, "pose"> & {
  pose:
    | ModelPose
    | { kind: "rest" }
    | { kind: "clip"; clip: string; phase: number }
    | { kind: "articulated"; articulation: Articulation };
};

/** JSON only at request admission. Typed Joint data is a flat numeric array. */
export function labModelsJson(models: readonly LabModel[]): string {
  return JSON.stringify(
    models.map((m) => ({
      ...m,
      pose: m.pose.kind === "posed" ? { ...m.pose, palette: Array.from(m.pose.palette) } : m.pose,
    })),
  );
}

/** One drawable the frame's batches name, by `World.appearances` row;
 *  `placed` for the static world's (`Appearance.placed`). */
export interface PresentedAppearance {
  name: string;
  state: string;
  lies: number;
  placed: boolean;
  /** What draws it (`Appearance.draws`): 0 a model or kit module, else the scenery. */
  draws: number;
  /** A felled tree's stump row's mesh (`draws` 6; `Presentation::stump_mesh`). */
  stump?: Mesh;
}

/** Own row metadata before any allocating stump query can grow WASM memory. */
export function readAppearanceRows(
  memory: ArrayBuffer,
  ptr: number,
  rust: Pick<Presentation, "stump_mesh">,
): PresentedAppearance[] {
  const views = worldViews(memory, ptr);
  const text = new TextDecoder();
  const rows = views.appearances.u32;
  const name = (at: number, len: number) => text.decode(views.strings.subarray(at, at + len));
  const loaded: PresentedAppearance[] = Array.from({ length: views.appearances.count }, (_, r) => {
    const o = r * APPEARANCE_WORDS;
    return {
      name: name(rows[o + Appearance.name], rows[o + Appearance.name_len]),
      state: name(rows[o + Appearance.state], rows[o + Appearance.state_len]),
      lies: rows[o + Appearance.lies],
      placed: rows[o + Appearance.placed] === 1,
      draws: rows[o + Appearance.draws],
    };
  });
  // After the views: each call may grow memory.
  loaded.forEach((row, r) => {
    if (row.draws === STUMP) row.stump = rust.stump_mesh(r) as Mesh;
  });
  return loaded;
}

/** Declare the complete wanted catalog and transport only Rust's missing content. */
export async function prepareAppearances(
  renderer: Pick<
    Renderer,
    "missing_content_ids" | "appearance_draft" | "begin_appearances" | "prepare_appearances"
  >,
  presentation: Presentation,
  assets: readonly ModelAsset[],
  detail: ModelDetail,
  nextFrame: () => Promise<boolean>,
): Promise<PreparedAppearances | null> {
  const missing = new Set(
    (
      JSON.parse(
        renderer.missing_content_ids(
          presentation,
          JSON.stringify(assets.map((asset) => asset.contentId)),
        ),
      ) as ContentId[]
    ).map(contentIdKey),
  );
  let draft: ReturnType<Renderer["appearance_draft"]> | null = renderer.appearance_draft(
    presentation,
    JSON.stringify(detail),
  );
  let warming: ReturnType<Renderer["begin_appearances"]> | null = null;
  const work = { bundlesEncoded: 0, bundleBytesSent: 0 };
  try {
    for (const asset of assets) {
      draft.declare(asset.name, JSON.stringify(asset.contentId));
      if (asset.placement) draft.set_placement(asset.name, JSON.stringify(asset.placement));
    }
    for (const asset of assets) {
      if (!missing.delete(contentIdKey(asset.contentId))) continue;
      if (!(await nextFrame())) return null;
      const { bundle, textureIds, textureBytes, skeleton } = asset.encode();
      try {
        work.bundlesEncoded++;
        work.bundleBytesSent += bundle.byteLength;
        draft.add_bundle(
          JSON.stringify(asset.contentId),
          bundle,
          textureIds,
          textureBytes,
          skeleton,
        );
      } finally {
        releaseTransport(bundle);
      }
    }
    if (!(await nextFrame())) return null;
    // Ownership moves into Rust even when synchronous catalog admission throws.
    const supplied = draft;
    draft = null;
    warming = renderer.begin_appearances(presentation, supplied);
    do {
      if (!(await nextFrame())) return null;
    } while (warming.step());
    if (!(await nextFrame())) return null;
    const complete = warming;
    warming = null;
    const prepared = await renderer.prepare_appearances(complete);
    preparedTransportWork.set(prepared, work);
    return prepared;
  } finally {
    draft?.free();
    warming?.free();
  }
}

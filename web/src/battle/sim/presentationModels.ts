/** Canonical appearance catalog records and external model pose requests. */
import type { ModelInstance, ModelPose } from "@packages/battle-renderer/src/models/modelInstances";
import { type Presentation, type Renderer, type PreparedAppearances } from "@wasm/renderer_browser";
import type { ModelAsset } from "../render/modelAssets";
import {
  APPEARANCE_WORDS,
  Appearance,
  worldViews,
} from "@packages/battle-renderer/src/records.gen";
import { STUMP } from "@packages/battle-renderer/src/scenery/walk";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import type { Articulation } from "@packages/scene-assets/src/articulation";

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

/** Transport complete appearance facts without changing the active catalog. */
export function prepareAppearances(
  renderer: Pick<Renderer, "appearance_draft" | "prepare_appearances">,
  presentation: Presentation,
  assets: readonly ModelAsset[],
  detail: ModelDetail,
): Promise<PreparedAppearances> {
  const draft = renderer.appearance_draft(JSON.stringify(detail));
  try {
    for (const asset of assets) {
      if (asset.placement) draft.set_placement(asset.name, JSON.stringify(asset.placement));
      draft.add_bundle(
        asset.name,
        asset.bundle,
        asset.textureIds,
        asset.textureBytes,
        asset.skeleton,
      );
    }
  } catch (error) {
    draft.free();
    throw error;
  }
  // Ownership moves into Rust even when synchronous GPU admission throws.
  return renderer.prepare_appearances(presentation, draft);
}

// Buildings as the page's static presentation draws them: a `Presentation`
// no battle publishes into, its buildings loaded from template art, framed by
// a lab camera; what each frame's static batches hold, as the renderer keeps
// them (a frame naming its batches unchanged leaves the last ones drawn).
import { ObservationDecoder } from "@web/battle/sim/presentation";
import { presentedCamera } from "@apps/battle-lab/src/presentedFrame";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type {
  BuildingArt,
  BuildingStyle,
  FallenBuilding,
  PlacedBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import {
  Batch,
  BATCH_WORDS,
  INSTANCE_WORDS,
  NONE,
} from "@packages/battle-renderer/src/records.gen";

/** The sun the buildings cast under: azimuth and elevation (rad), and the
 *  cascades' far plane (m). */
export interface PresentedSun {
  azimuth: number;
  elevation: number;
  farM: number;
}

/** One module instance drawn (pass 0) or cast (pass 1). */
export interface DrawnModule {
  /** Its library module. */
  module: number;
  tier: number;
  pass: number;
  /** x, y, z, yaw, then the scale. */
  transform: number[];
  tint: number[];
}

/** `placed` drawn from `art` under `style`, casting under `sun` (none: no
 *  shadows). Loading refuses what has no art by name. */
export function presentBuildings(
  placed: PlacedBuildings,
  art: BuildingArt,
  style: BuildingStyle,
  sun?: PresentedSun,
) {
  const presentation = new ObservationDecoder();
  const world = presentation.loadWorld(
    {
      sun_azimuth: sun?.azimuth ?? 0,
      sun_elevation: sun?.elevation ?? 1,
      cascade_far_m: sun?.farM,
      buildings: style,
    },
    new Map(),
    { placed, art },
  );
  const { kits, modules } = art.library;
  const moduleOf = world.map(({ name, state }) =>
    modules.findIndex((m) => kits[m.kit].appearance === name && m.module === state),
  );
  let held: DrawnModule[] = [];
  return {
    /** Frames `camera` drawing `height` pixels tall; true when what is drawn
     *  or cast changed. */
    look(camera: Camera3DParams, height: number): boolean {
      const width = Math.round(height * camera.aspect);
      const f = presentation.frame(0, presentedCamera({ camera3d: camera, width, height }), {
        selected: [],
        xray: false,
        ghosts: [],
      });
      const words = f.batches.u32;
      const records = f.instances.f32;
      let written = !f.batches.count && held.length > 0;
      if (written) held = [];
      for (let b = 0; b < f.batches.count; b++) {
        const o = b * BATCH_WORDS;
        const first = words[o + Batch.first];
        if (first === NONE) continue;
        if (!written) [written, held] = [true, []];
        for (let i = first; i < first + words[o + Batch.count]; i++) {
          const r = [...records.subarray(i * INSTANCE_WORDS, (i + 1) * INSTANCE_WORDS)];
          held.push({
            module: moduleOf[words[o + Batch.appearance]],
            tier: words[o + Batch.tier],
            pass: words[o + Batch.pass],
            transform: [...r.slice(0, 4), ...r.slice(12, 15)],
            tint: r.slice(8, 11),
          });
        }
      }
      return written;
    },
    /** Every instance the frames so far draw (or cast). */
    drawn(casters = false): DrawnModule[] {
      return held.filter((d) => d.pass === (casters ? 1 : 0));
    },
    /** What the side knows destroyed; returns those refused. */
    setFallen: (fallen: readonly FallenBuilding[]) => presentation.setFallen(fallen),
    /** The walk's stats, by name. */
    stats() {
      const [chunks, coarse, residentChunks, resident, expanded, refused] =
        presentation.buildingStats();
      return { chunks, coarse, residentChunks, resident, expanded, refused };
    },
    free: () => presentation.free(),
  };
}

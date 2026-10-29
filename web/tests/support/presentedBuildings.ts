// Buildings as a lab's presentation draws them: a `Presentation` no battle
// publishes into, its buildings loaded from template art, framed by a lab
// camera, its side knowing what the lab says it knows (`ui.known`); what each
// frame's static batches hold, as the renderer keeps them (a frame naming its
// batches unchanged leaves the last ones drawn).
import { gameLight } from "@apps/battle-lab/src/gameLight";
import { readFileSync } from "node:fs";
import { ScenePresentation, type PresentedSun } from "@web/battle/sim/presentation";
import { templateLibraryPath, type RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { decodeTemplateLibrary } from "@packages/scene-assets/src/templateLibrary";
import { gzipTransport, unpackGzip } from "@packages/scene-assets/src/gzip";
import { presentedCamera } from "@apps/battle-lab/src/presentedFrame";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type {
  BuildingArt,
  BuildingStyle,
  PlacedBuildings,
  SceneBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import {
  Batch,
  BATCH_WORDS,
  INSTANCE_WORDS,
  NONE,
} from "@packages/battle-renderer/src/records.gen";

export type { PresentedSun };

/** The template art that ships (`assets/runtime`), every module's bounds a
 *  box: which module is drawn is the question, not where its edges are. */
export async function shippedBuildingArt(): Promise<BuildingArt> {
  const runtime = (path: string) =>
    readFileSync(new URL(`../../../assets/runtime/${path}`, import.meta.url));
  const catalog = JSON.parse(runtime("catalog.json").toString("utf8")) as RuntimeCatalog;
  const hash = catalog.templates!.library;
  const gzip = gzipTransport(catalog, hash);
  const file = new Uint8Array(runtime(templateLibraryPath(gzip.hash)));
  const bytes = await unpackGzip(file, gzip, hash);
  const library = decodeTemplateLibrary(bytes);
  return {
    library,
    bytes,
    bounds: library.modules.map(() => ({ min: [-1, -1, 0], max: [1, 1, 1] })),
  };
}

/** One module instance drawn (pass 0) or cast (pass 1). */
export interface DrawnModule {
  /** Its library module. */
  module: number;
  tier: number;
  pass: number;
  cascadeMask: number;
  /** x, y, z, yaw, then the scale. */
  transform: number[];
  tint: number[];
}

/** `placed` drawn from `art` under `style`, casting under `sun` (none: no
 *  shadows), each part prop's building and the gutted shells' kinds read
 *  from `knowledge` (none: nothing ever falls). Loading refuses what has no
 *  art by name. */
export function presentBuildings(
  placed: PlacedBuildings,
  art: BuildingArt,
  style: BuildingStyle,
  sun?: PresentedSun,
  knowledge?: Omit<SceneBuildings, "placed">,
) {
  const presentation = new ScenePresentation();
  const world = presentation.loadWorld(
    {
      sun_azimuth: sun?.azimuth ?? 0,
      sun_elevation: sun?.elevation ?? 1,
      buildings: style,
    },
    new Map(),
    { placed, art, partBuilding: knowledge?.partBuilding, shells: knowledge?.shells },
  );
  if (sun) {
    // Flat receiver ground extends past the test buildings; they may cast onto
    // it from outside the camera, just as on terrain in a battle.
    presentation.setShadow(
      {
        ...gameLight,
        sun_azimuth: sun.azimuth,
        sun_elevation: sun.elevation,
        // Pin the receiver partitions: these fixtures test membership, not
        // the current game's shadow-quality tuning.
        cascades: {
          ...gameLight.cascades,
          map_size: 2048,
          split_lambda: 0.5,
          max_far_m: sun.farM,
        },
      },
      new Float32Array(
        [
          [-100000, -100000],
          [100000, -100000],
          [100000, 100000],
          [-100000, -100000],
          [100000, 100000],
          [-100000, 100000],
        ].flatMap(([x, y]) => [x, y, 0, 0, 0, 1, 0, 0, 0, 1]),
      ),
    );
  }
  let known = knowledge?.known ?? [];
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
      const f = presentation.frame(
        0,
        presentedCamera({ camera3d: camera, width, height }),
        { selected: [], xray: false, ghosts: [], known },
        0,
      );
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
            cascadeMask: words[o + Batch.cascade_mask],
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
    /** What the side knows from the next frame on: the lab's stand-in for
     *  the publication's known props. */
    know(next: NonNullable<SceneBuildings["known"]>) {
      known = next;
    },
    /** The buildings the side knows destroyed, as the last frame drew them. */
    fallen: () => presentation.fallenBuildings(),
    /** The walk's stats, by name. */
    stats() {
      const w = presentation.buildingWalk();
      return {
        chunks: w.chunks,
        coarse: w.coarse,
        residentChunks: w.residentChunks,
        resident: w.pool.used,
        expanded: w.expandedRows,
        refused: w.refused,
      };
    },
    /** The units' and buildings' tier thresholds over their authored values. */
    setDetailScale: (units: number, buildings: number) =>
      presentation.setDetailScale(units, buildings),
    free: () => presentation.free(),
  };
}

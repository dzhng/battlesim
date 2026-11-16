// What one side knows of the map's buildings, apart from everything else it
// knows: the buildings its presentation draws, the boxes its camera keeps
// clear of, and the probes that report both.
import { useMemo } from "react";
import { knownStanding, type PropBox } from "@packages/battle-renderer/src/models/propAppearance";
import {
  FRAME_FLOATS,
  type SceneBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import {
  buildingObstacles,
  buildingPartProps,
  knownOf,
} from "@packages/battle-renderer/src/buildingObstacles";
import type { SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";
import { buildingRemains } from "@packages/scene-assets/src/authority";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { KnownPropView } from "@web/battle/sim/observationView";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import { useFeed } from "@web/feed";
import type { MapBuildings, StaticWorld } from "./useStaticWorld";

/** `world`'s buildings (`map`) as the side whose known props are `knownKey`
 *  (their JSON) knows them: what changes only when that knowledge does.
 *  `presentation` says which have fallen, for the probes. */
export function useKnownBuildings(
  world: StaticWorld | null,
  map: MapBuildings | null,
  knownKey: string,
  units: UnitCatalog,
  surfaceZ: SurfaceHeight,
  presentation: ScenePresentation | null,
) {
  const placedProps = map?.props ?? null;
  const drawnBuildings = map?.index ?? null;
  const buildingParts = useMemo(() => world && buildingPartProps(world.exports.buildings), [world]);
  const knownBuildingsKey = useMemo(
    () =>
      buildingParts &&
      JSON.stringify(knownOf(JSON.parse(knownKey) as KnownPropView[], buildingParts)),
    [knownKey, buildingParts],
  );
  // The map's buildings, which the battle's presentation draws in the state
  // the side's publication knows them in.
  const buildings = useMemo<SceneBuildings | null>(
    () =>
      drawnBuildings && {
        placed: drawnBuildings.placed,
        partBuilding: drawnBuildings.partBuilding,
        shells: buildingRemains(units).shells,
      },
    [drawnBuildings, units],
  );
  const buildingsFeed = useFeed(buildings);
  // What the camera keeps clear of: the ground, and every building part the
  // side knows stands (a fallen one's remains once it has seen the fall).
  const cameraObstacles = useMemo(() => {
    if (!placedProps || !buildingParts || !knownBuildingsKey) return null;
    const started = performance.now();
    const view = buildingObstacles(
      placedProps,
      JSON.parse(knownBuildingsKey) as KnownPropView[],
      buildingParts,
      surfaceZ,
    );
    return { view, buildMs: performance.now() - started };
  }, [placedProps, buildingParts, knownBuildingsKey, surfaceZ]);
  const cameraObstaclesFeed = useFeed(cameraObstacles?.view ?? null);
  const probes = {
    /** The camera's obstacles: how many boxes, and what indexing them took. */
    cameraObstacles: () =>
      cameraObstacles && { boxes: cameraObstacles.view.count, buildMs: cameraObstacles.buildMs },
    /** The map's buildings: each one's template, owner, frame, the state the
     *  side draws it in, its parts as the map has them (`authored`) and as the
     *  side knows them (`parts`: the remains or the shell of one it has seen
     *  destroyed), which are the boxes its camera keeps clear of. */
    buildings: () => {
      if (!drawnBuildings || !knownBuildingsKey) return [];
      const fallen = presentation?.fallenBuildings() ?? [];
      const state = new Map(fallen.map((f) => [f.building, f.state]));
      const known = JSON.parse(knownBuildingsKey) as KnownPropView[];
      const { placed } = drawnBuildings;
      const box = (p: PropBox) => ({
        kind: p.kind,
        center: p.center,
        baseZ: p.baseZ,
        yaw: p.yaw,
        half: p.half,
      });
      return drawnBuildings.parts.map((parts, i) => ({
        template: placed.templates[placed.template[i]],
        owner: placed.owners[i],
        frame: [...placed.frames.subarray(i * FRAME_FLOATS, (i + 1) * FRAME_FLOATS)],
        fallen: state.has(i),
        state: state.get(i) ?? "intact",
        authored: parts.map(box),
        parts: knownStanding(parts, known, new Set(parts.map((p) => p.id))).map(box),
      }));
    },
  };
  /** The side's known buildings and camera obstacles, for a viewport. */
  const known = { buildings: buildingsFeed, obstacles: cameraObstaclesFeed };
  return { known, probes };
}

// What the camera keeps clear of on a map: the ground, and every building
// part the viewing side knows stands. Built from the public map and the
// side's observation alone, as the drawn buildings are, so a fall the side
// has not seen changes neither what it sees nor where its camera can go.
// Trees and other props never block the camera.
import {
  createCameraObstacles,
  type CameraObstacles,
} from "@packages/renderer-core/src/cameraObstacles";
import { knownStanding, type KnownProp, type MapProp } from "./models/propAppearance";
import type { PublicBuildings } from "./worldMesh";

/** The prop of every part of every building on the map. */
export function buildingPartProps(buildings: PublicBuildings): Set<number> {
  return new Set(buildings.buildings.flatMap((b) => b.parts.map((p) => p.prop)));
}

/** The side's knowledge of the building parts `parts`, out of all it knows:
 *  the camera's obstacles change only when this does. */
export function knownOf<Known extends KnownProp>(
  known: readonly Known[],
  parts: ReadonlySet<number>,
): Known[] {
  return known.filter((k) => k.authoredProp !== null && parts.has(k.authoredProp));
}

/** The obstacle view of `props`' building `parts` as `known` has them, over
 *  the ground `groundAt` gives. */
export function buildingObstacles(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  parts: ReadonlySet<number>,
  groundAt: (x: number, y: number) => number,
): CameraObstacles {
  return createCameraObstacles(knownStanding(props, known, parts), groundAt);
}

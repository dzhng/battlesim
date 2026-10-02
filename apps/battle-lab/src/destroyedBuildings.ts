// How the simulation ends a building, as the labs read it from the catalog's
// building rows: which remains say a building stands gutted (a battle's
// observation is classified with these), and, for a lab with no battle, what
// a side that saw a building destroyed would have been published.
import { buildingCollapse, buildingRemains } from "@packages/scene-assets/src/authority";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { templateArt, type TemplateArtLibrary } from "@packages/scene-assets/src/templateLibrary";
import { ruinHeight } from "@packages/scene-assets/src/templateSource";
import {
  FRAME_FLOATS,
  type BuildingIndex,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { KnownProp } from "@packages/battle-renderer/src/models/propAppearance";

/** The prop types a gutted building's parts become (`fallenBuildings`). */
export const gameGuttedShells = buildingRemains(UNITS).shells;

/**
 * What a side that saw building `building` of `index` destroyed knows of it,
 * by the simulation's rule and not by a battle: every part replaced on its
 * own plan, by a gutted shell at its full height where the template stands
 * gutted, else by remains at the building's one ruin height. Which of the two
 * is the template's own damage state in `library`: the bake holds each
 * template to the state the rule destroys it into.
 */
export function seenDestroyed(
  index: BuildingIndex,
  building: number,
  library: TemplateArtLibrary,
): KnownProp[] {
  const { placed } = index;
  const parts = index.parts[building];
  const collapse = buildingCollapse(UNITS);
  const gutted =
    templateArt(library, placed.templates[placed.template[building]]).states.gutted !== undefined;
  const ground = placed.frames[building * FRAME_FLOATS + 2];
  const remainsM =
    collapse &&
    ruinHeight(
      parts.map((part) => ({
        id: String(part.id),
        center: [part.center[0], part.center[1]],
        yaw: part.yaw,
        half_extents: [part.half[0], part.half[1], part.half[2]],
        base_z: part.baseZ - ground,
      })),
      collapse,
    );
  return parts.map((part) => {
    const ends = UNITS.view.props[part.kind]?.destroyed;
    if (typeof ends !== "object" || !ends.into.building || remainsM === null)
      throw new Error(`prop type "${part.kind}" does not end as a building does`);
    return {
      ...part,
      kind: gutted ? ends.into.building.gutted_prop : ends.into.prop,
      half: gutted ? part.half : [part.half[0], part.half[1], remainsM / 2],
      authoredProp: part.id,
      replaces: part.id,
    };
  });
}

// How the simulation ends a building, as the lab app reads it from the
// catalog's building rows: what a side that saw a building destroyed would
// have been published, for a lab with no battle. The presentation reads the
// building's state from those props as it reads a publication's.
import { buildingCollapse } from "@packages/scene-assets/src/authority";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { templateArt, type TemplateArtLibrary } from "@packages/scene-assets/src/templateLibrary";
import { ruinHeight } from "@packages/scene-assets/src/templateSource";
import {
  FRAME_FLOATS,
  type BuildingIndex,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { KnownProp } from "@packages/battle-renderer/src/models/propAppearance";

/**
 * What a side that saw building `building` of `index` destroyed knows of it,
 * by the simulation's rule and not by a battle: every part replaced on its
 * own plan, by a gutted shell at its full height where the template stands
 * gutted, else by remains at the building's one ruin height. Which of the two
 * is the template's own damage state in `library`: the bake holds each
 * template to the state the rule destroys it into.
 */
export function seenDestroyed(
  units: UnitCatalog,
  index: BuildingIndex,
  building: number,
  library: TemplateArtLibrary,
): KnownProp[] {
  const { placed } = index;
  const parts = index.parts[building];
  const collapse = buildingCollapse(units);
  const gutted =
    templateArt(library, placed.templates[placed.template[building]]).states.gutted !== undefined;
  const ground = placed.frames[building * FRAME_FLOATS + 2];
  const top = Math.max(...parts.map((part) => part.baseZ + 2 * part.half[2]));
  const remainsM = collapse && ruinHeight(top - ground, collapse);
  return parts.map((part) => {
    const ends = units.view.props[part.kind]?.destroyed;
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

// A tree, hedgerow or dressing appearance as the scenery draws it: the static bundle's
// summer state, from the one loader (`AppearanceLibrary`), turned into the
// renderer's vertex format per tier. The bundle is the unit; placement and
// the biome's tint only instance it.
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { MeshData, StaticBundle } from "@packages/scene-assets/src/schema";
import { VERTEX_FLOATS, type Mesh } from "../mesh";
import type { KindSize } from "./placement";

/** The state scenery draws (winter will be the next biome's). */
export const SCENERY_STATE = "summer";
/** The material whose triangles are foliage: shaded with leaf clumps, fogged
 *  as a volume. Every other material (bark) is a plain surface. */
export const FOLIAGE_MATERIAL = "leaves";

/** The installed scenery appearances `names` needs, by name; throws naming
 *  any that is missing or is not of one of the scenery `kinds`. */
export function sceneryAppearances(
  installed: InstalledAppearances,
  names: readonly string[],
  kinds: readonly string[],
): Map<string, StaticBundle> {
  const out = new Map<string, StaticBundle>();
  for (const name of names) {
    const entry = installed.appearances.get(name);
    if (!entry) throw new Error(`scenery: appearance "${name}" is not in the catalog (asset bake)`);
    if (
      entry.bundle.kind !== "static" ||
      entry.unit !== "scenery" ||
      !kinds.includes(entry.scenery ?? "") ||
      !entry.bundle.states.some((s) => s.name === SCENERY_STATE)
    )
      throw new Error(
        `scenery: appearance "${name}" is not a ${kinds.join(" or ")} with a summer state`,
      );
    out.set(name, entry.bundle);
  }
  return out;
}

function tiers(bundle: StaticBundle): MeshData[] {
  return bundle.states.find((s) => s.name === SCENERY_STATE)!.tiers;
}

/** Unscaled height and crown reach from the trunk's axis, over the finest tier. */
export function kindSize(bundle: StaticBundle): KindSize {
  const p = tiers(bundle)[0].positions;
  let height = 0,
    radius = 0;
  for (let i = 0; i < p.length; i += 3) {
    height = Math.max(height, p[i + 2]);
    radius = Math.max(radius, Math.hypot(p[i], p[i + 1]));
  }
  return { height, radius };
}

/** One tier as triangles in the renderer's vertex format: linear albedo
 *  (material colour times vertex colour) in rgb, 1 in alpha for foliage. */
export function tierMesh(bundle: StaticBundle, tier: number): Mesh {
  const mesh = tiers(bundle)[tier];
  const out = new Float32Array(mesh.indices.length * VERTEX_FLOATS);
  let o = 0;
  for (const draw of mesh.draws) {
    const material = bundle.materials[draw.material];
    const foliage = material.name === FOLIAGE_MATERIAL ? 1 : 0;
    for (let i = draw.first; i < draw.first + draw.count; i++) {
      const v = mesh.indices[i];
      out[o] = mesh.positions[v * 3];
      out[o + 1] = mesh.positions[v * 3 + 1];
      out[o + 2] = mesh.positions[v * 3 + 2];
      for (let c = 0; c < 3; c++) out[o + 3 + c] = mesh.normals[v * 4 + c] / 32767;
      for (let c = 0; c < 3; c++)
        out[o + 6 + c] = material.base_color[c] * (mesh.colors[v * 4 + c] / 255);
      out[o + 9] = foliage;
      o += VERTEX_FLOATS;
    }
  }
  return out;
}

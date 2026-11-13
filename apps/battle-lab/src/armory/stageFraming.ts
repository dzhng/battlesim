// What the Armory's camera frames once a unit stands on its spot: the unit's
// bounds at rest (a hull's model as it stands; a squad's soldiers where the
// simulation scattered them, each a soldier tall), the angle its spot looks
// from, and what the eye keeps clear of: the boxes of the unit and the yard's
// props, each as its art is authored, and the yard's ground.
import type { ObstacleBox } from "@packages/renderer-core/src/cameraObstacles";
import type { Bounds } from "@packages/scene-assets/src/schema";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { ResolvedRules } from "@web/battle/catalog/compose";
import type { ShowcaseStage, StageArt } from "@web/battle/render/showcaseStage";
import { boundsBox, type Framing } from "./armoryCamera";
import { restBounds, worldBox } from "./stageProbes";
import { yard, yardGround, yardModels, type Spot } from "./yard";

/** How far a standing soldier reaches round his feet, metres. */
const SOLDIER_REACH_M = 0.5;

/** The framing for what `stage` stands on `spot`, drawn from `art`. */
export function framingOf(
  stage: ShowcaseStage,
  art: StageArt,
  units: UnitCatalog,
  rules: Pick<ResolvedRules, "physics">,
  spot: Spot,
): Framing {
  const props = yardModels(yard, units.view.props, art.appearances).flatMap((m) => {
    const b = restBounds(art.appearances, m.appearance);
    if (!b) return [];
    const [cx, cy] = [(b.min[0] + b.max[0]) / 2, (b.min[1] + b.max[1]) / 2];
    const [c, s] = [Math.cos(m.yaw), Math.sin(m.yaw)];
    const box: ObstacleBox = {
      center: [m.x + c * cx - s * cy, m.y + s * cx + c * cy],
      yaw: m.yaw,
      half: [(b.max[0] - b.min[0]) / 2, (b.max[1] - b.min[1]) / 2, (b.max[2] - b.min[2]) / 2],
      baseZ: m.z + b.min[2],
    };
    return [box];
  });
  const standing = stage.standing();
  let bounds: Bounds | null = null;
  if (standing && standing.soldiers.length > 0) {
    const h = rules.physics.soldier_height_m;
    const xs = standing.soldiers.map((s) => s.at[0]);
    const ys = standing.soldiers.map((s) => s.at[1]);
    bounds = {
      min: [Math.min(...xs) - SOLDIER_REACH_M, Math.min(...ys) - SOLDIER_REACH_M, 0],
      max: [Math.max(...xs) + SOLDIER_REACH_M, Math.max(...ys) + SOLDIER_REACH_M, h],
    };
  } else if (standing) {
    const names = [units.type(standing.kind).appearance ?? []].flat();
    const at = standing.at;
    bounds = worldBox(
      names.map((name) => ({ name, at: [at[0], at[1], at[2]], yaw: standing.yaw, pose: [] })),
      art.appearances,
    );
  }
  return {
    bounds,
    pose: spot.camera,
    boxes: bounds ? [...props, boundsBox(bounds)] : props,
    ground: yardGround(),
  };
}

// How a model takes fog of war: the one decision, and the fragment term that
// follows it. Units are drawn by identification and never fogged;
// buildings and props are fogged face by face; corpses are seen or unseen
// whole, as the ground they lie on. A prop movers stand on (a bridge deck)
// is fogged face by face and takes the ground paint. The frame binds the fog
// group a class names (`FogVisibility`: units, faces, ground, paintedFaces)
// for each of the models' draws.
import { tgpu, d } from "typegpu";
import { fogIsGround, fogTerm } from "../frame/fogTerm";
import type { ModelPose } from "./modelInstances";

/** Which fog group a draw binds (`FogVisibility` layers). */
export type ModelFog = "units" | "faces" | "ground" | "paintedFaces";
export const UNITS = 0;
const FACES = 1;
const GROUND = 2;
const PAINTED_FACES = 3;
export const FOG_CLASSES = 4;

/**
 * The one decision of how a model takes fog. A posed body or vehicle is a
 * unit, drawn only while identified, so it is never fogged.
 * Buildings and props are the world's faces, fogged face by face. A corpse is
 * remains lying on the ground: seen or unseen whole, as the ground under it
 * is, so a body in plain view never splits into seen and unseen faces.
 */
export function modelFog(pose: ModelPose): ModelFog {
  if (pose.kind === "skinned" || pose.kind === "articulated") return "units";
  // A prop movers stand on takes the ground paint (`StaticModelPose.ground`).
  if (pose.kind === "static" && pose.ground) return "paintedFaces";
  return pose.kind === "corpse" ? "ground" : "faces";
}
export const FOG_INDEX: Record<ModelFog, number> = {
  units: UNITS,
  faces: FACES,
  ground: GROUND,
  paintedFaces: PAINTED_FACES,
};
/** Fog for a model's fragment: through the ground group, whole at its foot
 *  as the ground there is (corpses); otherwise face by face (the units' group
 *  answers seen everywhere). */
export const modelSeen = tgpu.fn(
  [d.vec3f, d.vec3f, d.vec3f, d.vec2f],
  d.f32,
)((world, normal, anchor, pixel) => {
  "use gpu";
  if (fogIsGround()) {
    return fogTerm(anchor, d.vec3f(0), pixel, true);
  }
  return fogTerm(world, normal, pixel, false);
});

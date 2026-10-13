// How a model takes fog of war: the fog groups a draw binds, and the
// fragment term that follows. Which group a model takes is Rust's
// (`view::model_fog`, as `Batch.fog`); everything inside an occluding
// structure's box takes that structure's fog whole (`fogWholeSeen`, in
// FogTerm itself). The frame binds the fog group a class names
// (`FogVisibility`: units, faces, ground, paintedFaces) for each of the
// models' draws.
import { tgpu, d } from "typegpu";
import { fogIsGround, fogTerm } from "../frame/fogTerm";
import { FOG_FACES, FOG_GROUND, FOG_PAINTED_FACES, FOG_UNITS } from "../records.gen";

/** Which fog group a draw binds (`FogVisibility` layers). */
export type ModelFog = "units" | "faces" | "ground" | "paintedFaces";
export const FOG_CLASSES = 4;

/** Each class's group, numbered as a batch names it (`Batch.fog`). */
export const FOG_INDEX: Record<ModelFog, number> = {
  units: FOG_UNITS,
  faces: FOG_FACES,
  ground: FOG_GROUND,
  paintedFaces: FOG_PAINTED_FACES,
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

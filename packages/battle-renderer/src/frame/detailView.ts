// What a frame's camera means for a lab's own models' detail: where the eye
// is, how many pixels a metre covers at a metre's distance, and the view's
// side planes, read off the presentation frame's `View` record. The
// presentation's batches carry their tiers already (`crates/presentation`'s
// `view.rs`); the models layer chooses a lab's models' tiers from this.
import { vec3, type Mat4, type Vec3 } from "math";
import { frustum, type Frustum } from "math/shapes";
import { View } from "../records.gen";

export interface DetailView {
  eye: Vec3;
  /** Device pixels per metre at one metre's distance: `height / (2·tan(fovY/2))`. */
  pixelsPerMetre: number;
  /** The camera's four side planes (near and far are not tested). */
  sides: Frustum;
}

export function createDetailView(): DetailView {
  return { eye: vec3.create(), pixelsPerMetre: 1, sides: frustum.create() };
}

/** Pose `out` for a frame's `View` record (`View`'s words). */
export function setDetailView(out: DetailView, view: Float32Array): DetailView {
  vec3.set(out.eye, view[View.eye], view[View.eye + 1], view[View.eye + 2]);
  out.pixelsPerMetre = view[View.size + 3];
  frustum.setFromViewProjectionMatrixSides(
    out.sides,
    view.subarray(View.projection, View.projection + 16) as unknown as Mat4,
    view.subarray(View.view, View.view + 16) as unknown as Mat4,
  );
  return out;
}

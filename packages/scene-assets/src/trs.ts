// Node transforms as translation / rotation / scale, on top of the `math`
// package. glTF nodes, joint binds and articulated binds are all Trs.

import { mat4, quat, vec3, type Mat4, type Quat, type Vec3 } from "math";

/** A node's local transform: translation, rotation (x, y, z, w), scale. */
export interface Trs {
  t: Vec3;
  r: Quat;
  s: Vec3;
}

export function trsMatrix(trs: Trs): Mat4 {
  return mat4.fromRotationTranslationScale(mat4.create(), trs.r, trs.t, trs.s);
}

/** `a · b` into a new matrix. */
export const mul = (a: Mat4, b: Mat4): Mat4 => mat4.multiply(mat4.create(), a, b);

/** The inverse of an affine transform; a singular one is an art error upstream, so it throws. */
export function inverse(a: Mat4): Mat4 {
  const out = mat4.invert(mat4.create(), a);
  if (!out) throw new Error("singular transform");
  return out;
}

export const pointAt = (m: Mat4, p: Vec3): Vec3 => vec3.transformMat4(vec3.create(), p, m);

// Relative tolerance for a TRS rebuild to count as exact: float32 source data
// round-trips to about 1e-7, and shear worth rejecting is far above 1e-5.
const TRS_REBUILD_TOLERANCE = 1e-5;

/** Decompose an affine matrix; null for shear, mirroring or a zero scale. */
export function decomposeTrs(m: Mat4): Trs | null {
  if (!(mat4.determinant(m) > 0)) return null;
  const t = vec3.create();
  const s = vec3.create();
  const r = quat.create();
  mat4.decompose(r, t, s, m);
  quat.normalize(r, r);
  const rebuilt = trsMatrix({ t, r, s });
  for (let i = 0; i < 16; i++)
    if (Math.abs(rebuilt[i] - m[i]) > TRS_REBUILD_TOLERANCE * Math.max(1, Math.abs(m[i])))
      return null;
  return { t, r, s };
}

// glTF exporters write uniform scales that differ in the 7th digit.
const UNIFORM_SCALE_TOLERANCE = 1e-4;

export function isUniformScale(s: Vec3): boolean {
  const max = Math.max(s[0], s[1], s[2]);
  const min = Math.min(s[0], s[1], s[2]);
  return min > 0 && max - min <= max * UNIFORM_SCALE_TOLERANCE;
}

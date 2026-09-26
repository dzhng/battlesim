// Minimal column-major 4×4 matrix math — the one matrix primitive for the real
// 3D camera (camera3d.ts). Column-major to match WGSL `mat4x4<f32>` and the
// GPU uniform packing, so a matrix built here uploads verbatim. Dependency-free
// on purpose: renderer-core stays portable (no gl-matrix / babylon pull-in).
//
// Storage: `m[col * 4 + row]`; element (row r, col c) = `m[c * 4 + r]`. A matrix
// applied as a linear map is `transformVec4(m, v)`.

export type Mat4 = Float32Array; // length 16, column-major
export type Vec3 = readonly [number, number, number];
type Vec4 = readonly [number, number, number, number];

export function identity(): Mat4 {
  const m = new Float32Array(16);
  m[0] = m[5] = m[10] = m[15] = 1;
  return m;
}

// C = A · B as linear maps: (A·B) v = A (B v).
export function multiply(a: Mat4, b: Mat4): Mat4 {
  const out = new Float32Array(16);
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += a[k * 4 + r] * b[c * 4 + k];
      out[c * 4 + r] = s;
    }
  }
  return out;
}

export function transformVec4(m: Mat4, v: Vec4): [number, number, number, number] {
  const out: [number, number, number, number] = [0, 0, 0, 0];
  for (let r = 0; r < 4; r++) {
    out[r] = m[r] * v[0] + m[4 + r] * v[1] + m[8 + r] * v[2] + m[12 + r] * v[3];
  }
  return out;
}

// Full 4×4 inverse (column-major), cofactor method. Returns null when singular.
export function invert(m: Mat4): Mat4 | null {
  const a00 = m[0], a01 = m[1], a02 = m[2], a03 = m[3];
  const a10 = m[4], a11 = m[5], a12 = m[6], a13 = m[7];
  const a20 = m[8], a21 = m[9], a22 = m[10], a23 = m[11];
  const a30 = m[12], a31 = m[13], a32 = m[14], a33 = m[15];

  const b00 = a00 * a11 - a01 * a10;
  const b01 = a00 * a12 - a02 * a10;
  const b02 = a00 * a13 - a03 * a10;
  const b03 = a01 * a12 - a02 * a11;
  const b04 = a01 * a13 - a03 * a11;
  const b05 = a02 * a13 - a03 * a12;
  const b06 = a20 * a31 - a21 * a30;
  const b07 = a20 * a32 - a22 * a30;
  const b08 = a20 * a33 - a23 * a30;
  const b09 = a21 * a32 - a22 * a31;
  const b10 = a21 * a33 - a23 * a31;
  const b11 = a22 * a33 - a23 * a32;

  let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
  if (det === 0) return null;
  det = 1.0 / det;

  const out = new Float32Array(16);
  out[0] = (a11 * b11 - a12 * b10 + a13 * b09) * det;
  out[1] = (a02 * b10 - a01 * b11 - a03 * b09) * det;
  out[2] = (a31 * b05 - a32 * b04 + a33 * b03) * det;
  out[3] = (a22 * b04 - a21 * b05 - a23 * b03) * det;
  out[4] = (a12 * b08 - a10 * b11 - a13 * b07) * det;
  out[5] = (a00 * b11 - a02 * b08 + a03 * b07) * det;
  out[6] = (a32 * b02 - a30 * b05 - a33 * b01) * det;
  out[7] = (a20 * b05 - a22 * b02 + a23 * b01) * det;
  out[8] = (a10 * b10 - a11 * b08 + a13 * b06) * det;
  out[9] = (a01 * b08 - a00 * b10 - a03 * b06) * det;
  out[10] = (a30 * b04 - a31 * b02 + a33 * b00) * det;
  out[11] = (a21 * b02 - a20 * b04 - a23 * b00) * det;
  out[12] = (a11 * b07 - a10 * b09 - a12 * b06) * det;
  out[13] = (a00 * b09 - a01 * b07 + a02 * b06) * det;
  out[14] = (a31 * b01 - a30 * b03 - a32 * b00) * det;
  out[15] = (a20 * b03 - a21 * b01 + a22 * b00) * det;
  return out;
}

// Right-handed lookAt: world → eye space, camera looking down its -Z. `up` is a
// hint; a robust fallback keeps it well-defined when looking near-vertical
// (straight-down top-down camera).
export function lookAt(eye: Vec3, target: Vec3, up: Vec3 = [0, 0, 1]): Mat4 {
  let fx = target[0] - eye[0], fy = target[1] - eye[1], fz = target[2] - eye[2];
  const fl = Math.hypot(fx, fy, fz) || 1;
  fx /= fl; fy /= fl; fz /= fl;

  // If forward is nearly parallel to `up`, pick a different up so the cross
  // products stay stable (top-down: forward ≈ -Z, default up +Z would collapse).
  let ux = up[0], uy = up[1], uz = up[2];
  if (Math.abs(fx * ux + fy * uy + fz * uz) > 0.999) {
    ux = 0; uy = 1; uz = 0;
  }

  // s = f × up (right), then true up u = s × f.
  let sx = fy * uz - fz * uy, sy = fz * ux - fx * uz, sz = fx * uy - fy * ux;
  const sl = Math.hypot(sx, sy, sz) || 1;
  sx /= sl; sy /= sl; sz /= sl;
  const tux = sy * fz - sz * fy, tuy = sz * fx - sx * fz, tuz = sx * fy - sy * fx;

  const m = new Float32Array(16);
  // Columns of the view matrix (column-major storage).
  m[0] = sx; m[1] = tux; m[2] = -fx; m[3] = 0;
  m[4] = sy; m[5] = tuy; m[6] = -fy; m[7] = 0;
  m[8] = sz; m[9] = tuz; m[10] = -fz; m[11] = 0;
  m[12] = -(sx * eye[0] + sy * eye[1] + sz * eye[2]);
  m[13] = -(tux * eye[0] + tuy * eye[1] + tuz * eye[2]);
  m[14] = fx * eye[0] + fy * eye[1] + fz * eye[2];
  m[15] = 1;
  return m;
}

// Reverse-Z perspective for WebGPU clip space (NDC z ∈ [0,1], y up), right-handed
// view space (looking down -Z). Near maps to depth 1, far to depth 0 — the
// precision-optimal convention. `far` omitted → infinite far plane (far → depth 0
// in the limit), which the water plane's long span wants.
export function perspectiveReverseZ(fovY: number, aspect: number, near: number, far?: number): Mat4 {
  const f = 1 / Math.tan(fovY / 2);
  const m = new Float32Array(16);
  m[0] = f / aspect;
  m[5] = f;
  m[11] = -1;
  // Row 2 (depth): finite far uses A,B; infinite far is the limit A=0, B=near.
  const A = far === undefined ? 0 : near / (far - near);
  const B = far === undefined ? near : (near * far) / (far - near);
  m[10] = A;
  m[14] = B;
  return m;
}

/** Orthographic WebGPU projection, right-handed view space, near→1 and far→0. */
export function orthographicReverseZ(
  left: number,
  right: number,
  top: number,
  bottom: number,
  near: number,
  far: number,
): Mat4 {
  const m = identity();
  m[0] = 2 / (right - left);
  m[5] = 2 / (top - bottom);
  m[10] = 1 / (far - near);
  m[12] = -(right + left) / (right - left);
  m[13] = -(top + bottom) / (top - bottom);
  m[14] = far / (far - near);
  return m;
}

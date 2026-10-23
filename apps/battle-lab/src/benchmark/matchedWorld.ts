import type { WorldExports } from "@packages/battle-renderer/src/worldMesh";

/** Append disconnected remote terrain, never recompile/resample the admitted patch.
 * Static populations and the local height-query grid retain their identities.
 * The complete mesh, including remote height, reaches production shadow fitting.
 */
export function appendRemoteTerrain(
  local: WorldExports,
  copies: number,
  elevation: number,
): WorldExports {
  if (!copies) return local;
  const positions = new Float32Array(local.positions.length * (copies + 1));
  const indices = new Uint32Array(local.indices.length * (copies + 1));
  const triangleSurfaces = new Uint8Array(local.triangleSurfaces.length * (copies + 1));
  const bounds = [...local.extents.physical] as [number, number, number, number];
  const step = Math.max(8192, bounds[2] - bounds[0] + 8192);
  for (let copy = 0; copy <= copies; copy++) {
    const offset = copy * local.positions.length;
    positions.set(local.positions, offset);
    for (let i = 0; i < local.positions.length; i += 3) {
      if (copy) {
        positions[offset + i] += copy * step;
        positions[offset + i + 2] += elevation;
      }
      bounds[2] = Math.max(bounds[2], positions[offset + i]);
    }
    for (let i = 0; i < local.indices.length; i++)
      indices[copy * local.indices.length + i] = local.indices[i] + offset / 3;
    triangleSurfaces.set(local.triangleSurfaces, copy * local.triangleSurfaces.length);
  }
  return {
    ...local,
    positions,
    indices,
    triangleSurfaces,
    extents: { ...local.extents, physical: bounds, rendered: bounds },
  };
}

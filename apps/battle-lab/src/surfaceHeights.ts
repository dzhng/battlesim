// The map's surface height, as the overlays drape their marks on it.
import type { SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";
import type { StaticWorld } from "./useStaticWorld";

/** Ground heights the page remembers before starting afresh. */
const SURFACE_HEIGHTS_MAX = 1 << 20;

/** The height of `world`'s surface at a point (0 with no world yet). The
 *  page's world is the map as loaded: a point's height never changes, and
 *  the overlays drape their marks at the same points every publication, so
 *  each height is asked of the module once, until the memory is full. */
export function surfaceHeights(world: StaticWorld | null): SurfaceHeight {
  let heights = new Map<number, Map<number, number>>();
  let size = 0;
  return (x, y) => {
    let row = heights.get(x);
    let z = row?.get(y);
    if (z === undefined) {
      z = world?.view.surface_at(x, y)[0] ?? 0;
      if (size >= SURFACE_HEIGHTS_MAX) {
        heights = new Map();
        size = 0;
        row = undefined;
      }
      if (!row) heights.set(x, (row = new Map()));
      row.set(y, z);
      size++;
    }
    return z;
  };
}

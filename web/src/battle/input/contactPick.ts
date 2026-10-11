/** Which contact the pointer picks. A contact not aloft is an area on the ground
 *  (its centre and radius, `ContactView`): the pointer picks it wherever its
 *  ground point falls inside that disc. An aloft contact hangs at its
 *  height: the pointer's ray picks it where it passes through the sphere of
 *  that radius round its centre, whatever ground lies beneath. Where areas
 *  overlap, the one whose centre is nearest the pointer wins. */
import type { ContactView } from "../sim/observation";

/** A pointer's world ray: from the eye, along `dir` (any length). */
export interface PickRay {
  origin: readonly number[];
  dir: readonly number[];
}

export function contactUnder(
  contacts: readonly Pick<ContactView, "id" | "center" | "radius" | "aloft">[],
  ground: readonly [number, number] | null,
  ray: PickRay | null = null,
): number | null {
  let best: number | null = null;
  let bestD2 = Infinity;
  for (const c of contacts) {
    const d2 = c.aloft ? rayDistance2(c.center, ray) : groundDistance2(c.center, ground);
    if (d2 <= c.radius * c.radius && d2 < bestD2) {
      bestD2 = d2;
      best = c.id;
    }
  }
  return best;
}

/** Squared distance across the ground from the centre to the ground point. */
function groundDistance2(center: readonly number[], ground: readonly [number, number] | null) {
  if (!ground) return Infinity;
  const dx = ground[0] - center[0],
    dy = ground[1] - center[1];
  return dx * dx + dy * dy;
}

/** Squared distance from the centre to the nearest point of the ray ahead
 *  of the eye; a centre behind the eye is never picked. */
function rayDistance2(center: readonly number[], ray: PickRay | null) {
  if (!ray) return Infinity;
  const [ox, oy, oz] = ray.origin,
    [dx, dy, dz] = ray.dir;
  const cx = center[0] - ox,
    cy = center[1] - oy,
    cz = center[2] - oz;
  const along = (cx * dx + cy * dy + cz * dz) / (dx * dx + dy * dy + dz * dz);
  if (!(along >= 0)) return Infinity;
  const px = cx - dx * along,
    py = cy - dy * along,
    pz = cz - dz * along;
  return px * px + py * py + pz * pz;
}

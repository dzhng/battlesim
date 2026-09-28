/** Which contact a ground point picks: a contact is an area on the ground
 *  (its centre and radius, `ContactView`), so the pointer picks it wherever
 *  its ground point falls inside that disc. Where areas overlap, the one
 *  whose centre is nearest wins. */
import type { ContactView } from "../sim/observation";

export function contactUnder(
  contacts: readonly Pick<ContactView, "id" | "center" | "radius">[],
  ground: readonly [number, number] | null,
): number | null {
  if (!ground) return null;
  let best: number | null = null;
  let bestD2 = Infinity;
  for (const c of contacts) {
    const dx = ground[0] - c.center[0],
      dy = ground[1] - c.center[1];
    const d2 = dx * dx + dy * dy;
    if (d2 <= c.radius * c.radius && d2 < bestD2) {
      bestD2 = d2;
      best = c.id;
    }
  }
  return best;
}

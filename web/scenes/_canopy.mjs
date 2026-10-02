// How far the canopy closes, checked at the ground rig's stations over the
// village's west wood: the crowns hide most of the forest floor and leave
// some of it in view. The measure is the floor's own class in the terrain's
// mask, with the trees and without: the share of the wood's floor pixels the
// crowns leave seen. It says nothing of one squad: men standing under two
// crowns are all hidden in a wood that passes (the x-ray draws them).
import { decode } from "./_png.mjs";
import { classAt, openStations, shoot } from "./_groundStations.mjs";

/** The wood from the play camera and from higher: the default distance and
 *  twice it. */
const STATIONS = ["forest-65", "forest-120"];
/** The share of the wood's floor seen through the crowns: under the first
 *  the wood reads as a lid over its units, over the second as an orchard. */
const FLOOR_SEEN = [0.12, 0.4];

/** How many of `mask`'s pixels are forest floor. */
function floorPixels(mask) {
  let floor = 0;
  for (let y = 0; y < mask.height; y++)
    for (let x = 0; x < mask.width; x++) if (classAt(mask, x, y)?.forest === "inside") floor++;
  return floor;
}

export async function canopyClosure(ctx) {
  const page = await openStations(ctx, "village");
  const seen = {};
  for (const station of STATIONS) {
    const mask = async (trees) =>
      floorPixels(
        decode(
          await shoot(page, "village", station, { view: "ground-classes", grass: false, trees }),
        ),
      );
    const [under, bare] = [await mask(true), await mask(false)];
    seen[station] = { floor: bare, share: +(under / bare).toFixed(3) };
  }
  await ctx.writeEvidence("canopy-closure.json", seen);
  ctx.check(
    `the canopy mostly closes and the floor still shows: ${FLOOR_SEEN[0]} to ${FLOOR_SEEN[1]} of the wood's floor is seen through the crowns`,
    Object.values(seen).every(
      (s) =>
        // The wood fills most of the frame.
        s.floor > 0.5 * 1920 * 1080 && s.share >= FLOOR_SEEN[0] && s.share <= FLOOR_SEEN[1],
    ),
    JSON.stringify(seen),
  );
  await page.close();
}

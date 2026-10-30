// What one of the lab's `suppress*` switches costs a frame, measured at the
// pose a page stands at (a ground station's, `_groundStations.mjs`).
import { median } from "./_colour.mjs";
import { lab } from "./_lab.mjs";

/** Frames a cost batch draws before its GPU time is read. */
const COST_FRAMES = 120;

/** What one of the lab's `suppress*` switches costs a frame at the pose the
 *  page stands at: the same frozen frame with what the switch draws and
 *  without it, in `pairs` interleaved batches. `plainMs` is the median frame
 *  without it, `costMs` the median of the paired differences, `differences`
 *  each pair's. Run it alone, under the GPU lock. */
export async function pairedCost(page, suppress, pairs) {
  // The page draws on demand, and the timer's mean runs over the frames
  // drawn: a view change resets it, then every frame is forced.
  const batch = (off) =>
    lab(
      page,
      async ({ suppress, off, frames }) => {
        await window.__lab[suppress](off);
        await window.__lab.setFrameView("final");
        for (let i = 0; i < frames; i++) await window.__lab.frame();
        return window.__lab.stats().gpu.meanMs;
      },
      { suppress, off, frames: COST_FRAMES },
    );
  const rows = { on: [], off: [] };
  for (let pair = 0; pair < pairs; pair++) {
    rows.off.push(await batch(true));
    rows.on.push(await batch(false));
  }
  const differences = rows.on.map((v, i) => v - rows.off[i]);
  return { plainMs: median(rows.off), costMs: median(differences), differences };
}

// The village watched, blue on its comparison script.
// The whole-battle frames the composed look is judged on (`battle`), and fog
// running on past the map edge with the playable area's border (`edge`).
// `WATCH_TOURS=battle` runs only that tour; `BATTLE_TICK` moves the frames.
// `WATCH_TOURS=cover` (opt-in, never run by default) captures how cover shows;
// `WATCH_TOURS=rounds` (opt-in) frames each round kind in flight;
// `WATCH_TOURS=light` (opt-in) frames the light each weapon casts.
import { battleTour, edgeTour } from "./_battleLook.mjs";
import { coverTour } from "./_coverSheet.mjs";
import { roundsTour } from "./_roundsSheet.mjs";
import { contactTour } from "./_contactSheet.mjs";
import { cadenceTour } from "./_cadenceSheet.mjs";
import { lightTour } from "./_lightSheet.mjs";

const TOURS = { edge: edgeTour, battle: battleTour, contacts: contactTour };
const OPT_IN = { cadence: cadenceTour, cover: coverTour, rounds: roundsTour, light: lightTour };

export async function run(ctx) {
  const only = process.env.WATCH_TOURS?.split(",");
  for (const name of only ?? Object.keys(TOURS)) await (TOURS[name] ?? OPT_IN[name])(ctx);
}

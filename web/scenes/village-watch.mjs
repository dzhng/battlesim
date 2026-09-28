// Battle-look slice 27: the village watched, blue on its comparison script.
// The whole-battle frames the composed look is judged on (`battle`), and fog
// running on past the map edge with the playable area's border (`edge`).
// `WATCH_TOURS=battle` runs only that tour; `BATTLE_TICK` moves the frames.
// `WATCH_TOURS=cover` (opt-in, never run by default) captures how cover shows.
import { battleTour, edgeTour } from "./_battleLook.mjs";
import { coverTour } from "./_coverSheet.mjs";

const TOURS = { edge: edgeTour, battle: battleTour };
const OPT_IN = { cover: coverTour };

export async function run(ctx) {
  const only = process.env.WATCH_TOURS?.split(",");
  for (const name of only ?? Object.keys(TOURS)) await (TOURS[name] ?? OPT_IN[name])(ctx);
}

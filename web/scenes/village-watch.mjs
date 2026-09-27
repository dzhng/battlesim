// Battle-look slice 27: the village watched, blue on its comparison script.
// The whole-battle frames the composed look is judged on (`battle`), and fog
// running on past the map edge with the playable area's border (`edge`).
// `WATCH_TOURS=battle` runs only that tour; `BATTLE_TICK` moves the frames.
import { battleTour, edgeTour } from "./_battleLook.mjs";

const TOURS = { edge: edgeTour, battle: battleTour };

export async function run(ctx) {
  const only = process.env.WATCH_TOURS?.split(",");
  for (const name of only ?? Object.keys(TOURS)) await TOURS[name](ctx);
}

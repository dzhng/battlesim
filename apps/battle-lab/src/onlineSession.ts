// The ticket and address travel in history state, never credentials.
import type { NavigateFunction } from "react-router";
import type { OnlineMap } from "./online/useOnlineMap";
import type { BattleTicket } from "@packages/multiplayer/src/wire";

export interface OnlineBattleEntry {
  room?: string;
  ticket: BattleTicket;
  address: string;
}

/** The battlefield a briefing already built and checked, kept for the battle
 *  page it leads to. It stays in memory: a prepared battlefield is megabytes,
 *  and history state is copied on every entry and limited in size. A reloaded
 *  page finds nothing here and builds the map again from its ticket. */
let built: { battle: string; map: OnlineMap } | null = null;

/** All online modes follow a ticket into the same page. */
export function followBattle(
  navigate: NavigateFunction,
  entry: OnlineBattleEntry,
  search: string,
  map?: OnlineMap,
) {
  built = map ? { battle: entry.ticket.battle, map } : null;
  navigate(`/online/battle/${encodeURIComponent(entry.ticket.battle)}${search}`, {
    replace: true,
    state: entry,
  });
}

/** The map built for `battle` before the page followed its ticket, if any. */
export const builtMap = (battle: string): OnlineMap | null =>
  built?.battle === battle ? built.map : null;

/** For a page entered without `followBattle`: a test's, or a scene's. */
export function keepBuiltMap(battle: string, map: OnlineMap) {
  built = { battle, map };
}

/** A skirmish's lanes as `sim::entries` numbers them: each entry's lane on
 *  its side and the player holding it, and the entry a seated player's view
 *  opens on. The page reads lanes only through here and never numbers one
 *  itself. */
import type { Player } from "./protocol";

/** One entry, in the sites' list order. */
export interface EntryLane {
  side: Player["side"];
  /** From 1 at the map's high-x end: the top of the lobby picture. */
  lane: number;
  /** The id of the player whose slot holds this lane, or null. */
  holder: string | null;
}

export interface SkirmishLanes {
  entries: EntryLane[];
  /** Index into the sites' entries, or null when there is none to open on. */
  opening: number | null;
}

export interface LanesModule {
  skirmish_lanes(sites: string, players: string): string;
}

/** `sites` (a `SkirmishSites` as JSON text) with `players` seated on it. */
export function lanesOf(
  wasm: LanesModule,
  sites: string,
  players: readonly Player[],
): SkirmishLanes {
  return JSON.parse(wasm.skirmish_lanes(sites, JSON.stringify(players))) as SkirmishLanes;
}

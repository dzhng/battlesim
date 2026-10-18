/** Which of the side's units a player may order (D47): the one rule every
 *  selection and command path reads. A side's observation holds every unit
 *  on it; in a team battle some are an ally's, which the player sees but
 *  never commands. Everything else on the side is an ally's (`allyOwners`).
 *  `player` is the player the client commands as (`commandingPlayer.ts`); a
 *  battle without player-owned units (a lab) lets the player order them all. */
import type { ObservationView, OwnUnitView } from "../sim/observationView";
import { playerName } from "../present/playerName";

/** The skirmish player index `player` holds, or null where units are not
 *  player-owned. */
export function playerIndex(observation: ObservationView | null, player: string | undefined) {
  return observation?.skirmish?.players.find((p) => p.id === player)?.index ?? null;
}

/** Whether `player` (its index) may order `unit`. */
const ordered = (unit: OwnUnitView, index: number | null) =>
  index === null || unit.owner == null || unit.owner === index;

/** The units `player` may order: its own, never an ally's. */
export function controlled(
  observation: ObservationView | null,
  player: string | undefined,
): OwnUnitView[] {
  const own = observation?.own ?? [];
  const index = playerIndex(observation, player);
  return index === null ? [...own] : own.filter((unit) => ordered(unit, index));
}

/** Each of the side's units `player` does not order, with the line naming
 *  its owner on its panel: "BLUE 2 · AI". Empty where it orders them all. */
export function allyOwners(
  observation: ObservationView | null,
  player: string | undefined,
): ReadonlyMap<number, string> {
  const index = playerIndex(observation, player);
  const players = observation?.skirmish?.players ?? [];
  const lines = new Map<number, string>();
  if (index === null) return lines;
  for (const unit of observation?.own ?? []) {
    if (ordered(unit, index)) continue;
    lines.set(unit.id, playerName(players.find((p) => p.index === unit.owner)!));
  }
  return lines;
}

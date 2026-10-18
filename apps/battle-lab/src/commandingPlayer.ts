// Which player the local client's orders go out as. A player-owned scenario
// refuses an order without its player, and the simulation refuses an order
// for a slot under AI control, so a view of one side commands as that side's
// human and never as an AI's slot. A session that seats no human (an
// uploaded scenario played AI-only) says so with `null`.
import type { SideName } from "@web/battle/sim/protocol";

interface ScenarioPlayer {
  readonly id: string;
  readonly side: SideName;
  readonly controller: string;
}

export function commandingPlayer(
  players: readonly ScenarioPlayer[],
  side: SideName,
  /** The session's seated human, or null when it seats none. */
  explicit?: string | null,
): string | undefined {
  if (explicit !== undefined) return explicit ?? undefined;
  return players.find((player) => player.side === side && player.controller === "human")?.id;
}

/** Whether the client takes orders at all: a scenario without players lets it
 *  order every unit on its side; a player-owned one only through a seat. */
export function takesOrders(
  players: readonly ScenarioPlayer[],
  commanding: string | undefined,
): boolean {
  return players.length === 0 || commanding !== undefined;
}

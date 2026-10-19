// Which player the local client plays as: its orders go out as that player,
// and the HUD shows that player's economy. A player-owned scenario refuses an
// order without its player, and the simulation refuses an order for a player
// under AI control, so a view of one side commands as that side's human and
// never as an AI player. A session that seats no human (an uploaded scenario
// played AI-only) says so with `null`.
import type { Player } from "@web/battle/prepare/protocol";
import type { SideName } from "@web/battle/sim/protocol";
import type { PlayerView, SkirmishView } from "@web/battle/sim/observationView";

type ScenarioPlayer = Pick<Player, "id" | "side" | "controller">;

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

/** The commanding player's credits, cap, readiness and pending purchases in
 *  the side's skirmish view; none when the client commands nobody. */
export function commandedEconomy(
  match: SkirmishView | null | undefined,
  commanding: string | undefined,
): PlayerView | undefined {
  return commanding === undefined
    ? undefined
    : match?.players.find((player) => player.id === commanding);
}

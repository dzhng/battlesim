// Which player the local client's orders go out as. A player-owned scenario
// refuses an order without its player, so a view of one side commands as that
// side's human, or its first player when the side has none (a lab or a
// benchmark watching AI play).
import type { SideName } from "@web/battle/sim/protocol";

interface ScenarioPlayer {
  readonly id: string;
  readonly side: SideName;
  readonly controller: string;
}

export function commandingPlayer(
  players: readonly ScenarioPlayer[],
  side: SideName,
  explicit?: string,
): string | undefined {
  if (explicit !== undefined) return explicit;
  const own = players.filter((player) => player.side === side);
  return (own.find((player) => player.controller === "human") ?? own[0])?.id;
}

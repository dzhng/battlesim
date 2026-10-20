/** A player as the battle's interface names them: their seat ("blue-2" is
 *  BLUE 2) and who plays it. The one naming every surface shares: an ally's
 *  panel, the match report's rows and timeline. */
import type { Player } from "../prepare/protocol";

/** A battle's player, as its scenario lists them (index order). */
export type Seat = Pick<Player, "id" | "side" | "controller">;

/** "BLUE 1 (YOU)" for the player the client plays, "RED 3 · AI" for an AI,
 *  "BLUE 2 · HUMAN" for another person. */
export function playerName(player: { id: string; controller: string }, you = false): string {
  const seat = player.id.replace(/-/g, " ").toUpperCase();
  if (you) return `${seat} (YOU)`;
  return `${seat} · ${player.controller === "human" ? "HUMAN" : "AI"}`;
}

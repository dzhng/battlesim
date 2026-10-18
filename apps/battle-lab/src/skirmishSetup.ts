import skirmishSizes from "@fixtures/skirmish-sizes.json";
import type { Player } from "@web/battle/prepare/protocol";

/** The shared fixture's entry-arm count for the larger combatant side. */
export function entryCountForPlayers(players: readonly Player[]): number {
  const largest = Math.max(
    players.filter((player) => player.side === "blue").length,
    players.filter((player) => player.side === "red").length,
  );
  if (largest === 0) return 0;
  const tier = skirmishSizes.tiers.find(
    (candidate) =>
      largest >= candidate.min_players &&
      (candidate.max_players === undefined || largest <= candidate.max_players),
  );
  if (!tier) throw new Error(`no skirmish size tier for ${largest} players`);
  return tier.entry_count;
}

/** Canonical player construction for routes that prepare a skirmish-shaped
 * workload without a lobby. */
export function playersFor(
  blueCount: number,
  redCount: number,
  humanBlueIndex: number | null = 0,
): Player[] {
  return (["blue", "red"] as const).flatMap((side) =>
    Array.from({ length: side === "blue" ? blueCount : redCount }, (_, index) => {
      const human = side === "blue" && humanBlueIndex === index;
      return {
        id: `${side}-${index + 1}`,
        side,
        faction: side === "blue" ? ("us" as const) : ("eastern" as const),
        controller: human ? ("human" as const) : ("active_ai" as const),
        human_capable: human,
      };
    }),
  );
}

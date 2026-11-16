import type { Player } from "../../../web/src/battle/prepare/protocol";
import type { MapSize } from "../../../web/src/maps/source";
import type { Faction } from "../../scene-assets/src/units";
import skirmishSizes from "../../../fixtures/skirmish-sizes.json";

export type SkirmishSide = "blue" | "red";
export const MAX_PLAYERS_PER_SIDE = 5;
export const DEFAULT_FACTION: Readonly<Record<SkirmishSide, Faction>> = {
  blue: "us",
  red: "eastern",
};

export type SeatId = string;
export type Occupant = { kind: "human"; who: string } | { kind: "ai" } | { kind: "open" };
export interface LobbySeat {
  id: SeatId;
  side: SkirmishSide;
  occupant: Occupant;
}

export type SkirmishSeat = {
  side: SkirmishSide;
  faction: Faction;
  controller: Player["controller"];
};

export type SizeTier = Omit<(typeof skirmishSizes.tiers)[number], "size"> & { size: MapSize };
const sizeTiers = skirmishSizes.tiers as readonly SizeTier[];

function controllerOf(occupant: Occupant): Player["controller"] | null {
  if (occupant.kind === "human") return "human";
  if (occupant.kind === "ai") return "active_ai";
  return null;
}

export function battlePlayers(seats: readonly LobbySeat[]): {
  players: Player[];
  playerOf: Record<SeatId, string>;
} {
  const players: Player[] = [];
  const playerOf: Record<SeatId, string> = {};
  for (const side of ["blue", "red"] as const) {
    const sideSeats = seats.filter((seat) => seat.side === side && seat.occupant.kind !== "open");
    if (!sideSeats.length) throw new Error(`${side} must have a human or AI seat`);
    sideSeats.forEach((seat, index) => {
      const id = playerId(side, index);
      playerOf[seat.id] = id;
      players.push({
        id,
        side,
        controller: controllerOf(seat.occupant)!,
        human_capable: seat.occupant.kind === "human",
      });
    });
  }
  return { players, playerOf };
}

export function skirmishPlayers(seats: readonly SkirmishSeat[]): Player[] {
  return (["blue", "red"] as const).flatMap((side) =>
    seats
      .filter((seat) => seat.side === side)
      .map((seat, index) => ({
        id: playerId(side, index),
        side,
        faction: seat.faction,
        controller: seat.controller,
        human_capable: seat.controller === "human",
      })),
  );
}

function playerId(side: SkirmishSide, index: number): string {
  return `${side}-${index + 1}`;
}

export function playersFor(
  blueCount: number,
  redCount: number,
  humanBlueIndex: number | null = 0,
): Player[] {
  return skirmishPlayers(
    (["blue", "red"] as const).flatMap((side) =>
      Array.from({ length: side === "blue" ? blueCount : redCount }, (_, index) => ({
        side,
        faction: DEFAULT_FACTION[side],
        controller:
          side === "blue" && index === humanBlueIndex ? ("human" as const) : ("active_ai" as const),
      })),
    ),
  );
}

export function sizeTier(players: readonly Pick<Player, "side">[]) {
  const largest = Math.max(
    1,
    players.filter((player) => player.side === "blue").length,
    players.filter((player) => player.side === "red").length,
  );
  const tier = sizeTiers.find(
    (candidate) =>
      largest >= candidate.min_players &&
      (candidate.max_players === undefined || largest <= candidate.max_players),
  );
  if (!tier) throw new Error(`no skirmish size tier for ${largest} players`);
  return tier;
}

export function entryCountForPlayers(players: readonly Pick<Player, "side">[]): number {
  return sizeTier(players).entry_count;
}

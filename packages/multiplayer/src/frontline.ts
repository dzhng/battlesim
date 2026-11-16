import { drawIndex } from "./random";
import { FACTIONS, type Faction } from "../../scene-assets/src/units";
import { MAP_REGIONS, MAP_TYPES, type MapType } from "../../../web/src/maps/source";
import { preset, reroll, setFaction, setRegion, setType } from "../../skirmish-setup/src/lobby";
import { allAi, toBattleSetup } from "./setup";
import type { BattleSetup } from "./wire";

export interface GameView {
  battle: string;
  scores: [number, number];
  target: number;
  seats: {
    side: "blue" | "red";
    number: number;
    held: { who: string; lastOrderAt: number; goneSince: number | null } | null;
    reservedUntil: number | null;
  }[];
  granted: string[];
}

export const IDLE_TIMEOUT_MS = 3 * 60 * 1000;
export const GONE_TIMEOUT_MS = 60 * 1000;
const SIDES = ["blue", "red"] as const;

function draw<T>(values: readonly T[], random: () => number): T {
  return values[drawIndex(random, values.length)]!;
}

function humans(game: GameView, side: (typeof SIDES)[number]): number {
  return game.seats.filter((seat) => seat.side === side && seat.held !== null).length;
}

function losingSide(game: GameView, random: () => number): (typeof SIDES)[number] {
  if (game.scores[0] < game.scores[1]) return "blue";
  if (game.scores[1] < game.scores[0]) return "red";
  const blueHumans = humans(game, "blue");
  const redHumans = humans(game, "red");
  if (blueHumans < redHumans) return "blue";
  if (redHumans < blueHumans) return "red";
  return random() < 0.5 ? "blue" : "red";
}

function available(game: GameView, side: (typeof SIDES)[number], now: number) {
  return game.seats
    .filter(
      (seat) =>
        seat.side === side &&
        seat.number >= 1 &&
        seat.number <= 4 &&
        seat.held === null &&
        (seat.reservedUntil === null || seat.reservedUntil <= now),
    )
    .sort((a, b) => a.number - b.number)[0];
}

export type Placement =
  | { join: string; side: "blue" | "red"; seat: number; opening: boolean }
  | { resume: string; side: "blue" | "red"; seat: number }
  | { create: true }
  | { full: true };

/** Chooses a running battle, or reports that the service must create one. */
export function place(
  games: GameView[],
  who: string,
  now: number,
  random: () => number,
  battleSlotFree: boolean,
): Placement {
  for (const game of games) {
    const held = game.seats.find((seat) => seat.held?.who === who);
    if (held) return { resume: game.battle, side: held.side, seat: held.number };
  }
  const candidates = games
    .map((game, order) => {
      if (!(Math.max(...game.scores) < game.target * 0.9)) return undefined;
      const side = losingSide(game, random);
      const seat = available(game, side, now);
      return seat ? { game, side, seat, order } : undefined;
    })
    .filter((candidate): candidate is NonNullable<typeof candidate> => candidate !== undefined)
    .sort(
      (a, b) =>
        humans(b.game, "blue") +
          humans(b.game, "red") -
          humans(a.game, "blue") -
          humans(a.game, "red") || a.order - b.order,
    );
  const candidate = candidates[0];
  if (candidate) {
    return {
      join: candidate.game.battle,
      side: candidate.side,
      seat: candidate.seat.number,
      opening: !candidate.game.granted.includes(who),
    };
  }
  return battleSlotFree ? { create: true } : { full: true };
}

export function seatsToFree(
  game: GameView,
  now: number,
): { side: "blue" | "red"; seat: number; why: "idle" | "gone" }[] {
  const freed: { side: "blue" | "red"; seat: number; why: "idle" | "gone" }[] = [];
  for (const seat of game.seats) {
    if (!seat.held) continue;
    if (seat.held.goneSince !== null && now - seat.held.goneSince >= GONE_TIMEOUT_MS) {
      freed.push({ side: seat.side, seat: seat.number, why: "gone" });
    } else if (now - seat.held.lastOrderAt >= IDLE_TIMEOUT_MS) {
      freed.push({ side: seat.side, seat: seat.number, why: "idle" });
    }
  }
  return freed;
}

function factionPair(random: () => number): [Faction, Faction] {
  const blue = draw(FACTIONS, random);
  const alternatives = FACTIONS.filter((faction) => faction !== blue);
  return [blue, draw(alternatives, random)];
}

function seed(random: () => number): string {
  return Math.floor(Math.min(1 - Number.EPSILON, Math.max(0, random())) * 2 ** 32).toString();
}

/** A fresh Frontline setup: five AI seats per side until Play hands one over. */
export function newGame(random: () => number): BattleSetup {
  const [blue, red] = factionPair(random);
  const type = draw(MAP_TYPES as readonly MapType[], random);
  const region = draw(MAP_REGIONS, random);
  let lobby = allAi(preset("5v5"));
  lobby = setFaction(setFaction(lobby, "blue", blue), "red", red);
  lobby = setType(lobby, type);
  lobby = setRegion(lobby, region);
  lobby = reroll(lobby, seed(random));
  lobby = {
    ...lobby,
    battleSeed: Math.floor(
      Math.min(1 - Number.EPSILON, Math.max(0, random())) * Number.MAX_SAFE_INTEGER,
    ),
  };
  return toBattleSetup(lobby, "frontline");
}

// Candidate maps and their initial placement are drawn together, in that order.
import { randomUUID } from "node:crypto";
import { newGame, place, type GameView } from "../../../packages/multiplayer/src/frontline.ts";

export function drawFrontlineCandidates(
  who: string,
  now: number,
  mapAttempts: number,
  random: () => number,
) {
  const battle = randomUUID();
  const draws = Array.from({ length: mapAttempts }, () => {
    const setup = newGame(random);
    const view: GameView = {
      battle,
      scores: [0, 0],
      target: Infinity,
      granted: [],
      seats: setup.seats.map((s) => ({
        side: s.side,
        number: setup.seats.filter((p) => p.side === s.side).indexOf(s) + 1,
        held: null,
        reservedUntil: null,
      })),
    };
    const initial = place([view], who, now, random, false);
    if (!("join" in initial)) throw new Error("a fresh battle has no seat");
    const index = view.seats.findIndex((s) => s.side === initial.side && s.number === initial.seat);
    setup.seats[index].controller = "human";
    return { setup, view, index };
  });
  return { battle, draws };
}

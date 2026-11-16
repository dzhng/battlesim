// @vitest-environment node
import { expect, test } from "vitest";
import {
  GONE_TIMEOUT_MS,
  IDLE_TIMEOUT_MS,
  newGame,
  place,
  seatsToFree,
  type GameView,
} from "../../../packages/multiplayer/src/frontline";

const aiSeat = (side: "blue" | "red", number: number) => ({
  side,
  number,
  held: null,
  reservedUntil: null,
});
const game = (overrides: Partial<GameView> = {}): GameView => ({
  battle: "battle-a",
  scores: [10, 20],
  target: 100,
  seats: [
    aiSeat("blue", 1),
    aiSeat("blue", 2),
    {
      side: "blue",
      number: 3,
      held: { who: "blue-human", lastOrderAt: 1, goneSince: null },
      reservedUntil: null,
    },
    aiSeat("blue", 4),
    aiSeat("blue", 5),
    aiSeat("red", 1),
    aiSeat("red", 2),
    aiSeat("red", 3),
    aiSeat("red", 4),
    aiSeat("red", 5),
  ],
  granted: [],
  ...overrides,
});

test("placement chooses a free seat on the losing side", () => {
  expect(place([game()], "new-player", 10, () => 0, true)).toEqual({
    join: "battle-a",
    side: "blue",
    seat: 1,
    opening: true,
  });
});

test("placement chooses the fullest eligible game", () => {
  const fuller = game({
    battle: "battle-b",
    seats: game().seats.map((seat) =>
      seat.side === "blue" && seat.number < 3
        ? { ...seat, held: { who: `human-${seat.number}`, lastOrderAt: 1, goneSince: null } }
        : seat,
    ),
  });
  expect(place([game(), fuller], "new-player", 10, () => 0, true)).toMatchObject({
    join: "battle-b",
  });
});

test("placement uses the lowest free seat and never offers the reserved fifth seat", () => {
  const seats = game().seats.map((seat) =>
    seat.side === "blue" && seat.number === 1
      ? { ...seat, held: { who: "already", lastOrderAt: 1, goneSince: null } }
      : seat,
  );
  expect(place([game({ seats })], "new-player", 10, () => 0, true)).toMatchObject({
    side: "blue",
    seat: 2,
  });
});

test("placement never uses seat five or a battle past its joining point", () => {
  const almostDone = game({ scores: [91, 10] });
  expect(place([almostDone], "new-player", 10, () => 0, true)).toEqual({ create: true });
  const noEarlySeat = game({
    seats: game().seats.map((seat) =>
      seat.side === "blue" && seat.number < 5
        ? { ...seat, held: { who: "human", lastOrderAt: 1, goneSince: null } }
        : seat,
    ),
  });
  expect(place([noEarlySeat], "new-player", 10, () => 0, true)).toEqual({ create: true });
});

test("placement creates a game when none fits and reports a full server at capacity", () => {
  expect(place([], "new-player", 10, () => 0, true)).toEqual({ create: true });
  const full = Array.from({ length: 8 }, (_, index) =>
    game({
      battle: `battle-${index}`,
      seats: game().seats.map((seat) =>
        seat.side === "blue" && seat.number < 5
          ? {
              ...seat,
              held: { who: `human-${index}-${seat.number}`, lastOrderAt: 1, goneSince: null },
            }
          : seat,
      ),
    }),
  );
  expect(place(full, "new-player", 10, () => 0, false)).toEqual({ full: true });
});

test("a returning player does not receive opening credits twice", () => {
  const returning = game({ granted: ["new-player"] });
  expect(place([returning], "new-player", 10, () => 0, true)).toMatchObject({ opening: false });
});

test("a player holding a seat resumes it instead of receiving a fresh join", () => {
  const result = place([game()], "blue-human", 10, () => 0, true);
  expect(result).toEqual({ resume: "battle-a", side: "blue", seat: 3 });
});

test("equal scores choose the side with fewer humans before the coin toss", () => {
  const equal = game({ scores: [20, 20] });
  expect(place([equal], "new-player", 10, () => 0, true)).toMatchObject({ side: "red" });
});

test("equal scores and equal humans use the coin toss to choose a losing side", () => {
  const equal = game({
    scores: [20, 20],
    seats: game().seats.map((seat) => ({ ...seat, held: null })),
  });
  expect(place([equal], "new-player", 10, () => 0, true)).toMatchObject({ side: "blue" });
  expect(place([equal], "new-player", 10, () => 0.99, true)).toMatchObject({ side: "red" });
});

test("a held seat comes free after idling or losing its stream", () => {
  const now = 1_000_000;
  const idle = game({
    seats: [
      {
        side: "blue",
        number: 2,
        held: { who: "idle", lastOrderAt: now - IDLE_TIMEOUT_MS, goneSince: null },
        reservedUntil: null,
      },
    ],
  });
  const gone = game({
    seats: [
      {
        side: "red",
        number: 2,
        held: { who: "gone", lastOrderAt: now, goneSince: now - GONE_TIMEOUT_MS },
        reservedUntil: null,
      },
    ],
  });
  expect(seatsToFree(idle, now)).toEqual([{ side: "blue", seat: 2, why: "idle" }]);
  expect(seatsToFree(gone, now)).toEqual([{ side: "red", seat: 2, why: "gone" }]);
});

test("a new game has both sides, five seats, different factions and a full-size map", () => {
  const setup = newGame(() => 0);
  expect(setup.seats).toHaveLength(10);
  expect(setup.seats.filter((seat) => seat.side === "blue")).toHaveLength(5);
  expect(setup.seats.every((seat) => seat.controller === "active_ai")).toBe(true);
  expect(setup.seats.every((seat) => seat.player.match(/^(blue|red)-[1-5]$/))).toBe(true);
  expect(setup.factions.blue).not.toBe(setup.factions.red);
  expect(["open", "mixed", "metro"]).toContain(setup.map.type);
  expect(["china", "new_york", "paris"]).toContain(setup.map.region);
  expect(setup.map.size).toBe("xl");
});

test("new games draw their map kind and region from the supplied random source", () => {
  const first = newGame(() => 0);
  const second = newGame(() => 0.99);
  expect({ type: first.map.type, region: first.map.region }).not.toEqual({
    type: second.map.type,
    region: second.map.region,
  });
});

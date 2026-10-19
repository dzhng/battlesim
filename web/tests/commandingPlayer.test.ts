import { expect, test } from "vitest";
import {
  commandedEconomy,
  commandingPlayer,
  takesOrders,
} from "@apps/battle-lab/src/commandingPlayer";
import type { PlayerView, SkirmishView } from "@web/battle/sim/observationView";

const players = [
  { id: "blue-1", side: "blue", controller: "active_ai" },
  { id: "blue-2", side: "blue", controller: "human" },
  { id: "red-1", side: "red", controller: "active_ai" },
] as const;

test("orders go out as the viewing side's human player", () => {
  expect(commandingPlayer(players, "blue")).toBe("blue-2");
});

test("a side with no human player is commanded by no one, and takes no input", () => {
  const commanding = commandingPlayer(players, "red");
  expect(commanding).toBeUndefined();
  expect(takesOrders(players, commanding)).toBe(false);
});

test("a session that seats no human commands nobody, even beside a declared human", () => {
  const commanding = commandingPlayer(players, "blue", null);
  expect(commanding).toBeUndefined();
  expect(takesOrders(players, commanding)).toBe(false);
});

test("an explicit player wins, and a scenario without players takes orders unseated", () => {
  expect(commandingPlayer(players, "blue", "blue-1")).toBe("blue-1");
  expect(commandingPlayer([], "blue")).toBeUndefined();
  expect(takesOrders([], undefined)).toBe(true);
  expect(takesOrders(players, "blue-2")).toBe(true);
});

/** One own-side player's economy in the side's skirmish view. */
const economy = (id: string, index: number, credits: number, ready: boolean): PlayerView => ({
  index,
  id,
  controller: id === "blue-2" ? "human" : "active_ai",
  ready,
  credits,
  unitCount: index,
  cap: 30,
  pending: [
    {
      id: index,
      kind: `${id}-unit`,
      destination: [0, 0],
      confirmedTick: 0,
      arrivalTick: 0,
      blocked: false,
    },
  ],
});
const match = {
  phase: "preparation",
  players: [economy("blue-1", 0, 900, true), economy("blue-2", 1, 40, false)],
} as SkirmishView;

test("a human seated as Blue 2 sees Blue 2's credits, readiness and pending purchases", () => {
  const own = commandedEconomy(match, commandingPlayer(players, "blue"));
  expect(own).toEqual(match.players[1]);
});

test("a client commanding nobody sees no player's economy, not the side's first", () => {
  expect(commandedEconomy(match, commandingPlayer(players, "red"))).toBeUndefined();
  expect(commandedEconomy(match, commandingPlayer(players, "blue", null))).toBeUndefined();
});

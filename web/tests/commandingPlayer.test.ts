import { expect, test } from "vitest";
import { commandingPlayer } from "@apps/battle-lab/src/commandingPlayer";

const players = [
  { id: "blue-1", side: "blue", controller: "active_ai" },
  { id: "blue-2", side: "blue", controller: "human" },
  { id: "red-1", side: "red", controller: "active_ai" },
] as const;

test("orders go out as the viewing side's human player", () => {
  expect(commandingPlayer(players, "blue")).toBe("blue-2");
});

test("a side with no human is commanded as its first player", () => {
  expect(commandingPlayer(players, "red")).toBe("red-1");
});

test("an explicit player wins, and a scenario without players has none", () => {
  expect(commandingPlayer(players, "blue", "blue-1")).toBe("blue-1");
  expect(commandingPlayer([], "blue")).toBeUndefined();
});

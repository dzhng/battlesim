import { expect, test } from "vitest";
import { commandingPlayer, takesOrders } from "@apps/battle-lab/src/commandingPlayer";

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

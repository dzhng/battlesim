import { SessionCatalogProvider } from "@web/battle/catalog/context";
// The end-of-match report: every player's totals under their side, a
// timeline naming each player, and a result judged from the viewer's side.
import { cleanup, render, within } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { MatchResultOverlay } from "@apps/battle-lab/src/MatchResultOverlay";
import { BattleClock, SkirmishStatus } from "@apps/battle-lab/src/battleStatus";
import type { Seat } from "@web/battle/present/playerName";
import { TEST_CATALOG, WithTestCatalog } from "./catalog";
import type { MatchEventView, SkirmishView } from "@web/battle/sim/observationView";

afterEach(cleanup);

/** A link's seats: `blue` and `red` controller codes, in scenario order. */
function seats(blue: string, red: string): Seat[] {
  const controller = (c: string) => (c === "h" ? ("human" as const) : ("active_ai" as const));
  return [
    ...[...blue].map((c, k) => ({
      id: `blue-${k + 1}`,
      side: "blue" as const,
      controller: controller(c),
    })),
    ...[...red].map((c, k) => ({
      id: `red-${k + 1}`,
      side: "red" as const,
      controller: controller(c),
    })),
  ];
}

/** Each player's totals, distinct per player so a row can't borrow another's. */
const totalsOf = (players: Seat[]) =>
  players.map((_, k) => ({
    spawned: k + 1,
    spent: 100 * (k + 3),
    killed: 2 * k + 1,
    refunded: 10 * k + 0.4,
  }));

function finished(
  players: Seat[],
  result: SkirmishView["result"],
  events: MatchEventView[] = [],
): SkirmishView {
  return {
    phase: "finished",
    players: [],
    preparationRemainingS: 0,
    scores: [700, 300],
    result,
    objectives: [],
    report: { totals: totalsOf(players), events },
  };
}

function report(players: Seat[], viewer: string | null, match = finished(players, "blue")) {
  return render(<MatchResultOverlay match={match} seats={players} viewer={viewer} />, {
    wrapper: WithTestCatalog,
  });
}

test.each([
  ["1v1", "h", "a"],
  ["3v2", "haa", "aa"],
  ["5v5", "haaaa", "aaaaa"],
])("a %s report shows every player's own totals under their side", (_, blue, red) => {
  const players = seats(blue, red);
  const view = report(players, "blue-1");
  const totals = totalsOf(players);
  players.forEach((player, k) => {
    const name =
      player.id === "blue-1" ? "BLUE 1 (YOU)" : `${player.id.replace("-", " ").toUpperCase()} · AI`;
    const side = view.getByRole("group", { name: player.side === "blue" ? "Blue" : "Red" });
    const row = within(side).getByRole("row", { name });
    const cells = within(row)
      .getAllByRole("cell")
      .map((cell) => cell.textContent);
    expect(cells).toEqual([
      String(totals[k].spawned),
      String(totals[k].spent),
      String(totals[k].killed),
      String(Math.round(totals[k].refunded)),
    ]);
  });
  // No row for anyone who isn't playing.
  expect(view.getAllByRole("row", { name: /(YOU|· AI)/ })).toHaveLength(players.length);
});

test("the result is judged from the viewer's side", () => {
  const players = seats("ha", "aa");
  expect(report(players, "blue-1", finished(players, "red")).getByRole("heading").textContent).toBe(
    "DEFEAT",
  );
  cleanup();
  expect(
    report(players, "blue-1", finished(players, "blue")).getByRole("heading").textContent,
  ).toBe("VICTORY");
  cleanup();
  // A viewer on Red wins when Red does: nothing reads the literal "blue".
  const onRed = seats("a", "h");
  expect(report(onRed, "red-1", finished(onRed, "red")).getByRole("heading").textContent).toBe(
    "VICTORY",
  );
  cleanup();
  expect(
    report(players, "blue-1", finished(players, "draw")).getByRole("heading").textContent,
  ).toBe("DRAW");
});

test("the battle's status strip judges the result from the viewer's side too", () => {
  const match = finished(seats("h", "a"), "red");
  const view = render(<SkirmishStatus match={match} viewer="red" />);
  expect(view.getByRole("status").textContent).toBe("VICTORY");
  view.rerender(<SkirmishStatus match={match} viewer={null} />);
  expect(view.getByRole("status").textContent).toBe("RED WINS");
});

test("spectating a battle with no human reads a neutral result", () => {
  const players = seats("aa", "aa");
  const view = report(players, null, finished(players, "red"));
  expect(view.getByRole("heading").textContent).toBe("RED WINS");
  expect(view.queryByText(/VICTORY|DEFEAT|YOU/)).toBeNull();
});

const spawn = (tick: number, owner: number, unitId: number, side: "blue" | "red" = "blue") =>
  ({ kind: "spawn", tick, side, unitId, unitKind: "test_tank", owner, amount: 300 }) as const;

test("the timeline names the player behind every action", () => {
  const players = seats("ha", "aa");
  const events: MatchEventView[] = [
    spawn(30, 1, 0),
    {
      kind: "kill",
      tick: 900,
      side: "red",
      unitId: 7,
      unitKind: "test_rifle",
      owner: 3,
      victimCost: 110,
      killer: { side: "blue", unitId: 0, unitKind: "test_tank", owner: 1, cost: 300 },
    },
    {
      kind: "refund",
      tick: 1800,
      side: "blue",
      unitId: 4,
      unitKind: "test_jeep",
      owner: 0,
      amount: 61.6,
    },
  ];
  const view = report(players, "blue-1", finished(players, "blue", events));
  const rows = within(view.getByLabelText("Unit action timeline")).getAllByRole("listitem");
  expect(rows).toHaveLength(3);
  expect(rows[0].textContent).toContain("BLUE 2 · AI");
  expect(rows[0].textContent).toContain("SPAWNED");
  // The killer's player, then the victim's.
  expect(rows[1].textContent).toMatch(/BLUE 2 · AI.*RED 2 · AI/s);
  expect(rows[1].textContent).toContain("110 CREDITS");
  expect(rows[2].textContent).toContain("BLUE 1 (YOU)");
  expect(rows[2].textContent).toContain("REFUNDED");
});

test("a player's purchases in one minute read as one line of the timeline", () => {
  const players = seats("haaaa", "aaaaa");
  // Ten AIs buying at once, their arrivals interleaved at the entries.
  const events: MatchEventView[] = [];
  for (let round = 0; round < 4; round++)
    for (let owner = 0; owner < 10; owner++)
      events.push(
        spawn(60 + round * 90 + owner, owner, round * 10 + owner, owner < 5 ? "blue" : "red"),
      );
  // The next minute's purchase is its own line.
  events.push(spawn(30 * 61, 2, 99));
  const view = report(players, "blue-1", finished(players, "blue", events));
  const rows = within(view.getByLabelText("Unit action timeline")).getAllByRole("listitem");
  expect(rows).toHaveLength(11);
  const red3 = rows.filter((row) => row.textContent?.includes("RED 3 · AI"));
  expect(red3).toHaveLength(1);
  expect(red3[0].textContent).toContain("4 UNITS");
  expect(red3[0].textContent).toContain("1200 CREDITS");
  expect(rows[10].textContent).toContain("BLUE 3 · AI");
  expect(rows[10].textContent).toContain("1:01");
});

test("consecutive battle clocks and report minutes use their own tick rates", () => {
  const players = seats("h", "a");
  const match = finished(players, "blue", [spawn(1100, 0, 1), spawn(1250, 0, 2)]);
  const battle = (tickHz: number) => (
    <SessionCatalogProvider
      catalog={{ ...TEST_CATALOG, rules: { ...TEST_CATALOG.rules, tick_hz: tickHz } }}
    >
      <BattleClock tick={1200} />
      <MatchResultOverlay match={match} seats={players} viewer="blue-1" />
    </SessionCatalogProvider>
  );
  const view = render(battle(30));
  expect(view.getByTestId("clock").textContent).toBe("0:40");
  expect(view.container.querySelectorAll('[data-event="spawn"]')).toHaveLength(1);
  view.rerender(battle(20));
  expect(view.getByTestId("clock").textContent).toBe("1:00");
  expect(view.container.querySelectorAll('[data-event="spawn"]')).toHaveLength(2);
});

// @vitest-environment node
import { expect, test } from "vitest";
import { askedBattle, battleHref, spoken } from "../../apps/battle-lab/src/battleLinks";
import { sizeTier } from "../../apps/battle-lab/src/skirmishSetup";
import { MAP_SIZES } from "@web/maps/source";

test("size labels use the lobby's S/M/L/XL vocabulary", () => {
  expect(["small", "medium", "large", "xl"].map(spoken)).toEqual(["S", "M", "L", "XL"]);
});

test("links round trip ordered player controllers and side factions", () => {
  const href = battleHref({ type: "mixed", size: "large", seed: "7" }, [
    { id: "blue-1", side: "blue", faction: "us", controller: "human", human_capable: true },
    { id: "blue-2", side: "blue", faction: "us", controller: "active_ai", human_capable: false },
    {
      id: "red-1",
      side: "red",
      faction: "eastern",
      controller: "passive_ai",
      human_capable: false,
    },
  ]);
  expect(askedBattle(new URL(href, "https://battle.invalid").search)).toMatchObject({
    kind: "generated",
    players: [
      expect.objectContaining({ id: "blue-1", controller: "human", faction: "us" }),
      expect.objectContaining({ id: "blue-2", controller: "active_ai", faction: "us" }),
      expect.objectContaining({ id: "red-1", controller: "passive_ai", faction: "eastern" }),
    ],
  });
});

test.each([
  [1, 1, 1],
  [3, 4, 0],
  [5, 5, 0],
] as const)("supports %sv%s player links (%s humans)", (blueCount, redCount, humans) => {
  const players = (["blue", "red"] as const).flatMap((side) =>
    Array.from({ length: side === "blue" ? blueCount : redCount }, (_, index) => ({
      id: `${side}-${index + 1}`,
      side,
      faction: side === "blue" ? ("us" as const) : ("eastern" as const),
      controller:
        humans && side === "blue" && index === 0 ? ("human" as const) : ("active_ai" as const),
      human_capable: humans > 0 && side === "blue" && index === 0,
    })),
  );
  const href = battleHref({ type: "mixed", size: "xl", seed: "9" }, players);
  const asked = askedBattle(new URL(href, "https://battle.invalid").search);
  expect(asked).toMatchObject({ kind: "generated", players });
});

test.each([
  ["?type=mixed&size=small&seed=1&faction=us&enemy=eastern", /faction/],
  ["?type=mixed&size=small&seed=1", /players/],
])("refuses old-shape link %s", (search, message) => {
  expect(askedBattle(search)).toMatchObject({ error: expect.stringMatching(message) });
});

test.each([
  ["h", "h"],
  ["h,h,a", "a,a,a"],
  ["a,a,a", "h,a,a"],
  ["h,a", "a,h,a"],
])("a link with humans has exactly one, on Blue: refuses blue=%s red=%s", (blue, red) => {
  const search = `?type=mixed&size=large&seed=1&battle=1&blue=${blue}&red=${red}&blue_faction=us&red_faction=eastern`;
  expect(askedBattle(search)).toMatchObject({
    error: expect.stringMatching(/exactly one Human on Blue/),
  });
});

test("a link with no human is an AI-only battle at any size", () => {
  for (const [blue, red] of [
    ["a", "a"],
    ["a,p,a", "a,a,a"],
  ])
    expect(
      askedBattle(
        `?type=mixed&size=large&seed=1&battle=1&blue=${blue}&red=${red}&blue_faction=us&red_faction=eastern`,
      ),
    ).toMatchObject({ kind: "generated" });
});

test.each([
  ["a", "a"],
  ["a,a", "a"],
  ["a,a,a", "a,a,a"],
  ["a,a,a,a,a", "a,a"],
])("a link without a size gets its players' size tier: blue=%s red=%s", (blue, red) => {
  const asked = askedBattle(
    `?type=mixed&seed=1&battle=1&blue=${blue}&red=${red}&blue_faction=us&red_faction=eastern`,
  );
  if (!("kind" in asked) || asked.kind !== "generated") throw new Error(JSON.stringify(asked));
  expect(asked.map.size).toBe(sizeTier(asked.players).size);
});

test("a one-a-side link without a size is on the smallest map", () => {
  expect(askedBattle("?seed=1&blue=a&red=a&blue_faction=us&red_faction=eastern")).toMatchObject({
    kind: "generated",
    map: { size: MAP_SIZES[0] },
  });
});

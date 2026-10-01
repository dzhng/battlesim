import { expect, test } from "vitest";
import { layoutReadoutDetails, type DetailCard } from "../src/battle/present/readoutDetails";

test("the closest cards get detail before farther cards consume the same room", () => {
  const cards: DetailCard[] = [
    { id: "near", distanceSq: 1, compact: [100, 100, 140, 110], full: [100, 100, 260, 200] },
    { id: "far", distanceSq: 100, compact: [200, 20, 240, 30], full: [200, 20, 260, 220] },
  ];
  const place = (c: DetailCard[]) => layoutReadoutDetails(c, [], [0, 0, 800, 600], 14, true, null);
  let result = place(cards);
  expect(result.get("near")!.full).toBe(true);
  expect(result.get("far")!.full).toBe(false);
  expect(result.get("near")!.box.slice(0, 2)).toEqual([100, 100]);
  result = place(cards.map((c) => ({ ...c, distanceSq: 101 - c.distanceSq })));
  expect(result.get("far")!.full).toBe(true);
  expect(result.get("near")!.full).toBe(false);
});

test("one shorter neighbor shift beats moving the expanded card farther", () => {
  const cards: DetailCard[] = [
    { id: "near", distanceSq: 1, compact: [8, 8, 48, 20], full: [8, 8, 208, 108] },
    { id: "blocker", distanceSq: 100, compact: [50, 80, 60, 92], full: [50, 80, 60, 92] },
  ];
  const result = layoutReadoutDetails(cards, [], [8, 8, 400, 400], 14, true, null);
  expect(result.get("near")!.box).toEqual([8, 8, 208, 108]);
  expect(result.get("blocker")!.box).toEqual([50, 122, 60, 134]);
});

test("hover reveals a farther card without Space and leaving restores compact cards", () => {
  const cards: DetailCard[] = [
    { id: "near", distanceSq: 1, compact: [100, 100, 140, 110], full: [100, 100, 260, 200] },
    { id: "far", distanceSq: 100, compact: [200, 20, 240, 30], full: [200, 20, 260, 220] },
  ];
  let result = layoutReadoutDetails(cards, [], [0, 0, 800, 600], 14, false, "far");
  expect(result.get("far")!.full).toBe(true);
  expect(result.get("near")!.full).toBe(false);
  result = layoutReadoutDetails(cards, [], [0, 0, 800, 600], 14, false, null);
  expect([...result.values()].map((p) => p.full)).toEqual([false, false]);
  expect(result.get("far")!.box).toEqual(cards[1].compact);
});

// @vitest-environment node
import { expect, test } from "vitest";
import { MusicDirector, type MusicDirection } from "@packages/battle-audio/src/music/musicDirector";

const RULES: MusicDirection = {
  combat_window_s: 20,
  calm_s: 45,
  crossfade_s: 4,
  combat_triggers: ["under_fire", "suppressed", "pinned", "hit", "man_down"],
};
const TRACKS = { tension: ["t1", "t2", "t3"], combat: ["c1", "c2", "c3"] };
/** A fixed sequence standing in for the caller's seeded randomness. */
function sequence(values: number[]) {
  let at = 0;
  return () => values[at++ % values.length];
}

test("an own under-fire bark switches to combat; 45 s without one returns to tension", () => {
  const director = new MusicDirector(RULES, TRACKS, Math.random);
  expect(director.mood(0)).toBe("tension");
  director.note("under_fire", 10);
  expect(director.mood(10)).toBe("combat");
  expect(director.mood(54.9)).toBe("combat");
  director.note("man_down", 50);
  expect(director.mood(94.9)).toBe("combat");
  expect(director.mood(95.1)).toBe("tension");
});

test("only a bark from the last 20 s brings combat in; one older than that never does", () => {
  const director = new MusicDirector(RULES, TRACKS, Math.random);
  director.note("pinned", 0);
  // First heard 25 s after it was shouted: too old to start combat.
  expect(director.mood(25)).toBe("tension");
  director.note("hit", 30);
  expect(director.mood(49.9)).toBe("combat");
});

test("barks outside B5–B9 never bring combat", () => {
  const director = new MusicDirector(RULES, TRACKS, Math.random);
  for (const trigger of ["contact_infantry", "opens_fire", "kill", "reloading", "move"] as const)
    director.note(trigger, 5);
  expect(director.mood(5)).toBe("tension");
});

test("a mood's tracks are shuffled, never one back to back, and every track comes up", () => {
  // A random source that would replay the last track first at every reshuffle.
  const director = new MusicDirector(RULES, TRACKS, sequence([0.99, 0.5, 0.01, 0.2, 0.7]));
  const played = Array.from({ length: 60 }, () => director.next("combat"));
  for (let i = 1; i < played.length; i++) expect(played[i]).not.toBe(played[i - 1]);
  for (let i = 0; i + 3 <= played.length; i += 3)
    expect(new Set(played.slice(i, i + 3))).toEqual(new Set(TRACKS.combat));
  expect(director.next("tension")).toMatch(/^t/);
});

test("a mood with one track repeats it, and a mood with none has nothing to play", () => {
  const director = new MusicDirector(RULES, { tension: ["only"], combat: [] }, Math.random);
  expect([director.next("tension"), director.next("tension")]).toEqual(["only", "only"]);
  expect(director.next("combat")).toBeNull();
});

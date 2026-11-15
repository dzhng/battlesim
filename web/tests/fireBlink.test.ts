import { expect, test } from "vitest";
import { FireBlinks } from "../src/battle/present/fireBlink";
import type { ObservationView, OwnUnitView } from "../src/battle/sim/observationView";

const HZ = 10;
// Two pulses of 0.1 s on and off: a blink lasts 0.4 s, four ticks; a colour
// waits a second before it shows again.
const timing = { on_s: 0.1, off_s: 0.1, wait_s: 1 };
const unit = (id: number, hp: number, shotAt = false, memberHp: number[] = []) =>
  ({ id, hp, memberHp, shotAt }) as unknown as OwnUnitView;
const frame = (tick: number, own: OwnUnitView[]) => ({ tick, own }) as unknown as ObservationView;

test("a card blinks yellow when an aimed round passes and red when its unit is hurt, by the wait and the cut-in", () => {
  const blinks = new FireBlinks(HZ, timing);
  const colour = (id: number) => blinks.feed.current.get(id)?.colour ?? null;
  const serial = (id: number) => blinks.feed.current.get(id)?.serial ?? null;

  blinks.observe(frame(0, [unit(1, 100), unit(2, 0, false, [1, 1])]));
  expect(blinks.feed.current.size).toBe(0);

  // A round aimed at unit 1 passes: yellow.
  blinks.observe(frame(1, [unit(1, 100, true), unit(2, 0, false, [1, 1])]));
  expect(colour(1)).toBe("yellow");
  const first = serial(1);
  // Another round mid-blink is ignored: the blink finishes.
  blinks.observe(frame(2, [unit(1, 100, true), unit(2, 0, false, [1, 1])]));
  expect(serial(1)).toBe(first);
  // A hit cuts in as red, mid-blink.
  blinks.observe(frame(3, [unit(1, 80, true), unit(2, 0, false, [1, 1])]));
  expect(colour(1)).toBe("red");
  // The squad loses a man: red for it too.
  blinks.observe(frame(4, [unit(1, 80), unit(2, 0, false, [1])]));
  expect(colour(2)).toBe("red");

  // Past the blink but inside yellow's wait: another pass shows nothing new,
  // and the finished blinks are gone, so a moved card never replays one.
  blinks.observe(frame(8, [unit(1, 80, true), unit(2, 0, false, [1])]));
  expect(colour(1)).toBe(null);
  expect(colour(2)).toBe(null);
  // Past the wait, yellow shows again.
  blinks.observe(frame(11, [unit(1, 80, true), unit(2, 0, false, [1])]));
  expect(colour(1)).toBe("yellow");
  // Hurt again within red's wait: no new blink; past it, red.
  blinks.observe(frame(12, [unit(1, 70), unit(2, 0, false, [1])]));
  expect(colour(1)).toBe("yellow");
  blinks.observe(frame(16, [unit(1, 60), unit(2, 0, false, [1])]));
  expect(colour(1)).toBe("red");
  // A unit that is gone takes its blink with it.
  blinks.observe(frame(17, [unit(2, 0, false, [1])]));
  expect(colour(1)).toBe(null);
});

test("a new battle starts over", () => {
  const blinks = new FireBlinks(HZ, timing);
  blinks.observe(frame(50, [unit(1, 100)]));
  blinks.observe(frame(51, [unit(1, 90)]));
  expect(blinks.feed.current.get(1)?.colour).toBe("red");
  // The tick runs backwards: a restart. Nothing carries over, and health
  // measured in the old battle is no baseline for the new one.
  blinks.observe(frame(0, [unit(1, 50)]));
  expect(blinks.feed.current.size).toBe(0);
  blinks.observe(frame(1, [unit(1, 40)]));
  expect(blinks.feed.current.get(1)?.colour).toBe("red");
});

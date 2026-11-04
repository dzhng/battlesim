// @vitest-environment node
import { expect, test } from "vitest";
import {
  CAPTION_ROWS,
  CAPTION_SECONDS,
  cueLine,
  foldCaptions,
  type CaptionLine,
} from "../src/battle/present/captions";
import type { SoundCueView } from "../src/battle/sim/observationView";

const HZ = 20;
const CAPTION_TICKS = CAPTION_SECONDS * HZ;

const cue = (c: Partial<SoundCueView>): SoundCueView => ({
  listener: 4,
  category: "shot",
  sector: 0,
  band: "far",
  moving: false,
  ...c,
});
const heard = (c: Partial<SoundCueView>, tick: number, name = "Tank") =>
  cueLine(cue(c), tick, name);

test("the same sound keeps one row, counted, with its latest wording on top", () => {
  let rows: CaptionLine[] = [];
  rows = foldCaptions(rows, [heard({}, 10)], 10, HZ);
  rows = foldCaptions(rows, [heard({ sector: 2 }, 11)], 11, HZ);
  rows = foldCaptions(rows, [heard({ band: "near" }, 12)], 12, HZ);
  expect(rows.map((r) => [r.text, r.count])).toEqual([
    ["Heard gunfire, near, east of Tank", 2],
    ["Heard gunfire, far, north of Tank", 1],
  ]);
});

test("another listener or kind of sound is another row, and only the newest few show", () => {
  // Two units of one type hear as one: the row reads the same for both.
  expect(heard({ listener: 5 }, 5).key).toBe(heard({}, 5).key);
  const rows = foldCaptions(
    [],
    [
      heard({}, 5),
      heard({}, 5, "Rifle squad"),
      heard({ category: "vehicle" }, 5),
      heard({ moving: true }, 5),
    ],
    5,
    HZ,
  );
  expect(rows).toHaveLength(CAPTION_ROWS);
  expect(rows[0].key).toBe(heard({ moving: true }, 5).key);
});

test("a row expires once its sound has not been heard for the caption lifetime", () => {
  const rows = foldCaptions([], [heard({}, 0)], 0, HZ);
  expect(foldCaptions(rows, [], CAPTION_TICKS - 1, HZ)).toHaveLength(1);
  expect(foldCaptions(rows, [], CAPTION_TICKS, HZ)).toHaveLength(0);
});

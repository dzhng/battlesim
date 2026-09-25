// @vitest-environment node
import { expect, test } from "vitest";
import {
  CAPTION_ROWS,
  CAPTION_TICKS,
  cueLine,
  foldCaptions,
  type CaptionLine,
} from "../src/battle/present/captions";
import type { SoundCueView } from "../src/battle/sim/observation";

const cue = (c: Partial<SoundCueView>): SoundCueView => ({
  listener: 4,
  category: "shot",
  sector: 0,
  band: "far",
  moving: false,
  ...c,
});
const heard = (c: Partial<SoundCueView>, tick: number) => cueLine(cue(c), tick, "tank #4");

test("the same sound keeps one row, counted, with its latest wording on top", () => {
  let rows: CaptionLine[] = [];
  rows = foldCaptions(rows, [heard({}, 10)], 10);
  rows = foldCaptions(rows, [heard({ sector: 2 }, 11)], 11);
  rows = foldCaptions(rows, [heard({ band: "near" }, 12)], 12);
  expect(rows.map((r) => [r.text, r.count])).toEqual([
    ["Heard gunfire, near, east of tank #4", 2],
    ["Heard gunfire, far, north of tank #4", 1],
  ]);
});

test("another listener or kind of sound is another row, and only the newest few show", () => {
  const rows = foldCaptions(
    [],
    [
      heard({}, 5),
      heard({ listener: 5 }, 5),
      heard({ category: "vehicle" }, 5),
      heard({ moving: true }, 5),
    ],
    5,
  );
  expect(rows).toHaveLength(CAPTION_ROWS);
  expect(rows[0].key).toBe(heard({ moving: true }, 5).key);
});

test("a row expires once its sound has not been heard for the caption lifetime", () => {
  const rows = foldCaptions([], [heard({}, 0)], 0);
  expect(foldCaptions(rows, [], CAPTION_TICKS - 1)).toHaveLength(1);
  expect(foldCaptions(rows, [], CAPTION_TICKS)).toHaveLength(0);
});

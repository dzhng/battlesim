// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  gameplayField,
  displayFieldValue,
  parseFieldValue,
} from "../../apps/mechanics-editor/src/fields";

describe("mechanics human units", () => {
  it("round trips landing spread without changing angular scatter", () => {
    const field = gameplayField("weapons", ["scatter_mrad"])!;
    const weapon = { range_m: 300, scatter_mrad: 15 };
    expect(displayFieldValue(field, 15, weapon)).toBe("4.5");
    expect(parseFieldValue(field, "4.5", weapon)).toEqual({ value: 15 });
  });
});

it("keeps invalid converted text from becoming an authored value", () => {
  const cadence = gameplayField("weapons", ["magazine", "shot_interval_s"])!;
  expect(parseFieldValue(cadence, "0", {})).toHaveProperty("error");
  expect(parseFieldValue(cadence, "-600", {})).toHaveProperty("error");
  expect(parseFieldValue(cadence, "nonsense", {})).toHaveProperty("error");
  expect(
    parseFieldValue(gameplayField("weapons", ["scatter_mrad"])!, "4.5", { range_m: 0 }),
  ).toHaveProperty("error");
});

it("round trips fractions, cadence and dimensions and retains untouched precision", () => {
  const fraction = gameplayField("weapons", ["armor_fraction"])!;
  expect(displayFieldValue(fraction, 0.15, {})).toBe("15");
  expect(parseFieldValue(fraction, "25", {})).toEqual({ value: 0.25 });
  const cadence = gameplayField("weapons", ["magazine", "shot_interval_s"])!;
  expect(displayFieldValue(cadence, 0.1, {})).toBe("600");
  expect(parseFieldValue(cadence, "300", {})).toEqual({ value: 0.2 });
  const dimensions = gameplayField("units", ["body", "hull", "half_extents_m"])!;
  expect(displayFieldValue(dimensions, [3.5, 1.8, 1.2], {})).toBe("7, 3.6, 2.4");
  expect(parseFieldValue(dimensions, "8, 4, 3", {})).toEqual({ value: [4, 2, 1.5] });
  const entry = { magazine: { shot_interval_s: 0.12345678912345678 } };
  expect(
    parseFieldValue(
      cadence,
      displayFieldValue(cadence, entry.magazine.shot_interval_s, entry),
      entry,
    ),
  ).toEqual({ value: entry.magazine.shot_interval_s });
});

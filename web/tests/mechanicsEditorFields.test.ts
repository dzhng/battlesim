// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  enabledValue,
  gameplayField,
  displayFieldValue,
  parseFieldValue,
  validateGameplayValue,
} from "../../apps/mechanics-editor/src/fields";

describe("mechanics human units", () => {
  it("edits a layer spread in milliradians", () => {
    const field = gameplayField("weapons", ["targets", "ground", "scatter_mrad"])!;
    const weapon = { targets: { ground: { scatter_mrad: 15 } } };
    expect(displayFieldValue(field, 15)).toBe("15");
    expect(parseFieldValue(field, "20", weapon)).toEqual({ value: 20 });
  });
});

it("enables a target layer at its saved values, else within the row's flight bound", () => {
  const ground = gameplayField("weapons", ["targets", "ground"])!;
  const saved = { range_m: 90, scatter_mrad: 3, classes: ["infantry"] };
  expect(enabledValue(ground, { range_m: 120, targets: { ground: saved } })).toEqual(saved);
  expect(enabledValue(ground, { range_m: 120, targets: {} })).toMatchObject({ range_m: 120 });
  expect(enabledValue(ground, { range_m: 0, targets: {} })).toMatchObject({ range_m: 0 });
});

it("keeps invalid converted text from becoming an authored value", () => {
  const cadence = gameplayField("weapons", ["magazine", "shot_interval_s"])!;
  expect(parseFieldValue(cadence, "0", {})).toHaveProperty("error");
  expect(parseFieldValue(cadence, "-600", {})).toHaveProperty("error");
  expect(parseFieldValue(cadence, "nonsense", {})).toHaveProperty("error");
});

it("round trips fractions, cadence and dimensions and retains untouched precision", () => {
  const fraction = gameplayField("weapons", ["gravity_scale"])!;
  expect(displayFieldValue(fraction, 0.15)).toBe("15");
  expect(parseFieldValue(fraction, "25", {})).toEqual({ value: 0.25 });
  const cadence = gameplayField("weapons", ["magazine", "shot_interval_s"])!;
  expect(displayFieldValue(cadence, 0.1)).toBe("600");
  expect(parseFieldValue(cadence, "300", {})).toEqual({ value: 0.2 });
  const dimensions = gameplayField("units", ["body", "hull", "half_extents_m"])!;
  expect(displayFieldValue(dimensions, [3.5, 1.8, 1.2])).toBe("7, 3.6, 2.4");
  expect(parseFieldValue(dimensions, "8, 4, 3", {})).toEqual({ value: [4, 2, 1.5] });
  const entry = { magazine: { shot_interval_s: 0.12345678912345678 } };
  expect(
    parseFieldValue(cadence, displayFieldValue(cadence, entry.magazine.shot_interval_s), entry),
  ).toEqual({ value: entry.magazine.shot_interval_s });
});

it("names the allowed choices when a choice list is not a list", () => {
  const classes = ["targets", "ground", "classes"];
  const expected = "Use a comma-separated list of: infantry, vehicle, missile.";
  expect(validateGameplayValue("weapons", classes, "infantry")).toBe(expected);
  expect(validateGameplayValue("weapons", classes, ["tank"])).toBe(expected);
  expect(validateGameplayValue("weapons", classes, ["infantry"])).toBeUndefined();
});

it("edits required concealment and optional spotting below one", () => {
  const concealment = gameplayField("units", ["concealment"]);
  const spotting = gameplayField("units", ["sensors", "spotting"]);
  expect(concealment).not.toBeNull();
  expect(spotting).not.toBeNull();
  expect(parseFieldValue(concealment!, "0.5", {})).toEqual({ value: 0.5 });
  expect(parseFieldValue(concealment!, "", {})).toHaveProperty("error");
  expect(parseFieldValue(spotting!, "0.8", {})).toEqual({ value: 0.8 });
  expect(parseFieldValue(spotting!, "", {})).toEqual({ value: 1 });
});

it("shows omitted numeric defaults and refuses nonpositive concealment inputs", () => {
  const spotting = gameplayField("units", ["sensors", "spotting"])!;
  expect(displayFieldValue(spotting, undefined)).toBe("1");
  for (const path of [["concealment"], ["sensors", "spotting"]]) {
    const field = gameplayField("units", path)!;
    expect(parseFieldValue(field, "0", {})).toHaveProperty("error");
    expect(parseFieldValue(field, "-0.5", {})).toHaveProperty("error");
  }
});

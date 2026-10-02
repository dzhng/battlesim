// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";

const vectors = JSON.parse(
  readFileSync(
    new URL("../../fixtures/parity/publication/codec-vectors.json", import.meta.url),
    "utf8",
  ),
);
const layout = vectors.layout as ObservationLayout;

function logical(phase: string) {
  const data = new Float32Array(Uint32Array.from(vectors.vectors[phase].bits).buffer);
  const header = Object.fromEntries(layout.header.map((name, i) => [name, data[i]]));
  let at = layout.header.length;
  const groups = layout.groups.map((group) => {
    const start = at;
    at += header[group.count] * group.fields.length;
    for (let row = 0; row < header[group.count]; row++)
      for (const section of group.sections)
        at +=
          data[start + row * group.fields.length + group.fields.indexOf(section.count)] *
          section.fields.length;
    return data.slice(start, at);
  });
  return { header, groups, fog: data.slice(at, at + header.fogFloats) };
}

function packet(
  phase: string,
  revision: number,
  snapshot: boolean,
  overrides: Map<number, number[]> = new Map(),
  sizes: Map<number, number> = new Map(),
  encodings: Map<number, number> = new Map(),
) {
  const { header, groups, fog } = logical(phase);
  Object.assign(header, {
    groundEpoch: 1,
    groundSide: 0,
    groundBase: 0,
    groundRevision: 0,
    groundFull: Number(revision === 1),
    groundRunCount: 0,
    fogBaseLo: revision - 1,
    fogBaseHi: 0,
    fogRevisionLo: revision,
    fogRevisionHi: 0,
    fogFull: Number(revision === 1),
    fogFloats: revision === 1 ? fog.length : 0,
  });
  for (const [i, size] of sizes)
    if (layout.groups[i].sections.length === 0)
      header[layout.groups[i].count] = size / layout.groups[i].fields.length;
  const wire = layout.header.map((name) => header[name]);
  groups.forEach((group, i) => {
    const payload = overrides.get(i) ?? (snapshot ? Array.from(group) : []);
    wire.push(
      sizes.get(i) ?? group.length,
      encodings.get(i) ?? Number(snapshot),
      payload.length,
      ...payload,
    );
  });
  if (revision === 1) wire.push(...fog);
  return new Float32Array(wire);
}

test("group replacements preserve signed zero, earlier views and untouched persistent rows", () => {
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  const corpseGroup = layout.groups.findIndex((g) => g.name === "corpses");
  const after = decoder.decode(packet("base", 2, false, new Map([[corpseGroup, [0, 1, -0]]])))!;
  expect(Object.is(after.corpses[0].position[0], -0)).toBe(true);
  expect(before.corpses[0].position[0]).toBe(vectors.vectors.base.frame.corpses[0].position[0]);
  expect(after.knownProps).toEqual(before.knownProps);
  expect(after.own).toEqual(before.own);
});

test("bad group ranges cannot advance any cursor; a corrected generation still applies", () => {
  const decoder = new ObservationDecoder(layout);
  decoder.decode(packet("base", 1, true));
  const corpseGroup = layout.groups.findIndex((g) => g.name === "corpses");
  expect(() =>
    decoder.decode(packet("base", 2, false, new Map([[corpseGroup, [999, 1, 10]]]))),
  ).toThrow(/range/);
  const frame = decoder.decode(packet("base", 2, false, new Map([[corpseGroup, [0, 1, 10]]])))!;
  expect(frame.corpses[0].position[0]).toBe(10);
  expect(() => decoder.decode(packet("base", 4, false))).toThrow(/baseline/);
});

test("group growth must supply every new word and rejects gaps without consuming the generation", () => {
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  const group = layout.groups.findIndex((g) => g.name === "corpses");
  const words = Array.from(logical("base").groups[group]);
  const sizes = new Map([[group, words.length * 2]]);
  const missingFirstWord = [words.length + 1, words.length - 1, ...words.slice(1)];
  expect(() =>
    decoder.decode(packet("base", 2, false, new Map([[group, missingFirstWord]]), sizes)),
  ).toThrow(/range/);
  const after = decoder.decode(
    packet("base", 2, false, new Map([[group, [words.length, words.length, ...words]]]), sizes),
  )!;
  expect(after.corpses).not.toBe(before.corpses);
  expect(after.corpses).toEqual([before.corpses[0], before.corpses[0]]);
  expect(before.corpses).toEqual([after.corpses[0]]);
});

test("fixed-row copies retain exact canonical order through insertion, removal and reorder", () => {
  const group = layout.groups.findIndex((g) => g.name === "corpses");
  const a = Array.from(logical("base").groups[group]);
  const b = [...a];
  b[0] = 5;
  b[2] = -0;
  b[4] += 1;
  const c = [...a];
  c[0] = 7;
  c[4] += 2;
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(
    packet("base", 1, true, new Map([[group, [...a, ...b]]]), new Map([[group, 18]])),
  )!;
  const after = decoder.decode(
    packet(
      "base",
      2,
      false,
      new Map([[group, [9, 9, -1, 9, ...c, 0, 9]]]),
      new Map([[group, 27]]),
      new Map([[group, 2]]),
    ),
  )!;
  const oracle = new ObservationDecoder(layout).decode(
    packet("base", 1, true, new Map([[group, [...b, ...c, ...a]]]), new Map([[group, 27]])),
  )!;
  expect(after.corpses).toEqual(oracle.corpses);
  expect(Object.is(after.corpses[0].position[2], -0)).toBe(true);
  expect(before.corpses).toEqual([after.corpses[2], after.corpses[0]]);
  const removed = decoder.decode(
    packet(
      "base",
      3,
      false,
      new Map([[group, [18, 9, 0, 9]]]),
      new Map([[group, 18]]),
      new Map([[group, 2]]),
    ),
  )!;
  expect(removed.corpses).toEqual(before.corpses);
});

test("malformed row copies leave the generation available for a corrected complete assembly", () => {
  const group = layout.groups.findIndex((g) => g.name === "corpses");
  const words = Array.from(logical("base").groups[group]);
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  const sizes = new Map([[group, words.length * 2]]);
  const encodings = new Map([[group, 2]]);
  for (const invalid of [
    [1, 9],
    [9, 9],
    [0, 10],
    [-2, 9],
    [-1, 9, ...words.slice(1)],
    [0, 9],
  ]) {
    expect(() =>
      decoder.decode(packet("base", 2, false, new Map([[group, invalid]]), sizes, encodings)),
    ).toThrow(/row cop/);
  }
  const after = decoder.decode(
    packet("base", 2, false, new Map([[group, [0, 9, 0, 9]]]), sizes, encodings),
  )!;
  expect(after.corpses).not.toBe(before.corpses);
  expect(after.corpses).toEqual([before.corpses[0], before.corpses[0]]);
  expect(before.corpses).toEqual([after.corpses[0]]);
});

test("variable word copies insert and remove a route without consuming invalid generations", () => {
  const group = layout.groups.findIndex((g) => g.name === "own");
  const words = Array.from(logical("base").groups[group]);
  const fields = layout.groups[group].fields;
  const routeCount = fields.indexOf("routeCount");
  const routeStart = fields.length;
  const added = [...words];
  added[routeCount] = 1;
  added.splice(routeStart, 0, 3, -0);
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  const sizes = new Map([[group, added.length]]);
  const encodings = new Map([[group, 2]]);
  for (const invalid of [
    [words.length, 1],
    [-2, words.length],
    [-1, 2, 3],
    [1, words.length],
  ]) {
    expect(() =>
      decoder.decode(packet("base", 2, false, new Map([[group, invalid]]), sizes, encodings)),
    ).toThrow(/cop/);
  }
  const payload = [
    0,
    routeCount,
    -1,
    1,
    1,
    routeCount + 1,
    routeStart - routeCount - 1,
    -1,
    2,
    3,
    -0,
    routeStart,
    words.length - routeStart,
  ];
  const after = decoder.decode(
    packet("base", 2, false, new Map([[group, payload]]), sizes, encodings),
  )!;
  const oracle = new ObservationDecoder(layout).decode(
    packet("base", 1, true, new Map([[group, added]]), sizes),
  )!;
  expect(after.own).toEqual(oracle.own);
  expect(Object.is(after.own[0].route[0][1], -0)).toBe(true);
  expect(before.own[0].route).toEqual([]);
  expect(() => decoder.decode(packet("base", 4, false))).toThrow(/baseline/);
  const removed = decoder.decode(
    packet(
      "base",
      3,
      false,
      new Map([
        [
          group,
          [
            0,
            routeCount,
            -1,
            1,
            0,
            routeCount + 1,
            routeStart - routeCount - 1,
            routeStart + 2,
            words.length - routeStart,
          ],
        ],
      ]),
      new Map([[group, words.length]]),
      encodings,
    ),
  )!;
  expect(removed.own).toEqual(before.own);
  expect(after.own).toEqual(oracle.own);
});

test("unchanged static groups reuse immutable arrays and rows while own units change", () => {
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  const own = layout.groups.findIndex((g) => g.name === "own");
  const x = layout.groups[own].fields.indexOf("x");
  const after = decoder.decode(packet("base", 2, false, new Map([[own, [x, 1, 17]]])))!;
  expect(after.own[0].position[0]).toBe(17);
  expect(before.own[0].position[0]).toBe(10);
  expect(after.corpses).toBe(before.corpses);
  expect(after.corpses[0]).toBe(before.corpses[0]);
  expect(after.corpses[0].position).toBe(before.corpses[0].position);
  expect(after.knownProps).toBe(before.knownProps);
  expect(after.knownProps[0]).toBe(before.knownProps[0]);
  expect(after.knownProps[0].half).toBe(before.knownProps[0].half);
});

test("cached static views still validate counts and commit only after the complete frame", () => {
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  const corpse = layout.groups.findIndex((g) => g.name === "corpses");
  const prop = layout.groups.findIndex((g) => g.name === "knownProps");
  for (const group of [corpse, prop]) {
    const invalid = packet("base", 2, false);
    invalid[layout.header.indexOf(layout.groups[group].count)] = 0;
    expect(() => decoder.decode(invalid)).toThrow(/counts/);
  }
  const rejected = packet(
    "base",
    2,
    false,
    new Map([
      [corpse, [0, 1, 99]],
      [prop, [layout.groups[prop].fields.indexOf("x"), 1, 88]],
    ]),
  );
  rejected[layout.header.indexOf("groundRunCount")] = 1;
  expect(() => decoder.decode(rejected)).toThrow(/length/);
  const corrected = decoder.decode(packet("base", 2, false))!;
  expect(corrected.corpses).toBe(before.corpses);
  expect(corrected.knownProps).toBe(before.knownProps);
  const changed = decoder.decode(
    packet(
      "base",
      3,
      false,
      new Map([
        [corpse, [0, 1, 99]],
        [prop, [layout.groups[prop].fields.indexOf("x"), 1, 88]],
      ]),
    ),
  )!;
  expect(changed.corpses).not.toBe(before.corpses);
  expect(changed.corpses[0].position[0]).toBe(99);
  expect(changed.knownProps).not.toBe(before.knownProps);
  expect(changed.knownProps[0].center[0]).toBe(88);
  expect(before.corpses[0].position[0]).toBe(2);
  expect(before.knownProps[0].center).toEqual(vectors.vectors.base.frame.known_props[0].center);
  const steady = decoder.decode(packet("base", 4, false))!;
  expect(steady.corpses).toBe(changed.corpses);
  expect(steady.knownProps).toBe(changed.knownProps);
});

test("new epochs and side invalidation replace static views without accepting stale snapshots", () => {
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  decoder.invalidate("red");
  expect(decoder.decode(packet("base", 1, true))).toBeNull();
  const red = (revision: number, snapshot: boolean) => {
    const record = packet("base", revision, snapshot);
    record[layout.header.indexOf("groundEpoch")] = 2;
    record[layout.header.indexOf("groundSide")] = 1;
    return record;
  };
  const fresh = decoder.decode(red(1, true))!;
  expect(fresh.corpses).toEqual(before.corpses);
  expect(fresh.corpses).not.toBe(before.corpses);
  expect(fresh.knownProps).not.toBe(before.knownProps);
  expect(decoder.decode(packet("base", 1, true))).toBeNull();
  const next = decoder.decode(red(2, false))!;
  expect(next.corpses).toBe(fresh.corpses);
  expect(next.knownProps).toBe(fresh.knownProps);
});

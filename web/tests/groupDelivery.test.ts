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
  expect(() => decoder.decode(rejected)).toThrow(/ground.*count/);
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

/** Independent little-endian bit fixture; encoded payload never passes through JS floats. */
function compactBits(write: (put: (value: number, bits: number) => void) => void) {
  let value = 0n;
  let count = 0n;
  write((word, bits) => {
    value |= BigInt(word >>> 0) << count;
    count += BigInt(bits);
  });
  const result = new Uint32Array(Math.ceil(Number(count) / 32));
  for (let i = 0; i < result.length; i++)
    result[i] = Number((value >> BigInt(i * 32)) & 0xffffffffn);
  return new Float32Array(result.buffer);
}
function compactPacket(revision: number, group: number, body: Float32Array) {
  const ordinary = packet("base", revision, revision === 1);
  let at = layout.header.length;
  for (let i = 0; i < group; i++) at += 3 + ordinary[at + 2];
  const end = at + 3 + ordinary[at + 2];
  const result = new Float32Array(ordinary.length - (end - at - 3) + body.length);
  result.set(ordinary.subarray(0, at + 3));
  result[at + 1] = 3;
  result[at + 2] = body.length;
  result.set(body, at + 3);
  result.set(ordinary.subarray(end), at + 3 + body.length);
  return result;
}

test("packed snapshots and residuals preserve NaN payload baselines, signed zero and prior frames", () => {
  const decoder = new ObservationDecoder(layout);
  const g = layout.groups.findIndex((group) => group.name === "corpses");
  const raw = new Uint32Array(logical("base").groups[g].slice().buffer);
  raw[0] = 0x7fc12345;
  raw[2] = 0x80000000;
  const cold = compactBits((put) => {
    put(1, 8);
    for (const word of raw) {
      put(4, 4);
      put(word, 32);
    }
  });
  const before = decoder.decode(compactPacket(1, g, cold))!;
  expect(Number.isNaN(before.corpses[0].position[0])).toBe(true);
  expect(Object.is(before.corpses[0].position[2], -0)).toBe(true);
  const delta = compactBits((put) => {
    put(0, 8);
    put(1, 8); // replacement, one operation
    put(0, 8);
    put(1, 8); // start, length
    put(9, 4);
    put(0x7fc12345 ^ 0x3fc00000, 32); // four-byte baseline XOR
  });
  const after = decoder.decode(compactPacket(2, g, delta))!;
  expect(after.corpses[0].position[0]).toBe(1.5);
  expect(Object.is(after.corpses[0].position[2], -0)).toBe(true);
  expect(Number.isNaN(before.corpses[0].position[0])).toBe(true);
});

test("malformed packed groups roll back atomically and allow the corrected same generation", () => {
  const decoder = new ObservationDecoder(layout);
  const before = decoder.decode(packet("base", 1, true))!;
  const g = layout.groups.findIndex((group) => group.name === "corpses");
  const fixtures = [
    compactBits((put) => put(3, 8)),
    compactBits((put) => {
      put(0, 8);
      put(128, 8);
      put(0, 8);
    }),
    compactBits((put) => {
      put(0, 8);
      for (let i = 0; i < 4; i++) put(128, 8);
    }),
    compactBits((put) => {
      put(0, 8);
      put(1, 8);
      put(9, 8);
      put(1, 8);
    }),
    compactBits((put) => {
      put(0, 8);
      put(1, 8);
      put(0, 8);
      put(1, 8);
      put(10, 4);
    }),
    compactBits((put) => {
      put(0, 8);
      put(1, 8);
      put(0, 8);
      put(1, 8);
      put(4, 4);
    }),
    ...[
      [2, 9],
      [10, 9],
      [1, 8],
      [1, 0],
    ].map(([source, count]) =>
      compactBits((put) => {
        put(2, 8);
        put(source, 8);
        put(count, 8);
      }),
    ),
    compactBits((put) => {
      put(0, 8);
      put(0, 8);
      put(1, 1);
    }),
    compactBits((put) => {
      put(0, 8);
      put(0, 8);
      put(0, 32);
    }),
  ];
  for (const body of fixtures)
    expect(() => decoder.decode(compactPacket(2, g, body))).toThrow(/packed/);
  const valid = compactBits((put) => {
    put(0, 8);
    put(1, 8);
    put(0, 8);
    put(1, 8);
    put(4, 4);
    put(0x41200000, 32);
  });
  const after = decoder.decode(compactPacket(2, g, valid))!;
  expect(after.corpses[0].position[0]).toBe(10);
  expect(before.corpses[0].position[0]).toBe(vectors.vectors.base.frame.corpses[0].position[0]);
  expect(after.knownProps).toBe(before.knownProps);
  expect(() => decoder.decode(compactPacket(4, g, valid))).toThrow(/baseline/);
  decoder.invalidate("red");
  expect(decoder.decode(compactPacket(2, g, valid))).toBeNull();
  const coldXor = compactBits((put) => {
    put(1, 8);
    put(5, 4);
  });
  expect(() => new ObservationDecoder(layout).decode(compactPacket(1, g, coldXor))).toThrow(/tag/);
});

function groundPacket(revision: number, rows: number, body: Float32Array) {
  const base = packet("base", revision, revision === 1);
  base[layout.header.indexOf("groundRunCount")] = rows;
  base[layout.header.indexOf("groundBase")] = revision - 1;
  base[layout.header.indexOf("groundRevision")] = revision;
  const result = new Float32Array(base.length + body.length);
  result.set(base);
  result.set(body, base.length);
  return result;
}

test("packed ground preserves marks and retained rows, rejects invalid tails before retry", () => {
  const decoder = new ObservationDecoder(layout);
  const full = compactBits((put) => {
    put(0, 8);
    put(0, 8);
    put(255, 8);
    put(31, 5);
    for (const mark of [17, 29, 31, 43, 255]) put(mark, 8);
    put(1, 8);
    put(7, 8);
    put(0, 8);
    put(0, 5);
  });
  const before = decoder.decode(groundPacket(1, 2, full))!;
  expect([...before.groundPatch.runs]).toEqual([
    0,
    65536,
    17 + 29 * 256,
    31 + 43 * 256 + 255 * 65536,
    1,
    263,
    0,
    0,
  ]);
  const valid = compactBits((put) => {
    put(1, 8);
    put(7, 8);
    put(0, 8);
    put(16, 5);
    put(255, 8);
  });
  for (const invalid of [
    valid.subarray(0, 1),
    compactBits((put) => {
      put(128, 8);
      put(0, 8);
      put(0, 8);
      put(0, 8);
      put(0, 5);
    }),
    compactBits((put) => {
      put(1, 8);
      put(255, 8);
      put(1, 8);
      put(0, 5);
    }),
    compactBits((put) => {
      put(1, 8);
      put(0, 8);
      put(0, 8);
      put(1, 5);
      put(0, 8);
    }),
    compactBits((put) => {
      put(1, 8);
      put(7, 8);
      put(0, 8);
      put(16, 5);
      put(255, 8);
      put(1, 1);
    }),
  ])
    expect(() => decoder.decode(groundPacket(2, 1, invalid))).toThrow();
  for (const count of [NaN, Infinity, -1, 0.5, 2 ** 24])
    expect(() => decoder.decode(groundPacket(2, count, valid))).toThrow();
  const after = decoder.decode(groundPacket(2, 1, valid))!;
  expect([...after.groundPatch.runs]).toEqual([1, 263, 0, 255 * 65536]);
  expect([...before.groundPatch.runs]).toEqual([
    0,
    65536,
    17 + 29 * 256,
    31 + 43 * 256 + 255 * 65536,
    1,
    263,
    0,
    0,
  ]);
});

test("a packed ground NaN carrier reconstructs finite canonical rows bit-exactly", () => {
  const body = compactBits((put) => {
    put(69, 8);
    put(35, 8);
    put(193, 8);
    put(31, 5);
    for (const mark of [3, 255, 255, 255, 255]) put(mark, 8);
  });
  expect(new Uint32Array(body.buffer)[0]).toBe(0x7fc12345);
  const copied = body.slice();
  const wider = { ...layout, ground: { ...layout.ground, cols: 160, rows: 160 } };
  const frame = new ObservationDecoder(wider).decode(groundPacket(1, 1, copied))!;
  const expected = Float32Array.of(69, 35 + 194 * 256, 3 + 255 * 256, 16777215);
  expect(new Uint32Array(frame.groundPatch.runs.buffer)).toEqual(new Uint32Array(expected.buffer));
});

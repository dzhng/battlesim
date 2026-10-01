// @vitest-environment node
// The grass build's prop table: where no grass grows. A clump asks only its
// own cell's list, so the list must hold every prop whose bare footprint
// covers any point of the cell; the answer is then the one a walk over every
// prop gives, whatever the map's size.
import { expect, test } from "vitest";
import { mulberry32, random } from "math/random";
import { packGrassProps } from "@packages/battle-renderer/src/terrain/grassField.ts";

const MARGIN = 0.6;

/** Random building-sized and trunk-sized footprints over a 3 km square. */
function footprints(count: number, seed: number): Float32Array {
  const state = mulberry32.create(seed);
  const rng = () => mulberry32.sample(state);
  const out = new Float32Array(count * 5);
  for (let i = 0; i < count; i++) {
    const trunk = random.bool(rng, 0.7);
    out.set(
      [
        random.float(rng, 500, 3500),
        random.float(rng, 200, 3200),
        random.float(rng, -Math.PI, Math.PI),
        trunk ? 0.35 : random.float(rng, 3, 45),
        trunk ? 0.35 : random.float(rng, 3, 20),
      ],
      i * 5,
    );
  }
  return out;
}

/** Whether (x, y) is on any footprint widened by the margin: every prop asked. */
function covered(props: Float32Array, x: number, y: number): boolean {
  for (let o = 0; o < props.length; o += 5) {
    const [px, py, yaw, hx, hy] = props.subarray(o, o + 5);
    const [dx, dy] = [x - px, y - py];
    const [c, s] = [Math.cos(yaw), Math.sin(yaw)];
    if (Math.abs(dx * c + dy * s) < hx + MARGIN && Math.abs(-dx * s + dy * c) < hy + MARGIN)
      return true;
  }
  return false;
}

/** The shader's reading of the table (`grassUnderProp`): the point's cell,
 *  then that cell's records alone. Returns the answer and how many it asked. */
function lookup(table: Float32Array, x: number, y: number) {
  const words = new Uint32Array(table.buffer);
  const [ox, oy, cell] = [table[0], table[1], table[2]];
  const [nx, ny] = [words[4], words[5]];
  const [gx, gy] = [
    Math.fround((Math.fround(x) - ox) / cell),
    Math.fround((Math.fround(y) - oy) / cell),
  ];
  if (gx < 0 || gy < 0 || Math.floor(gx) >= nx || Math.floor(gy) >= ny)
    return { hit: false, asked: 0 };
  const at = Math.floor(gy) * nx + Math.floor(gx);
  const range = (2 + Math.floor(at / 2)) * 4 + (at % 2) * 2;
  const [first, count] = [words[range], words[range + 1]];
  for (let i = 0; i < count; i++) {
    const o = (first + 2 * i) * 4;
    const [dx, dy] = [x - table[o], y - table[o + 1]];
    const [c, s] = [table[o + 2], table[o + 3]];
    if (Math.abs(dx * c + dy * s) < table[o + 4] && Math.abs(-dx * s + dy * c) < table[o + 5])
      return { hit: true, asked: i + 1 };
  }
  return { hit: false, asked: count };
}

test("a point's cell lists every prop that could cover it, and few others", () => {
  const props = footprints(4000, 7);
  const table = packGrassProps(props, MARGIN);
  const state = mulberry32.create(11);
  const rng = () => mulberry32.sample(state);
  let hits = 0;
  let asked = 0;
  const POINTS = 20000;
  for (let k = 0; k < POINTS; k++) {
    // Half the points on or beside a prop, where the answer changes.
    const near = k % 2 === 0;
    const o = random.int(rng, 0, 3999) * 5;
    const reach = Math.hypot(props[o + 3], props[o + 4]) + 2;
    const x = near ? props[o] + random.float(rng, -reach, reach) : random.float(rng, 0, 4000);
    const y = near ? props[o + 1] + random.float(rng, -reach, reach) : random.float(rng, 0, 4000);
    const got = lookup(table, x, y);
    expect(got.hit, `at ${x}, ${y}`).toBe(covered(props, x, y));
    hits += got.hit ? 1 : 0;
    asked += got.asked;
  }
  // Both answers occur, and a point asks a handful of props, not the map's.
  expect(hits).toBeGreaterThan(POINTS / 10);
  expect(hits).toBeLessThan(POINTS / 2);
  expect(asked / POINTS).toBeLessThan(8);
});

test("a map with no props answers no everywhere", () => {
  const table = packGrassProps(new Float32Array(0), MARGIN);
  expect(lookup(table, 10, 10)).toEqual({ hit: false, asked: 0 });
});

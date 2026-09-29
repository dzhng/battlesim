// @vitest-environment node
import { expect, test } from "vitest";
import { vec3 } from "math";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh.ts";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";

type P3 = [number, number, number];

function positions(mesh: Float32Array): P3[] {
  const out: P3[] = [];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push([mesh[i], mesh[i + 1], mesh[i + 2]]);
  return out;
}

/** Distance from p to the segment a–b. */
function toSegment(p: P3, a: P3, b: P3) {
  const d = vec3.sub([0, 0, 0], b, a);
  const t = Math.max(
    0,
    Math.min(1, vec3.dot(vec3.sub([0, 0, 0], p, a), d) / vec3.squaredLength(d)),
  );
  return vec3.distance(p, vec3.scaleAndAdd([0, 0, 0], a, d, t));
}

test("a trace tube hugs the flown chords within its half width", () => {
  const points: P3[] = [
    [0, 0, 1],
    [30, 0, 1.5],
    [60, 2, 1.2],
    // A vertical chord exercises the tube's other cross-section frame.
    [60, 2, -3],
  ];
  const half = 0.2;
  const { opaque, translucent } = buildFlightOverlay([{ points, outcome: "flying" }], [], [], half);
  expect(translucent.length).toBe(0);
  const verts = positions(opaque);
  expect(verts.length).toBeGreaterThan(0);
  const chords = points.slice(1).map((b, i) => [points[i], b] as const);
  // Each square corner sits at the tube's full radius from a flown chord,
  // regardless of triangle order or how many faces tessellate the tube.
  for (const v of verts)
    expect(chords.some(([a, b]) => Math.abs(toSegment(v, a, b) - half * Math.SQRT2) < 1e-5)).toBe(
      true,
    );
  // Every chord carries faces near its middle; endpoint caps alone cannot pass.
  const centres: P3[] = [];
  for (let i = 0; i < verts.length; i += 3) {
    const sum = vec3.add([0, 0, 0], verts[i], verts[i + 1]);
    vec3.add(sum, sum, verts[i + 2]);
    centres.push(vec3.scale(sum, sum, 1 / 3));
  }
  for (const [a, b] of chords) {
    const middle = vec3.lerp([0, 0, 0], a, b, 0.5);
    expect(Math.min(...centres.map((c) => vec3.distance(c, middle)))).toBeLessThan(
      vec3.distance(a, b) / 4 + half * Math.SQRT2,
    );
  }
});

test("a rejected arc is drawn translucent and marks are centred on their points", () => {
  const blocked: P3 = [40, 0, 3];
  const { opaque, translucent } = buildFlightOverlay(
    [{ points: [[0, 0, 1], blocked], outcome: "blocked" }],
    [{ at: blocked, kind: "blocked" }],
    [],
    0.1,
  );
  expect(translucent.length).toBeGreaterThan(0);
  const mark = positions(opaque);
  const centre = [0, 1, 2].map((k) => mark.reduce((s, v) => s + v[k], 0) / mark.length);
  expect(centre[0]).toBeCloseTo(blocked[0], 6);
  expect(centre[1]).toBeCloseTo(blocked[1], 6);
  expect(centre[2]).toBeCloseTo(blocked[2], 6);
});

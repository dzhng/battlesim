// @vitest-environment node
import { expect, test } from "vitest";
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
  const d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  const len2 = d[0] ** 2 + d[1] ** 2 + d[2] ** 2;
  const t = Math.max(
    0,
    Math.min(1, ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1] + (p[2] - a[2]) * d[2]) / len2),
  );
  return Math.hypot(p[0] - a[0] - d[0] * t, p[1] - a[1] - d[1] * t, p[2] - a[2] - d[2] * t);
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
  // 6 quads (12 triangles) per chord.
  expect(verts.length).toBe(3 * 36);
  verts.forEach((v, i) => {
    const chord = Math.floor(i / 36);
    const near = toSegment(v, points[chord], points[chord + 1]);
    // Tube corners sit on the square of half width `half` around their chord.
    expect(near).toBeCloseTo(half * Math.SQRT2, 5);
  });
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

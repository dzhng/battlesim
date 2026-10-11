// @vitest-environment node
import { expect, test } from "vitest";
// @ts-expect-error: a plain-JS tool beside scene.mjs
import { frameAnatomy, parseRows, unionLength } from "../gpu-trace.mjs";

/** One metal-gpu-intervals row as xctrace exports it: the first use of a value
 *  carries an id, later uses only a ref to it. */
const row = (
  start: number,
  duration: number,
  channel: string,
  frame: number,
  label: string,
  process: string,
  depth = 0,
) =>
  `<row><start-time fmt="t">${start}</start-time><duration fmt="d">${duration}</duration>` +
  `<gpu-channel-name fmt="${channel}">${channel}</gpu-channel-name><gpu-frame-number fmt="Frame ${frame}">${frame}</gpu-frame-number>` +
  `<sentinel/><metal-nesting-level fmt="${depth}">${depth}</metal-nesting-level>` +
  `<formatted-label fmt="${label}"><string fmt="x">x</string></formatted-label><sentinel/><sentinel/><sentinel/>` +
  `${process}</row>`;
const godot = '<process id="9" fmt="godot (1)"><pid fmt="1">1</pid></process>';
const godotRef = '<process ref="9"/>';
const other = '<process fmt="Chrome Helper (2)"/>';

test("rows resolve each ref to the element its id named, at any depth", () => {
  const rows = parseRows(
    `<trace-query-result><node><schema/>${row(0, 5, "Vertex", 1, "Render Command 1", godot)}${row(10, 5, "Fragment", 1, "Render Command 1", godotRef)}</node></trace-query-result>`,
  );
  expect(rows).toHaveLength(2);
  expect(rows[1][10].attrs.fmt).toBe("godot (1)");
  expect(rows[1][2].attrs.fmt).toBe("Fragment");
});

test("overlapping intervals count once", () => {
  expect(
    unionLength([
      [0, 10],
      [5, 15],
      [20, 25],
    ]),
  ).toBe(20);
  expect(unionLength([])).toBe(0);
});

test("a frame runs from its first GPU work to the next frame's: busy is its merged work, idle the rest, and other processes are counted apart", () => {
  const ms = 1e6;
  const xml = `<r>${[
    // Frame 1 is cut by the trace's start and dropped, as is the last frame.
    row(0, 1 * ms, "Vertex", 1, "Render Command 1", godot),
    // Frame 2: 0–4 ms vertex and 2–6 ms fragment overlap (6 ms busy) in a
    // 10 ms span; a nested interval adds no busy time.
    row(10 * ms, 4 * ms, "Vertex", 2, "Command Buffer 0:Render Command 3  (godot)", godotRef),
    row(12 * ms, 4 * ms, "Fragment", 2, "Command Buffer 0:Render Command 3  (godot)", godotRef),
    row(11 * ms, 1 * ms, "Vertex", 2, "Render Command 3", godotRef, 1),
    // Another process's work in the same window.
    row(13 * ms, 2 * ms, "Compute", 7, "Compute Command 0", other),
    row(20 * ms, 1 * ms, "Compute", 3, "Compute Command 2", godotRef),
    row(30 * ms, 1 * ms, "Compute", 4, "Compute Command 2", godotRef),
  ].join("")}</r>`;
  const anatomy = frameAnatomy(parseRows(xml), "godot");
  expect(anatomy.frames).toBe(2);
  expect(anatomy.perFrame[0]).toEqual({ frame: 2, spanMs: 10, busyMs: 6, idleMs: 4 });
  expect(anatomy.encodersMs["Vertex · Render Command 3"]).toBe(2);
  expect(anatomy.encodersMs["Vertex · Render Command 3 (nested)"]).toBe(0.5);
  expect(anatomy.foreign).toEqual({ busyMs: 2, share: 2 / (1 + 6 + 2 + 1 + 1) });
});

// @vitest-environment node
// A capture's recorded indicators: each frame gets back exactly the overlay
// the page uploaded, its parts combined in order; a part repeated between
// rebuilds is kept once; a stale or malformed file is refused.
import { expect, test } from "vitest";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import {
  OverlayRecording,
  decodeOverlayCapture,
  type OverlayPart,
} from "@web/battle/benchmark/overlayCapture";

const vertices = (n: number, from = 0) => Float32Array.from({ length: n * 10 }, (_, i) => from + i);

/** Frame `index` of a decoded file, its parts combined as the page combined them. */
function drawn(bytes: Uint8Array, index: number) {
  const { parts, frames } = decodeOverlayCapture(bytes);
  return combineWorldMeshes(
    frames[index].map((p) => {
      const [opaque, translucent, painted, paintedMarching] = parts[p];
      // Decode allocates ArrayBuffer-backed floats; the Mesh alias names that.
      return {
        opaque: new Float32Array(opaque),
        translucent: new Float32Array(translucent),
        painted: new Float32Array(painted),
        paintedMarching: new Float32Array(paintedMarching),
      };
    }),
  );
}

test("each frame reads back the overlay the page combined and uploaded", () => {
  const border: OverlayPart = { painted: vertices(400) };
  const glyphs = (x: number): OverlayPart => ({ translucent: vertices(3, x) });
  const recording = new OverlayRecording();
  const frames: (OverlayPart[] | undefined)[] = [
    undefined,
    [border, glyphs(0)],
    [border, glyphs(0)],
    [border, glyphs(500), { paintedMarching: vertices(2, 9) }],
  ];
  for (const parts of frames) recording.frame(parts);
  const bytes = recording.encode();
  expect(decodeOverlayCapture(bytes).frames).toHaveLength(frames.length);
  expect(drawn(bytes, 0)).toEqual(combineWorldMeshes([]));
  for (const [i, parts] of frames.entries())
    if (parts) expect(drawn(bytes, i), `frame ${i}`).toEqual(combineWorldMeshes(parts));
});

test("a part repeated between rebuilds, the same object or the same floats, is kept once", () => {
  const recording = new OverlayRecording();
  const border = { painted: vertices(400) };
  for (let tick = 0; tick < 30; tick++)
    recording.frame([border, { painted: vertices(64, 7) }, { translucent: vertices(3, tick) }]);
  const { parts } = decodeOverlayCapture(recording.encode());
  expect(parts).toHaveLength(2 + 30);
});

test("a channel of partial vertices is refused as it is recorded", () => {
  expect(() => new OverlayRecording().frame([{ opaque: new Float32Array(9) }])).toThrow(
    /not whole vertices/,
  );
});

/** The documented layout of one part (an opaque vertex, 0…9) drawn by the
 *  second of two frames: the bytes `native/client/src/overlays.rs` reads. */
const GOLDEN =
  "424f564c" + // "BOVL"
  "01000000" + // version 1
  "02000000" + // 2 frames
  "01000000" + // 1 part
  "0a000000" +
  "00000000".repeat(3) + // 10, 0, 0, 0 floats per channel
  "00000000" +
  "0000803f" +
  "00000040" +
  "00004040" +
  "00008040" +
  "0000a040" +
  "0000c040" +
  "0000e040" +
  "00000041" +
  "00001041" + // 0.0 … 9.0
  "00000000" + // frame 0: no parts
  "01000000" +
  "00000000"; // frame 1: part 0

test("the file is the documented layout", () => {
  const recording = new OverlayRecording();
  recording.frame([]);
  recording.frame([{ opaque: vertices(1) }]);
  expect(Buffer.from(recording.encode()).toString("hex")).toBe(GOLDEN);
});

test("a stale or malformed file is refused", () => {
  const golden = Uint8Array.from(Buffer.from(GOLDEN, "hex"));
  expect(() => decodeOverlayCapture(golden)).not.toThrow();
  const refused = (edit: (b: Uint8Array) => Uint8Array, says: RegExp) =>
    expect(() => decodeOverlayCapture(edit(golden.slice()))).toThrow(says);
  refused((b) => ((b[0] = 0x58), b), /not a recorded overlay file/);
  refused((b) => ((b[4] = 2), b), /version 2/);
  refused((b) => ((b[16] = 9), b), /not whole 10-float vertices/);
  refused((b) => ((b[b.length - 4] = 1), b), /names a part past/);
  refused((b) => b.slice(0, b.length - 2), /ends early/);
  refused((b) => Uint8Array.from([...b, 0]), /past the last frame/);
});

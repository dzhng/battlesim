// @vitest-environment node
// The workbench's CPU seams: a dropped file judged as the CLI judges it and
// installed through the one loader; the pose kernel's clip table read as
// `sampleClip` reads a clip; the impostor atlas's framing and filter.
import { expect, test } from "vitest";
import { quat, vec3 } from "math";
import { previewRuntime } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { validateLoose } from "@packages/scene-assets/src/loose.ts";
import { farPoseBounds, sampleClip } from "@packages/scene-assets/src/pose.ts";
import type { SkinnedBundle } from "@packages/scene-assets/src/schema.ts";
import {
  SAMPLE_FLOATS,
  buildClipTable,
  clipFrames,
} from "@packages/battle-renderer/src/models/clipTable.ts";
import {
  boxFilter,
  impostorViews,
  IMPOSTOR_SPEC,
} from "@packages/battle-renderer/src/models/impostor.ts";
import { AUTHORITY, TOLERANCES, soldierGlb, tankGlb, testCatalog } from "./synthetic";

const context = { authority: AUTHORITY, tolerances: TOLERANCES, provenance: [] };
const empty = { skeletons: {}, appearances: {} };

test("a dropped GLB outside the catalog is judged, and its preview installs through the loader", async () => {
  const result = await validateLoose("drop.glb", tankGlb({ muzzleX: 5.9 }), empty, context, {
    type: "tank",
  });
  expect(result.unit).toBe("vehicle");
  const codes = result.appearance!.findings.map((f) => f.code);
  expect(codes).toEqual(expect.arrayContaining(["fit.vehicle_muzzle", "provenance.unlisted"]));
  expect(result.appearance!.bundle).toBeNull();
  const preview = result.appearance!.preview!;
  expect(preview.kind).toBe("articulated");
  const files = await previewRuntime(
    [{ name: "drop.glb", unit: result.unit, bundle: preview as never }],
    testCatalog().sides,
  );
  const installed = await new AppearanceLibrary(memoryFetch(files, "mem:/")).load("mem:/");
  const got = installed.appearances.get("drop.glb")!.bundle;
  expect(got.kind === "articulated" && got.nodes.map((n) => n.name)).toEqual(
    preview.kind === "articulated" && preview.nodes.map((n) => n.name),
  );
});

test("a dropped skinned body brings its own clips, installed under their id", async () => {
  const result = await validateLoose("man.glb", soldierGlb(), empty, context, {
    loops: ["idle", "walk"],
    yaw: 90,
  });
  expect(result.yaw).toBe(90);
  expect(result.clips!.preview!.clips.find((c) => c.name === "walk")!.loop).toBe(true);
  expect(result.clips!.preview!.clips.find((c) => c.name === "run")!.loop).toBe(false);
  const body = result.appearance!.preview as SkinnedBundle;
  const files = await previewRuntime(
    [{ name: "man.glb", unit: "soldier", bundle: body, clips: result.clips!.preview! }],
    testCatalog().sides,
  );
  const installed = await new AppearanceLibrary(memoryFetch(files, "mem:/")).load("mem:/");
  expect(installed.skeletons.get("adhoc")!.clips.length).toBe(7);
});

test("the clip table holds, frame for frame, what sampleClip reads", async () => {
  const result = await validateLoose("man.glb", soldierGlb(), empty, context, { loops: ["walk"] });
  const clips = result.clips!.preview!;
  const body = result.appearance!.preview as SkinnedBundle;
  const table = buildClipTable(clips, body.joints);
  const n = body.joints.length;
  for (const clip of clips.clips) {
    const entry = table.clips.get(clip.name)!;
    for (const phase of [0, 0.37, 0.5, 1]) {
      const frames = clipFrames({ f0: 0, f1: 0, w: 0 }, entry, phase);
      const cpu = sampleClip(clips, clip, body.joints, phase);
      for (let j = 0; j < n; j++) {
        const at = (f: number) => entry.offset + (f * n + j) * SAMPLE_FLOATS;
        const a = table.data.subarray(at(frames.f0), at(frames.f0) + 8);
        const b = table.data.subarray(at(frames.f1), at(frames.f1) + 8);
        const q = quat.slerp(
          quat.create(),
          [a[0], a[1], a[2], a[3]],
          [b[0], b[1], b[2], b[3]],
          frames.w,
        );
        // Both read the same snorm16 samples; they differ only in rounding.
        expect(Math.abs(quat.dot(q, cpu[j].r))).toBeCloseTo(1, 4);
        const t = vec3.lerp(vec3.create(), [a[4], a[5], a[6]], [b[4], b[5], b[6]], frames.w);
        expect(vec3.distance(t, cpu[j].t)).toBeLessThan(1e-5);
        expect(a[7]).toBeCloseTo(body.joints[j].bind.s[0], 6);
      }
    }
  }
});

test("a looping clip wraps its phase; a one-shot clip holds its ends", () => {
  const loop = {
    name: "l",
    offset: 0,
    frames: 31,
    duration: 1,
    rate: 30,
    loop: true,
    stride_m: null,
  };
  expect(clipFrames({ f0: 0, f1: 0, w: 0 }, loop, 1.25)).toEqual({ f0: 7, f1: 8, w: 0.5 });
  const once = { ...loop, loop: false };
  expect(clipFrames({ f0: 0, f1: 0, w: 0 }, once, 1.25)).toEqual({ f0: 30, f1: 30, w: 0 });
  expect(clipFrames({ f0: 0, f1: 0, w: 0 }, once, -1)).toEqual({ f0: 0, f1: 1, w: 0 });
});

test("impostor cells frame the far pose's sphere the same from every side", async () => {
  const result = await validateLoose("man.glb", soldierGlb(), empty, context);
  const body = result.appearance!.preview as SkinnedBundle;
  const bounds = farPoseBounds(body, result.clips!.preview!);
  // The far pose (idle) stands: its bounds are a standing man's, not every clip's.
  expect(bounds.max[2] - bounds.min[2]).toBeCloseTo(1.7, 1);
  const reach = (b: typeof bounds) => Math.max(b.max[0] - b.min[0], b.max[1] - b.min[1]);
  expect(reach(body.bounds)).toBeGreaterThan(reach(bounds) + 0.5); // the fall lies long
  const frame = impostorViews(IMPOSTOR_SPEC, bounds);
  expect(frame.views).toHaveLength(IMPOSTOR_SPEC.yaws * IMPOSTOR_SPEC.pitches.length);
  const corners = [0, 1, 2, 3, 4, 5, 6, 7].map((k) => [
    k & 1 ? bounds.max[0] : bounds.min[0],
    k & 2 ? bounds.max[1] : bounds.min[1],
    k & 4 ? bounds.max[2] : bounds.min[2],
    1,
  ]);
  for (const view of frame.views)
    for (const c of corners) {
      const m = view.viewProj;
      const x = m[0] * c[0] + m[4] * c[1] + m[8] * c[2] + m[12];
      const y = m[1] * c[0] + m[5] * c[1] + m[9] * c[2] + m[13];
      const z = m[2] * c[0] + m[6] * c[1] + m[10] * c[2] + m[14];
      expect(Math.abs(x)).toBeLessThanOrEqual(1 + 1e-6);
      expect(Math.abs(y)).toBeLessThanOrEqual(1 + 1e-6);
      expect(z).toBeGreaterThanOrEqual(-1e-6);
      expect(z).toBeLessThanOrEqual(1 + 1e-6);
    }
});

test("the atlas filter weights colour by coverage, so edges never darken", () => {
  // Two texels: opaque red and fully transparent black.
  const source = Uint8Array.from([255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 0]);
  expect(Array.from(boxFilter(source, 2, 2, 2))).toEqual([255, 0, 0, 128]);
});

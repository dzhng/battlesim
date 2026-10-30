// @vitest-environment node
import { expect, test } from "vitest";
import { Presentation } from "@wasm/renderer_browser";
import {
  ScenePresentation,
  loadPresentation,
  type PresentedFrame,
} from "@web/battle/sim/presentation";

function words(frame: PresentedFrame) {
  const { header, ...sections } = frame;
  return {
    header: Array.from(header),
    sections: Object.fromEntries(
      Object.entries(sections).map(([name, { count, u32 }]) => [
        name,
        { count, words: Array.from(u32) },
      ]),
    ),
  };
}

test("the shared browser module reacquires every UI stream after its linear memory grows", async () => {
  const { memory } = await loadPresentation();
  const scene = new ScenePresentation();
  try {
    scene.loadWorld({ sun_azimuth: 0.4, sun_elevation: 0.8 });
    const before = scene.frame(
      2.75,
      {
        eye: [12, -20, 30],
        target: [0, 0, 0],
        up: [0, 0, 1],
        fovY: 1,
        near: 0.1,
        width: 320,
        height: 180,
      },
      { selected: [], xray: false, ghosts: [] },
      0,
    );
    const expected = words(before);
    scene.noteAudio({ note: (feed) => expect(feed).toBeInstanceOf(Presentation) });

    memory.grow(1);
    expect(before.header.byteLength).toBe(0);
    const after = scene.lastFrame();

    expect(after.camera.f32.buffer).toBe(memory.buffer);
    expect(words(after)).toEqual(expected);
  } finally {
    scene.free();
  }
});

function articulatedBundle(): Uint8Array {
  const mesh = { positions: { $view: 0 } };
  const head = new TextEncoder().encode(
    JSON.stringify({
      content: {
        kind: "articulated",
        nodes: [
          {
            name: "hull",
            parent: -1,
            bind: { t: [0, 0, 0], r: [0, 0, 0, 1], s: [1, 1, 1] },
            tiers: [mesh, mesh, mesh, mesh],
          },
        ],
      },
      views: [["f32", 0, 9]],
    }),
  );
  const positions = new Float32Array([1, 0, 0, 0, 1, 0, 0, 0, 1]);
  const bytes = new Uint8Array(16 + head.length + positions.byteLength);
  const header = new DataView(bytes.buffer);
  [0x54414742, 4, head.length, positions.byteLength].forEach((v, i) =>
    header.setUint32(i * 4, v, true),
  );
  bytes.set(head, 16);
  bytes.set(new Uint8Array(positions.buffer), 16 + head.length);
  return bytes;
}

test("external model admission publishes canonical joints and keeps the prior catalog after failed replacement", async () => {
  await loadPresentation();
  const scene = new ScenePresentation();
  const detail = { lod_px: [80, 40, 20] as const, impostor_px: 4 };
  const camera = {
    eye: [12, -20, 30],
    target: [0, 0, 0],
    up: [0, 0, 1],
    fovY: 1,
    near: 0.1,
    width: 320,
    height: 180,
  };
  const ui = { selected: [], xray: false, ghosts: [] };
  try {
    scene.loadWorld({ sun_azimuth: 0.4, sun_elevation: 0.8 });
    const rows = scene.installLabAssets([{ name: "tool", bundle: articulatedBundle() }], detail);
    expect(rows.map((r) => r.name)).toContain("tool");
    scene.setLabModels([
      {
        appearance: "tool",
        x: 2,
        y: 3,
        z: 4,
        yaw: 0.5,
        tier: 0,
        pose: { kind: "posed", palette: new Float32Array([1, 0, 0, 5, 0, 1, 0, 6, 0, 0, 1, 7]) },
      },
    ]);
    const before = scene.frame(1, camera, ui, 0);
    expect(Array.from(before.instances.f32.slice(0, 4))).toEqual([2, 3, 4, 0.5]);
    const expectedInstances = Array.from(before.instances.f32);
    const palette = before.instances.f32[4] * 12;
    expect(Array.from(before.palettes.f32.slice(palette, palette + 12))).toEqual([
      1, 0, 0, 5, 0, 1, 0, 6, 0, 0, 1, 7,
    ]);
    expect(() =>
      scene.installLabAssets([{ name: "broken", bundle: new Uint8Array() }], detail),
    ).toThrow("not a bundle");
    const after = scene.frame(1, camera, ui, 0);
    expect(Array.from(after.instances.f32)).toEqual(expectedInstances);
    scene.setLabModels([]);
    expect(scene.frame(1, camera, ui, 0).instances.count).toBe(0);
  } finally {
    scene.free();
  }
});

// @vitest-environment node
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { expect, test } from "vitest";
import { Presentation } from "@wasm/renderer_browser";
import {
  ScenePresentation,
  loadPresentation,
  type PresentedFrame,
} from "@web/battle/sim/presentation";
import { fieldUnits, type RuntimeFiles } from "@web/battle/sim/fielding";
import { loadWasm } from "@web/battle/sim/module";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import { Counts } from "@packages/battle-renderer/src/records.gen";

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

const runtime = new URL("../../assets/runtime/", import.meta.url);

/** The page's runtime files, read from disk as the page fetches them. */
function diskRuntime(): RuntimeFiles {
  const catalog = JSON.parse(readFileSync(new URL("catalog.json", runtime), "utf8"));
  return {
    catalog,
    contents: async (hashes: readonly string[]) =>
      hashes.map((hash) =>
        gunzipSync(readFileSync(new URL(`${catalog.gzip[hash].hash}/bundle.bin`, runtime))),
      ),
  } as unknown as RuntimeFiles;
}

test("a presentation shows one fielded squad at rest with no battle, and clearing it takes its rows away", async () => {
  await loadPresentation();
  const wasm = await loadWasm();
  const set = await resolveCatalogSet("test");
  const rules = JSON.stringify(set.rules);
  const kind = "test_rifle";
  const fielded = await fieldUnits({
    rules,
    catalog: set.units.view,
    kinds: [kind],
    runtime: diskRuntime(),
  });
  const showground = new wasm.Showground(rules);
  const squad = JSON.parse(showground.squad(kind, 7)) as {
    soldiers: { id: number; slot: number; at: [number, number] }[];
    operators: (number | null)[];
  };
  showground.free();
  const scene = new ScenePresentation();
  const camera = {
    eye: [-8, -12, 6],
    target: [0, 0, 1],
    up: [0, 0, 1],
    fovY: 1,
    near: 0.1,
    width: 320,
    height: 180,
  };
  const ui = { selected: [], xray: false, ghosts: [] };
  const soldiersDrawn = () => scene.frame(1, camera, ui, 0).counts.u32[Counts.soldiers];
  try {
    const before = scene.canonicalAppearanceRows().length;
    scene.setShowcase(fielded, {
      kind,
      side: "blue",
      at: [0, 0, 0],
      yaw: 0,
      soldiers: squad.soldiers,
      operators: squad.operators,
    });
    // The unit's art is among the presentation's own rows: a scene built
    // on it loads that art.
    const drawn = Object.keys((fielded.units as { appearances: object }).appearances);
    const shown = scene.canonicalAppearanceRows().slice(before);
    expect(shown.length).toBeGreaterThan(0);
    for (const row of shown) expect(drawn).toContain(row.name);
    expect(soldiersDrawn()).toBe(squad.soldiers.length);
    scene.clearShowcase();
    expect(scene.canonicalAppearanceRows()).toHaveLength(before);
    expect(soldiersDrawn()).toBe(0);
  } finally {
    scene.free();
  }
});

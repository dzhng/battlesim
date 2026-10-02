// @vitest-environment node
import { beforeAll, expect, test } from "vitest";
import { readFileSync } from "node:fs";
import * as wasm from "@wasm/game_wasm.js";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { loadMap } from "@web/maps/node";

beforeAll(() =>
  wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }),
);

test("public queries preserve ground, surface and the first hit without a simulation world", () => {
  const source = new wasm.WorldView(loadMap("geometry").json, JSON.stringify(GAME_RULES));
  const publicWorld = new wasm.PublicWorld(source.public_queries());
  try {
    for (const [x, y] of [
      [0, 0],
      [40, 150],
      [100, 100],
      [180, 140],
      [200, 150],
      [260, 80],
      [340, 220],
      [400, 300],
      [-1, 20],
      [401, 150],
    ]) {
      expect(publicWorld.height_at(x, y)).toEqual(source.height_at(x, y));
      expect(publicWorld.surface_at(x, y)).toEqual(source.surface_at(x, y));
      for (const [dx, dy, dz] of [
        [0, 0, -1],
        [1, 0, -0.2],
        [-1, 0, -0.1],
        [0, 1, -0.2],
        [1, 1, 0],
      ]) {
        expect(publicWorld.raycast(x, y, 30, dx, dy, dz, 5000)).toEqual(
          source.raycast(x, y, 30, dx, dy, dz, 5000),
        );
      }
    }
  } finally {
    publicWorld.free();
    source.free();
  }
});

test("learned clearing changes public foliage exactly as the observed world would", () => {
  const map = loadMap("village");
  const source = new wasm.WorldView(map.json, JSON.stringify(GAME_RULES));
  const publicWorld = new wasm.PublicWorld(source.public_queries());
  try {
    const layout = JSON.parse(wasm.world_layout(JSON.stringify(GAME_RULES)));
    const props = source.props();
    const kind = layout.propKinds.indexOf(GAME_RULES.forests.tree);
    const offsets = Object.fromEntries(layout.propFields.map((f: string, i: number) => [f, i]));
    let row = 0;
    while (row < props.length && props[row + offsets.kind] !== kind) row += layout.propStride;
    expect(row).toBeLessThan(props.length);
    const cols = map.definition.size[0],
      cellM = 1;
    const tile =
      Math.floor(props[row + offsets.y] / 16) * Math.ceil(cols / 16) +
      Math.floor(props[row + offsets.x] / 16);
    const runs = Uint32Array.from([tile, 256 * 256]);
    expect(source.foliage_cleared(runs, cols, cellM)).not.toEqual(source.foliage());
    expect(publicWorld.foliage_cleared(runs, cols, cellM)).toEqual(
      source.foliage_cleared(runs, cols, cellM),
    );
    expect(publicWorld.foliage_cleared(new Uint32Array(), cols, cellM)).toEqual(source.foliage());
  } finally {
    publicWorld.free();
    source.free();
  }
});

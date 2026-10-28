import type { Renderer } from "@wasm/renderer_browser/renderer_browser.js";
import { backdrop_mesh, map_box } from "@wasm/presentation/presentation_wasm.js";
import type { BattleFixture } from "./oneRendererBattle";
import { gameLight } from "../gameLight";

/** Transport presentation tables unchanged; Rust owns admission and selection. */
export function loadSharedGround(renderer: Renderer, fixture: BattleFixture) {
  const ground = fixture.presentation.presentedGround();
  if (!ground) throw new Error("presentation did not resolve ground");
  const tables = ground.ground_terrain();
  const box = map_box(fixture.world.backdropTerrain ?? fixture.terrain);
  try {
    renderer.load_ground_world({
      params: tables.params(),
      nodes: tables.nodes(),
      plots: ground.ground_plots(true),
      surfaces: tables.surfaces(),
      surfaceIndex: tables.surface_index(),
      scars: tables.scars(),
      heights: ground.pages(),
      terrain: fixture.terrain,
      skirt: fixture.world.props,
      backdrop: box ? backdrop_mesh(box, gameLight.backdrop.reach_m) : new Float32Array(0),
      water: fixture.world.terrain.water ? ground.ground_water_mesh() : new Float32Array(0),
    });
  } finally {
    tables.free();
  }
  let params: Uint32Array = new Uint32Array(0);
  if (fixture.world.grass) {
    const kinds = JSON.parse(ground.grass_kinds()) as string[];
    const packed = renderer.pack_grass_shapes(kinds) as {
      shapes: number[];
      facts: unknown;
      maxBlades: number;
    };
    params = ground.grass_params_from_facts(packed.facts, 1);
    renderer.load_grass_world({
      params,
      props: ground.grass_props(),
      shapes: packed.shapes,
      maxBlades: packed.maxBlades,
      capacity: ground.grass_capacity(),
    });
  }
  const camera = new Float32Array(fixture.camera.camera.f32);
  const scarSnapshot = (region: Float64Array | undefined, clipped: boolean) => {
    const upload = ground.prepare_scars(region, clipped);
    try {
      return upload.snapshot();
    } finally {
      upload.free();
    }
  };
  return (refresh: boolean) => {
    if (refresh) ground.resend_scars();
    const window = fixture.world.grass ? ground.grass_window_record() : new Float64Array(0);
    if (window.length) ground.grass_frame_params(params, window);
    const bounds = fixture.world.grass
      ? renderer.prepare_grass(window, camera, params, ground.scars_changed())
      : new Float64Array(0);
    // Preparing grass can change regional cache admission. Read regions after it.
    const grass = bounds.length ? scarSnapshot(bounds, false) : null;
    const regions = ground.scar_regions();
    renderer.prepare_scars(grass, regions, (region: Float64Array | null, clipped: boolean) =>
      scarSnapshot(region ?? undefined, clipped),
    );
  };
}

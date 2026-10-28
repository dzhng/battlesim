import type { Renderer } from "@wasm/renderer_browser";
import { backdrop_mesh, map_box } from "@wasm/renderer_browser";
import type { BattleFixture } from "./oneRendererBattle";
import { gameLight } from "../gameLight";

/** Only external geometry crosses JavaScript; canonical ground data stays in Rust. */
export function loadSharedGround(renderer: Renderer, fixture: BattleFixture) {
  const ground = fixture.presentation.presentedGround();
  if (!ground) throw new Error("presentation did not resolve ground");
  const box = map_box(fixture.world.backdropTerrain ?? fixture.terrain);
  renderer.set_world_geometry(
    fixture.terrain,
    fixture.world.props,
    box ? backdrop_mesh(box, gameLight.backdrop.reach_m) : new Float32Array(0),
    fixture.world.terrain.water ? ground.ground_water_mesh() : new Float32Array(0),
  );
}

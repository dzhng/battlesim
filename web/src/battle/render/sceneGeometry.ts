/** External meshes cross the browser boundary; canonical ground tables do not. */
import { backdrop_mesh, map_box, type Renderer } from "@wasm/renderer_browser";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type { ScenePresentation } from "../sim/presentation";

export function installSceneGeometry(
  renderer: Renderer,
  presentation: ScenePresentation,
  world: WorldLayers,
  light: LightPresentation,
): void {
  const ground = presentation.presentedGround();
  if (!ground) throw new Error("presentation did not resolve ground");
  const terrain = world.terrain.mesh;
  const box = map_box(world.backdropTerrain ?? terrain);
  renderer.set_world_geometry(
    terrain,
    world.props,
    box ? backdrop_mesh(box, light.backdrop.reach_m) : new Float32Array(0),
    world.terrain.water ? ground.ground_water_mesh() : new Float32Array(0),
  );
}

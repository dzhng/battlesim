/** Rust constructs world layers; the browser only transports its inputs. */
import type { Renderer, Presentation } from "@wasm/renderer_browser";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import { EMPTY_MESH } from "@packages/battle-renderer/src/mesh";
import { presentationSettings } from "@web/presentationSettings";

export function installSceneGeometry(
  renderer: Pick<Renderer, "set_world_geometry">,
  presentation: Presentation,
  world: WorldLayers,
): void {
  const { triangles } = world.terrain;
  renderer.set_world_geometry(
    presentation,
    triangles?.positions ?? world.skirtPositions ?? EMPTY_MESH,
    triangles?.indices ?? new Uint32Array(0),
    triangles?.surfaces,
    triangles?.blocked ?? 0,
    world.skirt_and_props,
    world.terrain.water,
    presentationSettings.ground.backdrop_reach_m,
    triangles ? undefined : world.terrain.mesh,
    world.backdropTerrain,
  );
}

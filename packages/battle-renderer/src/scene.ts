/** Plain world and overlay inputs for the shared renderer. */
import type { Mesh } from "./mesh";
import type { ProxyKind } from "./proxies";
import type { TerrainSurface } from "./terrain/terrainSurface";
import type { StaticBundle } from "@packages/scene-assets/src/schema";
import type { GrassAppearances } from "./terrain/grassField";
/** A drawn proxy: world placement plus presentation tint. */
export interface SceneInstance {
  kind: ProxyKind;
  x: number;
  y: number;
  z: number;
  /** Heading of local +X, radians counter-clockwise from world +X. */
  yaw: number;
  color: readonly [number, number, number];
  highlight?: boolean;
}

/** Marks in world space. `opaque` and `translucent` are display-space
 *  overlay (contacts, and the labs' tracers, garrison and guidance marks):
 *  drawn after post, never fogged or lit; translucent triangles draw after
 *  everything opaque without writing depth. `painted` and `paintedMarching`
 *  are painted on the ground in the lit world: orders and selection,
 *  a truck's reach, the zone and the border, all shadowed,
 *  fogged, under the effects, glowing by their own emissive; the marching marks' normals carry (phase in cycles, cycles a
 *  second, amplitude) instead of a direction (`orderOverlay.ts`
 *  `travelChevrons`). */
export interface WorldMeshes {
  opaque: Mesh;
  translucent: Mesh;
  painted?: Mesh;
  paintedMarching?: Mesh;
}

/** The trees, hedgerows and forest-floor dressing's art: the appearances
 *  (`tree`, `hedgerow` and `dressing` bundles from the one loader) the
 *  scenery's rows name. Where each stands, and a felled tree's stump, are
 *  the presentation's. */
export interface WorldScenery {
  appearances: ReadonlyMap<string, StaticBundle>;
}

/** The static world in layers, each with its material and fog ground flag: the terrain is ground; the map's skirt (and the
 *  traversal view's prop boxes) are faces. The map's props draw as their
 *  appearances, fitted to their boxes by the scene's presentation (its
 *  static batches). The scenery's forest stands in the simulation's forests
 *  and draws their trunks; `null` draws none (a route without appearances). */
export interface WorldLayers {
  terrain: TerrainSurface;
  /** Geometry the visual backdrop surrounds when it differs from the terrain
   * drawn and received by shadows (for example disconnected remote patches).
   * Omitted: surround the full terrain. This never bounds terrain selection. */
  backdropTerrain?: Mesh;
  props: Mesh;
  scenery: WorldScenery | null;
  /** The grass kinds the biome grows (scenery "grass" appearances) by catalog
   *  name; `null` draws no grass (a route without appearances, or a view
   *  where the tint replaces the biome). */
  grass: GrassAppearances | null;
}

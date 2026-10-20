// Exact static world triangles, spatially selected by the shared Rust owner.
import { TerrainPartition } from "@wasm/presentation/presentation_wasm.js";
import type { SubmittedDraws } from "./submitted";
import type { Mesh } from "../mesh";
import { CAMERA_WORDS, CameraAt } from "../records.gen";
import { MeshSlot, type Drawable } from "./geometry";
import type { GpuRegistry } from "./registry";

export class SpatialMeshDraws {
  private partition: TerrainPartition | null = null;
  private visible: Uint32Array = new Uint32Array(0);
  private casters: Uint32Array[] = [];
  visited = 0;
  get vertices() {
    return this.mesh.vertices;
  }

  constructor(
    private readonly mesh: MeshSlot,
    registry: GpuRegistry,
  ) {
    registry.adopt(() => this.partition?.free());
  }

  set(vertices: Mesh) {
    // Spatial batching only: mesh resolution and visibility are unchanged.
    const next = new TerrainPartition(vertices);
    try {
      this.mesh.set(next.vertices() as Mesh);
    } catch (error) {
      next.free();
      throw error;
    }
    this.partition?.free();
    this.partition = next;
  }

  prepare(camera: Float32Array, casters: Float32Array, count: number) {
    const partition = this.partition;
    if (!partition) return;
    const projection = (words: Float32Array, offset = 0) =>
      words.subarray(offset + CameraAt.view_proj, offset + CameraAt.view_proj + 16);
    this.visible = partition.select(projection(camera));
    this.visited = partition.visited();
    this.casters.length = count;
    for (let i = 0; i < count; i++) {
      this.casters[i] = partition.select(projection(casters, i * CAMERA_WORDS));
      this.visited += partition.visited();
    }
  }

  draw(bound: Drawable, submitted?: SubmittedDraws, cascade?: number) {
    this.mesh.draw(bound, submitted, cascade === undefined ? this.visible : this.casters[cascade]);
  }
}

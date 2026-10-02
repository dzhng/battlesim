import {
  readWorldExports,
  type WorldExportSource,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";

/** Public static geometry; no navigation, units or live destruction. */
export interface PublicWorldData {
  layout: WorldLayout;
  exports: WorldExports;
  queries: string;
}
export function publicWorldOf(
  source: WorldExportSource & { public_queries(): string },
  layout: string,
): PublicWorldData {
  return {
    layout: JSON.parse(layout),
    exports: readWorldExports(source),
    queries: source.public_queries(),
  };
}
export function publicWorldBuffers(world: PublicWorldData): ArrayBuffer[] {
  return [
    world.exports.terrain.pageIds,
    world.exports.terrain.heights,
    ...Object.values(world.exports).filter((v): v is ArrayBufferView => ArrayBuffer.isView(v)),
  ].map((a) => a.buffer as ArrayBuffer);
}

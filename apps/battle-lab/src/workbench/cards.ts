import type { ImpostorAtlas } from "@packages/battle-renderer/src/models/impostor";

const toBase64 = (bytes: Uint8Array) => {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000)
    s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
};

export async function atlasPixels(bake: () => Promise<ImpostorAtlas>) {
  const atlas = await bake();
  return {
    hash: atlas.hash,
    width: atlas.width,
    height: atlas.height,
    albedo: toBase64(atlas.albedo),
    normal: toBase64(atlas.normal),
  };
}

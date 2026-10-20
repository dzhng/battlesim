// Workbench's offline card export uses the same baker as the diagnostic atlas.
import { CARD_SPEC, encodeCard } from "@packages/scene-assets/src/cards";
import { encodeBundle } from "@packages/scene-assets/src/codec";
import { sha256Hex } from "@packages/scene-assets/src/glb";
import { staticCardPose } from "@packages/scene-assets/src/propCards";
import type { BattleFrame } from "@packages/battle-renderer/src/scene";
import type { ImpostorAtlas } from "@packages/battle-renderer/src/models/impostor";
import type { LoadedModel } from "./sources";

export async function prepareCard(model: LoadedModel | null, frame?: BattleFrame) {
  const art = model?.installed.appearances.get(model.name);
  if (!model || !art || !staticCardPose(art) || !frame)
    throw new Error("selected appearance does not support a static card");
  const atlas = await frame.bakeImpostor(model.name, CARD_SPEC);
  const normal = new Uint8Array(atlas.normal);
  for (let i = 3; i < normal.length; i += 4) normal[i] = atlas.mask[i - 3];
  return Array.from(
    encodeCard({
      appearance: model.name,
      bundle: await sha256Hex(encodeBundle(art.bundle)),
      state: "default",
      spec: CARD_SPEC,
      width: atlas.width,
      height: atlas.height,
      center: atlas.center,
      radius: atlas.radius,
      albedo: atlas.albedo,
      normal,
      depth: atlas.depth,
    }),
  );
}

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

/** Parsed art transported once to presentation and GPU admission. */
import type { InstalledAppearances, InstalledAppearance } from "@packages/scene-assets/src/loader";
import { splitTextures } from "@packages/scene-assets/src/codec";
import { encodeTexture } from "@packages/scene-assets/src/texture";
import type { Bundle, SkeletonClips, StaticBundle } from "@packages/scene-assets/src/schema";

export interface LabAsset {
  name: string;
  /** Inflated scene-assets container; texture bytes belong to rendering. */
  bundle: Uint8Array;
  skeleton?: Uint8Array;
}

/** Authored facts that mesh BGAT deliberately does not own. */
export interface ModelPlacement {
  footprint: InstalledAppearance["footprint"];
  scenery: InstalledAppearance["scenery"];
  family: InstalledAppearance["regionalFamily"];
  paints: InstalledAppearance["paints"];
  wreck: InstalledAppearance["wreck"];
}

export interface ModelAsset extends LabAsset {
  placement?: ModelPlacement;
  textureIds: string[];
  textureBytes: Uint8Array[];
}

export function modelAsset(
  name: string,
  bundle: Exclude<Bundle, SkeletonClips>,
  textures = new Map<string, Uint8Array>(),
): ModelAsset {
  const split = splitTextures(bundle);
  return {
    name,
    bundle: split.content,
    textureIds: split.textures.map((texture) => texture.id),
    textureBytes: split.textures.map((texture) => {
      let bytes = textures.get(texture.id);
      if (!bytes) {
        bytes = encodeTexture(texture);
        textures.set(texture.id, bytes);
      }
      return bytes;
    }),
  };
}

/** Uses the existing scene-assets codec for fetched and generated art alike. */
export function modelAssetsOf(
  installed: InstalledAppearances | null,
  names: Iterable<string>,
  staticSources: readonly ReadonlyMap<string, StaticBundle>[] = [],
): ModelAsset[] {
  const skeletons = new Map<string, Uint8Array>();
  const textures = new Map<string, Uint8Array>();
  return [...new Set(names)].map((name) => {
    const entry = installed?.appearances.get(name);
    const bundle = entry?.bundle ?? staticSources.map((source) => source.get(name)).find(Boolean);
    if (!bundle) throw new Error(`appearance ${name} is not installed`);
    let skeleton: Uint8Array | undefined;
    if (bundle.kind === "skinned") {
      skeleton = skeletons.get(bundle.skeleton);
      if (!skeleton) {
        const clips = installed?.skeletons.get(bundle.skeleton);
        if (!clips) throw new Error(`lab skeleton ${bundle.skeleton} is not installed`);
        skeleton = splitTextures(clips).content;
        skeletons.set(bundle.skeleton, skeleton);
      }
    }
    return {
      ...modelAsset(name, bundle, textures),
      skeleton,
      placement: entry && {
        footprint: entry.footprint,
        scenery: entry.scenery,
        family: entry.regionalFamily,
        paints: entry.paints,
        wreck: entry.wreck,
      },
    };
  });
}

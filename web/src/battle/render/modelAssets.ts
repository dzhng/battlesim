/** Parsed art transported once to presentation and GPU admission. */
import type {
  ContentId,
  InstalledAppearances,
  InstalledAppearance,
} from "@packages/scene-assets/src/loader";
import { splitTextures } from "@packages/scene-assets/src/codec";
import { encodeTexture } from "@packages/scene-assets/src/texture";
import type { Bundle, SkeletonClips, StaticBundle } from "@packages/scene-assets/src/schema";
import { generatedContentId } from "./contentIdentity";

export interface LabAsset {
  name: string;
  /** Catalog identity used to retain presentation facts across deltas. */
  contentId: ContentId;
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

export interface ModelAsset {
  name: string;
  contentId: ContentId;
  placement?: ModelPlacement;
  /** Borrow encoded transport only when Rust lacks this content. */
  encode(): {
    bundle: Uint8Array;
    textureIds: string[];
    textureBytes: Uint8Array[];
    skeleton?: Uint8Array;
  };
}

export function modelAsset(
  name: string,
  bundle: Exclude<Bundle, SkeletonClips>,
  contentId: ContentId,
  textures = new Map<string, Uint8Array>(),
  skeleton?: () => Uint8Array,
): ModelAsset {
  return {
    name,
    contentId,
    // Parsed art is the durable browser source; encoded transport is borrowed
    // by admission, rather than retained beside both parsed and Rust copies.
    encode() {
      return {
        bundle: splitTextures(bundle).content,
        skeleton: skeleton?.(),
        textureIds: bundle.textures.map((texture) => texture.id),
        textureBytes: bundle.textures.map((texture) => {
          let bytes = textures.get(texture.id);
          if (!bytes) {
            bytes = encodeTexture(texture);
            textures.set(texture.id, bytes);
          }
          return bytes;
        }),
      };
    },
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
    const skeleton =
      bundle.kind === "skinned"
        ? () => {
            let bytes = skeletons.get(bundle.skeleton);
            if (!bytes) {
              const clips = installed?.skeletons.get(bundle.skeleton);
              if (!clips) throw new Error(`lab skeleton ${bundle.skeleton} is not installed`);
              bytes = splitTextures(clips).content;
              skeletons.set(bundle.skeleton, bytes);
            }
            return bytes;
          }
        : undefined;
    return {
      ...modelAsset(
        name,
        bundle,
        entry ? entry.contentId : generatedContentId(bundle),
        textures,
        skeleton,
      ),
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

/** Page-generated art identities, shared by proxy and tooling model paths. */
import { bytes_hash } from "@wasm/renderer_browser";
import { encodeBundle } from "@packages/scene-assets/src/codec";
import type { ContentId } from "@packages/scene-assets/src/loader";
import type { Bundle, SkeletonClips } from "@packages/scene-assets/src/schema";

const generated = new WeakMap<Exclude<Bundle, SkeletonClips>, ContentId>();

/** Hash a generated bundle once for the life of its bundle object. */
export function generatedContentId(bundle: Exclude<Bundle, SkeletonClips>): ContentId {
  let content = generated.get(bundle);
  if (!content) {
    content = { bundle: bytes_hash(encodeBundle(bundle)), skeleton: null };
    generated.set(bundle, content);
  }
  return content;
}

/** The identity pair has a stable key independent of JavaScript property order. */
export function contentIdKey(content: ContentId): string {
  return JSON.stringify([content.bundle, content.skeleton]);
}

/** Names route to content; array order and source-object identity do not matter. */
export function appearanceContentKey(
  entries: Iterable<readonly [string, { contentId: ContentId }]>,
): string {
  return JSON.stringify(
    [...entries]
      .map(([name, asset]) => [name, contentIdKey(asset.contentId)])
      .sort(([a], [b]) => a.localeCompare(b)),
  );
}

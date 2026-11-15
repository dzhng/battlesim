/** Page-generated art identities, shared by proxy and tooling model paths. */
import { bytes_hash } from "@wasm/renderer_browser";
import { encodeBundle } from "@packages/scene-assets/src/codec";
import type { ContentId } from "@packages/scene-assets/src/contentIdentity";
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

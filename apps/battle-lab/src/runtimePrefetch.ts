// Browser-cache warm-up shares the loader's content-addressed runtime files.
import {
  bundlePath,
  texturePath,
  templateLibraryPath,
  type RuntimeCatalog,
} from "@packages/scene-assets/src/schema";
import { appResources } from "./appResources";
let prefetching: Promise<void> | null = null;
let prefetchTimer: ReturnType<typeof setTimeout> | null = null;
let prefetchAllowed = false;
export let prefetchStatus: Record<string, unknown> = { state: "idle" };
let priority: () => readonly string[] = () => [];
/** Warm the browser's network/cache storage without decoding or uploading art.
 * The page cancels it as it leaves (`appResources.signal`): that is no failure,
 * where a fetch the network fails while the page stays is. */
export async function prefetchRuntimeAssets(
  baseUrl: string,
  order?: () => readonly string[],
): Promise<void> {
  if (order) priority = order;
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer);
    prefetchTimer = null;
  }
  if (prefetching) return prefetching;
  const { signal } = appResources;
  prefetching = (async () => {
    prefetchStatus = { state: "loading" };
    const response = await fetch(`${baseUrl}catalog.json`, { cache: "force-cache", signal });
    if (!response.ok) throw new Error(`appearance prefetch catalog: HTTP ${response.status}`);
    const catalog = (await response.json()) as RuntimeCatalog;
    const files = new Set<string>();
    const byName = new Map<string, string[]>();
    const add = (hash: string | undefined, path: (hash: string) => string) => {
      if (!hash) return;
      const transport = catalog.gzip?.[hash];
      if (transport) files.add(`${baseUrl}${path(transport.hash)}`);
    };
    for (const [name, entry] of Object.entries(catalog.appearances ?? {})) {
      const hashes = [
        entry.bundle,
        ...(catalog.textures?.[entry.bundle] ?? []),
        ...(entry.skeleton ? [catalog.skeletons[entry.skeleton]] : []),
      ];
      const textures = new Set(catalog.textures?.[entry.bundle]);
      // A hash with no transport record has no file to warm, here as below.
      byName.set(
        name,
        hashes.flatMap((hash) => {
          const transport = catalog.gzip?.[hash];
          if (!transport) return [];
          const path = textures.has(hash) ? texturePath : bundlePath;
          return [`${baseUrl}${path(transport.hash)}`];
        }),
      );
      add(entry.bundle, bundlePath);
    }
    for (const ids of Object.values(catalog.textures ?? {}))
      for (const id of ids) add(id, texturePath);
    for (const hash of Object.values(catalog.skeletons ?? {})) add(hash, bundlePath);
    if (catalog.templates) add(catalog.templates.library, templateLibraryPath);
    const urls = [...files.keys()];
    const cache =
      typeof caches === "undefined"
        ? null
        : await caches.open("battle-runtime-v1").catch(() => null);
    const claimed = new Set<string>();
    const take = () => {
      const ordered = [...priority().flatMap((name) => byName.get(name) ?? []), ...urls];
      const next = ordered.find((url) => !claimed.has(url));
      if (next) claimed.add(next);
      return next;
    };
    let failed = 0;
    const worker = async () => {
      for (;;) {
        const url = take();
        if (!url) return;
        try {
          if (cache && (await cache.match(url))) continue;
          const file = await fetch(url, { cache: "force-cache", signal });
          if (!file.ok) throw new Error(`HTTP ${file.status}`);
          if (cache) await cache.put(url, file.clone());
          await file.arrayBuffer();
        } catch (error) {
          if (signal.aborted) return;
          failed++;
          console.error("Background appearance prefetch failed.", { url, error });
        }
      }
    };
    await Promise.all(Array.from({ length: Math.min(6, urls.length) }, () => worker()));
    if (cache && failed === 0 && !signal.aborted) {
      const expected = new Set(
        urls.map((url) => new URL(url, globalThis.location?.href ?? "http://localhost/").href),
      );
      for (const request of await cache.keys()) {
        if (!expected.has(request.url)) await cache.delete(request);
      }
    }
    prefetchStatus = signal.aborted
      ? { state: "cancelled" }
      : { state: failed ? "partial" : "ready", files: urls.length, failed };
  })()
    .catch((error) => {
      if (signal.aborted) {
        prefetchStatus = { state: "cancelled" };
        return;
      }
      prefetchStatus = {
        state: "failed",
        error: error instanceof Error ? error.message : String(error),
      };
      console.error("Background appearance prefetch failed.", error);
    })
    .then(() => undefined);
  return prefetching;
}

export function scheduleRuntimePrefetch(baseUrl: string): void {
  if (!prefetchAllowed || prefetching || prefetchTimer !== null) return;
  prefetchStatus = { state: "scheduled" };
  // Let the first page settle before broadening the network/cache work beyond
  // the current view's required art.
  prefetchTimer = setTimeout(() => {
    prefetchTimer = null;
    void prefetchRuntimeAssets(baseUrl);
  }, 3_000);
}

/** Start warming unused art only after the first page has a stable graphics tier. */
export function allowRuntimePrefetch(): void {
  prefetchAllowed = true;
  scheduleRuntimePrefetch("/");
}

/** A re-bake changes the immutable file set; the next view warms it afresh. */
export function resetRuntimePrefetch(): void {
  prefetching = null;
  if (prefetchTimer !== null) clearTimeout(prefetchTimer);
  prefetchTimer = null;
  prefetchStatus = { state: "idle" };
  priority = () => [];
}

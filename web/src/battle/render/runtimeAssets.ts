/** Verified runtime transport; authored asset interpretation belongs to Rust. */
import type { Renderer } from "@wasm/renderer_browser";
import { gzipTransport, readGzip, type ReadRuntime } from "@packages/scene-assets/src/gzip";
import { bundlePath, texturePath, type RuntimeCatalog } from "@packages/scene-assets/src/schema";

/** One immutable catalog generation. Decoded transport payloads live only for
 * an admission batch; Rust retains the resident library through device recovery. */
export class RuntimeAssets {
  private readonly installed = new WeakMap<Pick<Renderer, "load_bundle">, Set<string>>();

  constructor(
    private readonly catalog: RuntimeCatalog,
    private readonly read: ReadRuntime,
  ) {}

  async install(renderer: Pick<Renderer, "load_bundle">, names: Iterable<string>) {
    let installed = this.installed.get(renderer);
    if (!installed) this.installed.set(renderer, (installed = new Set()));
    const bundles = new Map<string, Promise<Uint8Array>>();
    const textures = new Map<string, Promise<Uint8Array>>();
    const payload = (
      cache: Map<string, Promise<Uint8Array>>,
      kind: string,
      hash: string,
      path: (hash: string) => string,
    ) => {
      let bytes = cache.get(hash);
      if (!bytes) {
        bytes = readGzip(this.catalog, kind, hash, path, this.read);
        cache.set(hash, bytes);
      }
      return bytes;
    };
    for (const name of new Set(names)) {
      if (installed.has(name)) continue;
      const hash = this.catalog.appearances[name]?.bundle;
      if (!hash) throw new Error(`runtime catalog has no appearance ${name}`);
      const ids = this.catalog.textures?.[hash] ?? [];
      const [bundle, pixels] = await Promise.all([
        payload(bundles, "bundle", hash, bundlePath),
        Promise.all(ids.map((id) => payload(textures, "texture", id, texturePath))),
      ]);
      renderer.load_bundle(name, bundle, ids, pixels);
      installed.add(name);
    }
    const content = new Set([...bundles.keys(), ...textures.keys()]);
    return {
      bundleCount: bundles.size,
      textureCount: textures.size,
      /** Catalog wire weight; an HTTP-cache hit is not a measured download. */
      transportBytes: [...content].reduce(
        (bytes, hash) => bytes + gzipTransport(this.catalog, hash).bytes,
        0,
      ),
    };
  }
}

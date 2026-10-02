// The one appearance loader, for the workbench and the battle alike. It reads
// the runtime catalog and fetches, by content hash, every appearance that is
// not a kit, and the template art library. A kit (a building set's modules:
// tens of megabytes) is fetched when something that draws it asks. Every
// file's hash is verified, and a load or a request for kits installs one whole
// generation: a failure anywhere leaves the installed generation in place.

import type { Vec3 } from "math";
import { decodeBundle } from "./codec.ts";
import { lfsPointerOid, lfsPullCommand, sha256Hex } from "./glb.ts";
import {
  bundlePath,
  templateLibraryPath,
  type Bundle,
  type RuntimeCatalog,
  type SideTints,
  type SkeletonClips,
  type AppearanceUnit,
} from "./schema.ts";
import {
  bindModules,
  decodeTemplateLibrary,
  TemplateArtError,
  type BoundModule,
  type TemplateArtLibrary,
} from "./templateLibrary.ts";
import type { MountDraws } from "./units.ts";

/** Where the runtime directory lives in the repo, for LFS pull hints. */
export const RUNTIME_DIR = "assets/runtime";

export interface InstalledAppearance {
  unit: AppearanceUnit;
  scenery: string | null;
  /** The simulation box a static appearance is authored to (catalog `footprint_half_m`). */
  footprint: Vec3 | null;
  /** A vehicle's rig per mount name (catalog `mounts`); null when it declares none. */
  mounts: MountDraws | null;
  bundle: Exclude<Bundle, SkeletonClips>;
}

export interface InstalledAppearances {
  generation: number;
  sides: SideTints;
  skeletons: Map<string, SkeletonClips>;
  /** Every appearance of the catalog that is not a kit, and the kits asked
   *  for so far (`AppearanceLibrary.withKits`). */
  appearances: Map<string, InstalledAppearance>;
  /** Every kit the catalog names. One is among `appearances` only once
   *  something asked for it; until then nothing can draw it. */
  kits: ReadonlySet<string>;
  /** The template art library, when the catalog has one: city buildings are
   *  drawn by resolving their template against it (`templateLibrary.ts`). */
  templates?: InstalledTemplateArt;
}

/** The template art library as installed: its rows, and each of its modules
 *  bound to the installed kit bundle state that draws it, or to none while
 *  its kit is absent (parallel to `library.modules`). */
export interface InstalledTemplateArt {
  library: TemplateArtLibrary;
  modules: BoundModule[];
}

export type Fetch = (url: string) => Promise<{
  ok: boolean;
  status: number;
  arrayBuffer(): Promise<ArrayBuffer>;
  json(): Promise<unknown>;
}>;

/** A loaded catalog and where its files are served (ending in "/"). */
interface Source {
  baseUrl: string;
  catalog: RuntimeCatalog;
}

export class AppearanceLibrary {
  private readonly fetcher: Fetch;
  private current: InstalledAppearances | null = null;
  /** What the installed generation was loaded from: where its kits are. */
  private source: Source | null = null;
  /** Kits on their way, by bundle hash: two askers share one fetch. */
  private readonly arriving = new Map<string, Promise<InstalledAppearance>>();

  constructor(fetcher: Fetch = (url) => fetch(url)) {
    this.fetcher = fetcher;
  }

  get installed(): InstalledAppearances | null {
    return this.current;
  }

  /** Load the catalog under `baseUrl` (ending in "/"): every appearance but
   *  the kits, and the template art library. Installs only if all succeed. */
  async load(baseUrl: string): Promise<InstalledAppearances> {
    const response = await this.fetcher(`${baseUrl}catalog.json`);
    if (!response.ok) throw new Error(`appearance catalog: HTTP ${response.status}`);
    const catalog = (await response.json()) as RuntimeCatalog;
    const source = { baseUrl, catalog };
    const skeletons = new Map<string, SkeletonClips>();
    await Promise.all(
      Object.entries(catalog.skeletons ?? {}).map(async ([id, hash]) => {
        const bundle = await this.bundle(source, hash);
        if (bundle.kind !== "clips" || bundle.id !== id)
          throw new Error(`skeleton ${id}: bundle ${hash} is not its clips`);
        skeletons.set(id, bundle);
      }),
    );
    const appearances = await Promise.all(
      Object.entries(catalog.appearances ?? {})
        .filter(([, entry]) => entry.unit !== "kit")
        .map(async ([name]) => [name, await this.appearance(source, name, skeletons)] as const),
    );
    if (!catalog.sides) throw new Error("appearance catalog has no side tints; re-bake");
    const library = catalog.templates && (await this.templateLibrary(source, catalog.templates));
    return this.install(source, catalog.sides, skeletons, new Map(appearances), library);
  }

  /**
   * The installed generation with the kits `names` too. Those it lacks are
   * fetched and installed together as the next generation; when it has them
   * all it is returned as it is, so a kit is fetched once however many maps
   * draw from it. A kit that cannot be had is refused by name, and the
   * installed generation stays.
   */
  async withKits(names: Iterable<string>): Promise<InstalledAppearances> {
    const source = this.source;
    if (!source || !this.current)
      throw new Error("kits were asked for before a catalog was loaded");
    const held = this.current.appearances;
    const absent = [...new Set(names)].filter((name) => !held.has(name));
    if (absent.length === 0) return this.current;
    const kits = await Promise.all(
      absent.map(async (name) => [name, await this.kit(source, name)] as const),
    );
    // The catalog was loaded again meanwhile: these kits are the new one's to answer for.
    if (this.source !== source) return this.withKits(absent);
    // Onto whatever is installed by now: another request may have landed first.
    const { sides, skeletons, appearances, templates } = this.current;
    return this.install(
      source,
      sides,
      skeletons,
      new Map([...appearances, ...kits]),
      templates?.library,
    );
  }

  /** Install the next generation: `appearances` of `source`, and its template
   *  art library bound to the kits among them. A library that does not fit
   *  its kits is refused before anything is installed. */
  private install(
    source: Source,
    sides: SideTints,
    skeletons: Map<string, SkeletonClips>,
    appearances: Map<string, InstalledAppearance>,
    library: TemplateArtLibrary | undefined,
  ): InstalledAppearances {
    const modules =
      library &&
      bindModules(library, (name) => {
        const bundle = appearances.get(name)?.bundle;
        return bundle?.kind === "static" ? bundle : undefined;
      });
    const catalog = Object.entries(source.catalog.appearances ?? {});
    this.source = source;
    this.current = {
      generation: (this.current?.generation ?? 0) + 1,
      sides,
      skeletons,
      appearances,
      kits: new Set(catalog.filter(([, entry]) => entry.unit === "kit").map(([name]) => name)),
      ...(library && modules ? { templates: { library, modules } } : {}),
    };
    return this.current;
  }

  /** The file at `path`, which the catalog says has content hash `hash`. */
  private async verified(
    { baseUrl }: Source,
    what: string,
    hash: string,
    path: string,
  ): Promise<Uint8Array> {
    const res = await this.fetcher(`${baseUrl}${path}`);
    if (!res.ok) throw new Error(`${what} ${hash}: HTTP ${res.status}`);
    const bytes = new Uint8Array(await res.arrayBuffer());
    if (lfsPointerOid(bytes) !== null)
      throw new Error(
        `${what} ${hash} is a Git LFS pointer; run: ${lfsPullCommand(`${RUNTIME_DIR}/${path}`)}`,
      );
    const actual = await sha256Hex(bytes);
    if (actual !== hash) throw new Error(`${what} ${hash}: content hash is ${actual}`);
    return bytes;
  }

  private async bundle(source: Source, hash: string): Promise<Bundle> {
    return decodeBundle(await this.verified(source, "bundle", hash, bundlePath(hash)));
  }

  /** The catalog's appearance `name`: its bundle, held to what its entry says. */
  private async appearance(
    source: Source,
    name: string,
    skeletons: ReadonlyMap<string, SkeletonClips>,
  ): Promise<InstalledAppearance> {
    const entry = source.catalog.appearances[name];
    const bundle = await this.bundle(source, entry.bundle);
    if (bundle.kind === "clips" || bundle.kind !== entry.kind)
      throw new Error(`appearance ${name}: bundle is ${bundle.kind}, catalog says ${entry.kind}`);
    if (bundle.kind === "skinned") {
      const clips = skeletons.get(bundle.skeleton);
      if (!clips)
        throw new Error(`appearance ${name}: skeleton ${bundle.skeleton} is not in the catalog`);
      const same =
        clips.joints.length === bundle.joints.length &&
        clips.joints.every(
          (j, i) => j.name === bundle.joints[i].name && j.parent === bundle.joints[i].parent,
        );
      if (!same)
        throw new Error(`appearance ${name}: joints differ from skeleton ${bundle.skeleton}`);
    }
    return {
      unit: entry.unit,
      scenery: entry.scenery ?? null,
      footprint: entry.footprint_half_m ?? null,
      mounts: entry.mounts ?? null,
      bundle,
    };
  }

  /** The catalog's kit `name`, fetched once however many ask while it is on
   *  its way. Whatever stops it names the kit. */
  private kit(source: Source, name: string): Promise<InstalledAppearance> {
    const entry = source.catalog.appearances[name];
    if (entry?.unit !== "kit")
      return Promise.reject(new Error(`kit "${name}" is not in the appearance catalog`));
    let arriving = this.arriving.get(entry.bundle);
    if (!arriving) {
      arriving = this.appearance(source, name, this.current!.skeletons).catch((error: unknown) => {
        const why = error instanceof Error ? error.message : String(error);
        throw new Error(`kit "${name}": ${why}`, { cause: error });
      });
      const settled = () => this.arriving.delete(entry.bundle);
      this.arriving.set(entry.bundle, arriving);
      arriving.then(settled, settled);
    }
    return arriving;
  }

  /** The template art library the catalog names: the art it says it is, over
   *  the catalogues it says, packed against the kits the catalog has. Anything
   *  else fails the whole load, with no kit fetched to find it out. */
  private async templateLibrary(
    source: Source,
    named: NonNullable<RuntimeCatalog["templates"]>,
  ): Promise<TemplateArtLibrary> {
    const path = templateLibraryPath(named.library);
    const library = decodeTemplateLibrary(
      await this.verified(source, "template library", named.library, path),
    );
    if (library.art_hash !== named.art_hash || library.covers.join() !== named.covers.join())
      throw new Error(
        `template library ${named.library} is art ${library.art_hash} over catalogues ${library.covers.join(", ")}, not what the catalog names; re-bake`,
      );
    for (const kit of library.kits) {
      const entry = source.catalog.appearances[kit.appearance];
      if (entry?.unit !== "kit")
        throw new TemplateArtError(
          "kit.missing",
          `kit "${kit.appearance}" is not in the appearance catalog; re-bake`,
        );
      if (entry.bundle !== kit.bundle)
        throw new TemplateArtError(
          "kit.missing",
          `kit "${kit.appearance}" is bundle ${entry.bundle.slice(0, 12)}, but the library was packed against ${kit.bundle.slice(0, 12)}; re-bake`,
        );
    }
    return library;
  }
}

/** A fetch over runtime files held in memory, keyed by their path under `baseUrl`. */
export function memoryFetch(files: ReadonlyMap<string, Uint8Array>, baseUrl: string): Fetch {
  return async (url) => {
    const bytes = url.startsWith(baseUrl) ? files.get(url.slice(baseUrl.length)) : undefined;
    return {
      ok: !!bytes,
      status: bytes ? 200 : 404,
      arrayBuffer: async () => bytes!.slice().buffer,
      json: async () => JSON.parse(new TextDecoder().decode(bytes)),
    };
  };
}

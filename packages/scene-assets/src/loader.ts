// The one appearance loader, for the workbench and the battle alike. It reads
// the runtime catalog and fetches, by content hash, the appearances its load
// takes (`catalogLoadNames`: the scenery, and the art the page's units wear),
// the skeleton clips and the template art library. A later session's units
// widen it (`withUnits`). A kit (a building
// set's modules: tens of megabytes) and a regional look are fetched on
// request (`fetchedOnRequest`): when something that draws them asks. A
// texture is fetched once however many bundles name it. Every file's hash is
// verified (`gzip.ts`), and a load or a request installs one whole
// generation: a failure anywhere leaves the installed generation in place.

import type { Vec3 } from "math";
import {
  templateLibraryPath,
  type Bundle,
  type RuntimeCatalog,
  type SideTints,
  type SkeletonClips,
  type AppearanceUnit,
  type FactionLooks,
  type Texture,
} from "./schema.ts";
import {
  bindModules,
  decodeTemplateLibrary,
  TemplateArtError,
  type BoundModule,
  type TemplateArtLibrary,
} from "./templateLibrary.ts";
import type { MountDraws } from "./units.ts";
import {
  catalogLoadBytes,
  downloadBytes,
  onRequestLabel,
  readBundle,
  readGzip,
  readTexture,
  type ReadRuntime,
} from "./gzip.ts";
import {
  CATALOG_LOAD_MAX_BYTES,
  catalogLoadNames,
  fetchedOnRequest,
  KIT_BUNDLE_MAX_BYTES,
  MAP_DOWNLOAD_MAX_BYTES,
  onRequestOf,
} from "./schema.ts";

export interface InstalledAppearance {
  unit: AppearanceUnit;
  scenery: string | null;
  /** The simulation box a static appearance is authored to (catalog `footprint_half_m`). */
  footprint: Vec3 | null;
  /** A vehicle's rig per mount name (catalog `mounts`); null when it declares none. */
  mounts: MountDraws | null;
  /** The regional family it is a look of (catalog `regional_family`); null
   *  for one of every region. */
  regionalFamily: string | null;
  /** The sRGB tints its paintable surfaces take, one per body (catalog
   *  `paints`); null for an appearance drawn as authored. */
  paints: Vec3[] | null;
  /** A vehicle's own wreck appearance (catalog `wreck`); null for anything else. */
  wreck: string | null;
  /** A soldier's look on a faction's side (catalog `factions`); null for one look. */
  factions: FactionLooks | null;
  bundle: Exclude<Bundle, SkeletonClips>;
}

export interface InstalledAppearances {
  generation: number;
  sides: SideTints;
  skeletons: Map<string, SkeletonClips>;
  /** The appearances the catalog load took (`catalogLoadNames`), and those
   *  added since (`AppearanceLibrary.withUnits`, `withAppearances`). */
  appearances: Map<string, InstalledAppearance>;
  /** Every appearance the catalog fetches on request, by name: the regional
   *  family it is a look of, or null for a kit. One is among `appearances`
   *  only once something asked for it; until then nothing can draw it. */
  onRequest: ReadonlyMap<string, string | null>;
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

/** A loaded catalog, how its files are read, the unit art its load takes
 *  (`catalogLoadNames`'s `wearing`; undefined, every one), and its textures
 *  fetched so far, by address: two bundles naming one share its one fetch. */
interface Source {
  catalog: RuntimeCatalog;
  read: ReadRuntime;
  wearing: ReadonlySet<string> | undefined;
  textures: Map<string, Promise<Texture>>;
}

export class AppearanceLibrary {
  private readonly fetcher: Fetch;
  private current: InstalledAppearances | null = null;
  /** What the installed generation was loaded from: where its kits are. */
  private source: Source | null = null;
  /** Appearances on their way after the load, by bundle hash: two askers
   *  share one fetch. */
  private readonly arriving = new Map<string, Promise<InstalledAppearance>>();

  constructor(fetcher: Fetch = (url) => fetch(url)) {
    this.fetcher = fetcher;
  }

  get installed(): InstalledAppearances | null {
    return this.current;
  }

  /** Load the catalog under `baseUrl` (ending in "/") for a page whose units
   *  wear `wearing` (`UnitCatalog.appearances`; every unit's art when
   *  absent): the appearances its load takes (`catalogLoadNames`), the
   *  skeleton clips and the template art library. Installs only if all
   *  succeed. */
  async load(baseUrl: string, wearing?: ReadonlySet<string>): Promise<InstalledAppearances> {
    const response = await this.fetcher(`${baseUrl}catalog.json`);
    if (!response.ok) throw new Error(`appearance catalog: HTTP ${response.status}`);
    const catalog = (await response.json()) as RuntimeCatalog;
    admitLoad(catalog, wearing);
    const source: Source = {
      catalog,
      read: this.reader(baseUrl),
      wearing,
      textures: new Map(),
    };
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
      catalogLoadNames(catalog, wearing).map(
        async (name) => [name, await this.appearance(source, name, skeletons)] as const,
      ),
    );
    if (!catalog.sides) throw new Error("appearance catalog has no side tints; re-bake");
    const library = catalog.templates && (await this.templateLibrary(source, catalog.templates));
    return this.install(source, catalog.sides, skeletons, new Map(appearances), library);
  }

  /**
   * The installed generation with the art units wearing `wearing` draw too
   * (`UnitCatalog.appearances`): a page's next session runs other units than
   * its first (a lab after the game). The catalog load widens to take them,
   * held to `CATALOG_LOAD_MAX_BYTES`; what the installed generation lacks is
   * fetched, its textures the load already has are not, and when it lacks
   * nothing it is returned as it is.
   */
  async withUnits(wearing: ReadonlySet<string>): Promise<InstalledAppearances> {
    const source = this.source;
    if (!source || !this.current)
      throw new Error("unit art was asked for before a catalog was loaded");
    if (!source.wearing) return this.current;
    const wider = new Set([...source.wearing, ...wearing]);
    const held = this.current.appearances;
    const absent = catalogLoadNames(source.catalog, wider).filter((name) => !held.has(name));
    if (absent.length === 0) return this.current;
    admitLoad(source.catalog, wider);
    const { sides, skeletons } = this.current;
    const arrived = await Promise.all(
      absent.map(
        async (name) =>
          [
            name,
            await this.arrive(source, name, () => this.appearance(source, name, skeletons)),
          ] as const,
      ),
    );
    // The catalog was loaded again meanwhile: these are the new one's to answer for.
    if (this.source !== source) return this.withUnits(wearing);
    source.wearing = new Set([...(source.wearing ?? []), ...wearing]);
    const { appearances, templates } = this.current;
    return this.install(
      source,
      sides,
      skeletons,
      new Map([...appearances, ...arrived]),
      templates?.library,
    );
  }

  /**
   * The installed generation with the appearances fetched on request `names`
   * too (kits, regional looks). Those it lacks are fetched and installed
   * together as the next generation; when it has them all it is returned as
   * it is, so one is fetched once however many maps draw from it. What they
   * download together is held to `MAP_DOWNLOAD_MAX_BYTES` before any is
   * fetched. One that cannot be had is refused by name, and the installed
   * generation stays.
   */
  async withAppearances(names: Iterable<string>): Promise<InstalledAppearances> {
    const source = this.source;
    if (!source || !this.current)
      throw new Error("appearances were asked for before a catalog was loaded");
    const requested = [...new Set(names)];
    const bytes = downloadBytes(
      source.catalog,
      requested.filter((name) => {
        const entry = source.catalog.appearances[name];
        return entry && fetchedOnRequest(entry);
      }),
      source.wearing,
    );
    if (bytes > MAP_DOWNLOAD_MAX_BYTES)
      throw new Error(`map download ${bytes} bytes is over ${MAP_DOWNLOAD_MAX_BYTES}`);
    const held = this.current.appearances;
    const absent = requested.filter((name) => !held.has(name));
    if (absent.length === 0) return this.current;
    const arrived = await Promise.all(
      absent.map(async (name) => [name, await this.onRequest(source, name)] as const),
    );
    // The catalog was loaded again meanwhile: these are the new one's to answer for.
    if (this.source !== source) return this.withAppearances(requested);
    // Onto whatever is installed by now: another request may have landed first.
    const { sides, skeletons, appearances, templates } = this.current;
    return this.install(
      source,
      sides,
      skeletons,
      new Map([...appearances, ...arrived]),
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
    this.source = source;
    this.current = {
      generation: (this.current?.generation ?? 0) + 1,
      sides,
      skeletons,
      appearances,
      onRequest: onRequestOf(source.catalog),
      ...(library && modules ? { templates: { library, modules } } : {}),
    };
    return this.current;
  }

  /** Runtime files under `baseUrl`, by their path there. */
  private reader(baseUrl: string): ReadRuntime {
    return async (path) => {
      const res = await this.fetcher(`${baseUrl}${path}`);
      if (!res.ok) throw new Error(`${path}: HTTP ${res.status}`);
      return new Uint8Array(await res.arrayBuffer());
    };
  }

  /** The bundle `hash`, each of its textures fetched once for the source. */
  private bundle(source: Source, hash: string, onRequest = false): Promise<Bundle> {
    const texture = (id: string) => {
      let arriving = source.textures.get(id);
      if (!arriving) {
        arriving = readTexture(source.catalog, id, source.read);
        source.textures.set(id, arriving);
        // A failed fetch is not kept: the next request tries again.
        arriving.catch(() => source.textures.delete(id));
      }
      return arriving;
    };
    const max = onRequest ? KIT_BUNDLE_MAX_BYTES : Infinity;
    return readBundle(source.catalog, hash, source.read, texture, max);
  }

  /** The catalog's appearance `name`: its bundle, held to what its entry says. */
  private async appearance(
    source: Source,
    name: string,
    skeletons: ReadonlyMap<string, SkeletonClips>,
  ): Promise<InstalledAppearance> {
    const entry = source.catalog.appearances[name];
    const bundle = await this.bundle(source, entry.bundle, fetchedOnRequest(entry));
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
      regionalFamily: entry.regional_family ?? null,
      paints: entry.paints ?? null,
      wreck: entry.wreck ?? null,
      factions: entry.factions ?? null,
      bundle,
    };
  }

  /** The catalog's appearance `name` that is fetched on request, fetched once
   *  however many ask while it is on its way. Whatever stops it names it. */
  private onRequest(source: Source, name: string): Promise<InstalledAppearance> {
    const entry = source.catalog.appearances[name];
    const what = `${onRequestLabel(entry)} "${name}"`;
    if (!entry || !fetchedOnRequest(entry))
      return Promise.reject(new Error(`${what} is not in the appearance catalog`));
    return this.arrive(source, name, () =>
      this.appearance(source, name, this.current!.skeletons).catch((error: unknown) => {
        const why = error instanceof Error ? error.message : String(error);
        throw new Error(`${what}: ${why}`, { cause: error });
      }),
    );
  }

  /** The appearance `name` fetched by `fetch`, or the fetch of its bundle
   *  already on its way. */
  private arrive(
    source: Source,
    name: string,
    fetch: () => Promise<InstalledAppearance>,
  ): Promise<InstalledAppearance> {
    const bundle = source.catalog.appearances[name].bundle;
    let arriving = this.arriving.get(bundle);
    if (!arriving) {
      arriving = fetch();
      const settled = () => this.arriving.delete(bundle);
      this.arriving.set(bundle, arriving);
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
    const library = decodeTemplateLibrary(
      await readGzip(
        source.catalog,
        "template library",
        named.library,
        templateLibraryPath,
        source.read,
      ),
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

/** Refuse a catalog load of the art units wearing `wearing` draw that is
 *  over `CATALOG_LOAD_MAX_BYTES`, before anything of it is fetched. */
function admitLoad(catalog: RuntimeCatalog, wearing: ReadonlySet<string> | undefined): void {
  const bytes = catalogLoadBytes(catalog, wearing);
  if (bytes > CATALOG_LOAD_MAX_BYTES)
    throw new Error(`catalog load ${bytes} bytes is over ${CATALOG_LOAD_MAX_BYTES}`);
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

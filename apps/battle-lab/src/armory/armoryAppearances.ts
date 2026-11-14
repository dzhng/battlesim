// The Armory's own appearances: the yard and the one unit on show. Browsing
// every unit through the page's shared library (`gameAppearances.ts`) would
// keep each one decoded for the page's life, so each selection builds a fresh
// `AppearanceLibrary` and the previous one goes with its unit. The files the
// newest selection read are kept, so the yard and the catalog download once
// however many units are browsed. Only the newest selection is answered, and
// a failure goes to the caller, never to `appResources`.
import { countedFetch } from "@web/downloads";
import { AppearanceLibrary, type Fetch } from "@packages/scene-assets/src/loader";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import type { Faction, UnitCatalog } from "@packages/scene-assets/src/units";
import type { StageArt } from "@web/battle/render/showcaseStage";

/** The runtime files, served at the site root (Vite's publicDir). */
const BASE_URL = "/";
/** The runtime catalog, as the loader fetches it. */
const CATALOG_URL = `${BASE_URL}catalog.json`;

/** The art one selection on show asks for. */
export interface ArtSelection {
  /** The unit type on show; null for the yard alone. */
  kind: string | null;
  faction: Faction;
  /** The yard's scenery appearances. */
  scenery: ReadonlySet<string>;
}

/** A file as it arrived, held for the next selection. */
interface Kept {
  ok: boolean;
  status: number;
  bytes: Uint8Array;
}

const superseded = () =>
  new DOMException("a newer Armory selection replaced this one", "AbortError");

export class ArmoryAppearances {
  /** The newest selection's files by URL, arrived or on their way. */
  private kept = new Map<string, Promise<Kept>>();
  /** The newest selection; a call that is no longer it is superseded. */
  private newest = 0;
  /** What the newest answered selection read: its files and their bytes. */
  private read = { files: 0, bytes: 0 };

  constructor(
    private readonly units: UnitCatalog,
    private readonly fetcher: Fetch = countedFetch,
  ) {}

  /** The runtime catalog, read through the kept files, so the next
   *  selection's load reuses its bytes. A failed read is not kept. */
  async catalog(): Promise<RuntimeCatalog> {
    let file = this.kept.get(CATALOG_URL);
    if (!file) {
      file = this.download(CATALOG_URL);
      this.kept.set(CATALOG_URL, file);
    }
    const forget = () => this.kept.get(CATALOG_URL) === file && this.kept.delete(CATALOG_URL);
    const { ok, status, bytes } = await file.catch((error: unknown) => {
      forget();
      throw error;
    });
    if (!ok) {
      forget();
      throw new Error(`runtime catalog: HTTP ${status}`);
    }
    return JSON.parse(new TextDecoder().decode(bytes)) as RuntimeCatalog;
  }

  /** The yard with `selection`'s unit, on a library of its own: what it
   *  draws, and the files it was loaded from (what Rust fields the unit
   *  from). Resolves for the newest call only; an earlier one rejects with
   *  AbortError. */
  async show(selection: ArtSelection): Promise<StageArt> {
    const call = ++this.newest;
    const read = new Map<string, Promise<Kept>>();
    const library = new AppearanceLibrary((url) => this.fetch(call, url, read));
    const wearing =
      selection.kind === null ? new Set<string>() : this.units.appearancesFor([selection.kind]);
    const appearances = await library
      .load(BASE_URL, wearing, new Set([selection.faction]), selection.scenery)
      // Superseded, the call answers AbortError however its load ended.
      .finally(() => {
        if (call !== this.newest) throw superseded();
      });
    this.kept = read;
    const files = await Promise.all(read.values());
    this.read = { files: files.length, bytes: files.reduce((n, f) => n + f.bytes.length, 0) };
    return { appearances, runtime: library };
  }

  /** The files the newest answered selection read, and their bytes: the
   *  yard's, the catalog's and its unit's (the load line, S5). */
  held(): { files: number; bytes: number } {
    return this.read;
  }

  /** Supersede every call and let go of the kept files. */
  dispose() {
    this.newest++;
    this.kept = new Map();
  }

  /** `url` for selection `call`, from the kept files when they have it. */
  private fetch(call: number, url: string, read: Map<string, Promise<Kept>>) {
    if (call !== this.newest) return Promise.reject(superseded());
    const file = read.get(url) ?? this.kept.get(url) ?? this.download(url);
    read.set(url, file);
    // A failed read is not kept: the loader asks again for one the network dropped.
    file.catch(() => read.get(url) === file && read.delete(url));
    return file.then(({ ok, status, bytes }) => ({
      ok,
      status,
      arrayBuffer: async () => bytes.slice().buffer,
    }));
  }

  /** `url` as it arrives, its body read only when it is a file. */
  private download(url: string): Promise<Kept> {
    return this.fetcher(url).then(async (response) => ({
      ok: response.ok,
      status: response.status,
      bytes: response.ok ? new Uint8Array(await response.arrayBuffer()) : new Uint8Array(),
    }));
  }
}

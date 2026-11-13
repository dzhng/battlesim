// The Armory's own appearances: the yard and the one unit on show. Browsing
// every unit through the page's shared library (`gameAppearances.ts`) would
// keep each one decoded for the page's life, so each selection builds a fresh
// `AppearanceLibrary` and the previous one goes with its unit. The files the
// newest selection read are kept, so the yard and the catalog download once
// however many units are browsed. Only the newest selection is answered, and
// a failure goes to the caller, never to `appResources`.
import { countedFetch } from "@web/downloads";
import {
  AppearanceLibrary,
  type Fetch,
  type InstalledAppearances,
} from "@packages/scene-assets/src/loader";
import type { Faction, UnitCatalog } from "@packages/scene-assets/src/units";
import type { RuntimeFiles } from "../presentedFrame";

/** The runtime files, served at the site root (Vite's publicDir). */
const BASE_URL = "/";

/** One selection on show. */
export interface ArmorySelection {
  /** The unit type on show; null for the yard alone. */
  kind: string | null;
  faction: Faction;
  /** The yard's scenery appearances. */
  scenery: ReadonlySet<string>;
}

/** What a selection draws, and the files it was loaded from: what Rust
 *  resolves the unit from (`fieldedBattle`). */
export interface ArmoryArt {
  appearances: InstalledAppearances;
  runtime: RuntimeFiles;
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

  constructor(
    private readonly units: UnitCatalog,
    private readonly fetcher: Fetch = countedFetch,
  ) {}

  /** The yard with `selection`'s unit, on a library of its own. Resolves for
   *  the newest call only; an earlier one rejects with AbortError. */
  async show(selection: ArmorySelection): Promise<ArmoryArt> {
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
    return { appearances, runtime: library };
  }

  /** Supersede every call and let go of the kept files. */
  dispose() {
    this.newest++;
    this.kept = new Map();
  }

  /** `url` for selection `call`, from the kept files when they have it. */
  private fetch(call: number, url: string, read: Map<string, Promise<Kept>>) {
    if (call !== this.newest) return Promise.reject(superseded());
    const file =
      read.get(url) ??
      this.kept.get(url) ??
      this.fetcher(url).then(async (response) => ({
        ok: response.ok,
        status: response.status,
        bytes: response.ok ? new Uint8Array(await response.arrayBuffer()) : new Uint8Array(),
      }));
    read.set(url, file);
    return file.then(({ ok, status, bytes }) => ({
      ok,
      status,
      arrayBuffer: async () => bytes.slice().buffer,
      json: async () => JSON.parse(new TextDecoder().decode(bytes)) as unknown,
    }));
  }
}

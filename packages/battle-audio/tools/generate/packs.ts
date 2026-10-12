import { createHash } from "node:crypto";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { packUrl, voicePack, voiceUrl, type VoiceCatalog } from "../../src/voices/voiceCatalog";
import { LOCK, readLock, verdictOf, writeAtomic, type TakeRecord } from "./lock";

/**
 * Whether a take belongs in its set's pack: judged ok on the bytes served.
 * Before the user's review, `unreviewed` also admits a take nobody has judged
 * yet whose transcript check passed; a take judged bad, or flagged, never.
 */
export function packAdmits(record: TakeRecord, unreviewed: boolean): boolean {
  const verdict = verdictOf(record);
  return verdict === "ok" || (unreviewed && verdict === null && !record.match.flagged);
}

/**
 * `generate pack`: writes each set's `pack.json` (all sets, or `sets`) from the
 * takes it admits whose served file holds the bytes the lock names. A set that
 * admits none has no pack, so the battle leaves it silent.
 */
export function writePacks(options: {
  root: string;
  catalog: VoiceCatalog;
  sets?: readonly string[];
  unreviewed: boolean;
  log: (line: string) => void;
}): { packs: number; takes: number } {
  const { root, catalog, unreviewed, log } = options;
  const lock = readLock(join(root, LOCK));
  const served = (url: string) => join(root, "assets/runtime", url);
  let packs = 0;
  let takes = 0;
  for (const set of options.sets ?? Object.keys(catalog.sets).sort()) {
    if (!catalog.sets[set]) throw new Error(`unknown set ${set}`);
    const pack = voicePack(catalog, set, (id, take) => {
      const record = lock.takes[id];
      if (!record || !packAdmits(record, unreviewed)) return null;
      const path = served(voiceUrl(take, record.served_sha256));
      if (!existsSync(path)) return null;
      const sha256 = createHash("sha256").update(readFileSync(path)).digest("hex");
      if (sha256 !== record.served_sha256) {
        log(`CHANGED ${id}`);
        return null;
      }
      return { sha256, duration_s: record.duration_s };
    });
    const path = served(packUrl(set));
    if (!pack.takes.length) {
      if (existsSync(path)) {
        rmSync(path);
        log(`EMPTY ${set} (pack removed)`);
      }
      continue;
    }
    writeAtomic(path, `${JSON.stringify(pack, null, 2)}\n`);
    packs++;
    takes += pack.takes.length;
    log(`PACK ${set} takes=${pack.takes.length}`);
  }
  return { packs, takes };
}

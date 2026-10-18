import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import {
  packUrl,
  takeId,
  voicePack,
  voiceUrl,
  type VoiceCatalog,
  type VoicePack,
} from "../../src/voices/voiceCatalog";
import { LOCK, readLock, verdictOf, writeAtomic, type TakeRecord, type VoiceLock } from "./lock";

const VOICES = "assets/runtime/audio/voices";
const served = (root: string, url: string) => join(root, "assets/runtime", url);
const sha256Of = (path: string) => createHash("sha256").update(readFileSync(path)).digest("hex");

/** The voice voices.json names now for one of a set's voices; null while it waits for one. */
const currentVoice = (catalog: VoiceCatalog, set: string, voice: string) =>
  catalog.sets[set].voices.find(({ id }) => id === voice)?.voice_id ?? null;

/**
 * Whether a take belongs in its set's pack: spoken by the voice voices.json
 * names now (a replaced voice's takes leave with it, even while regeneration
 * is unfinished) and judged ok on the bytes served. Before the user's review,
 * `unreviewed` also admits a take nobody has judged yet whose transcript check
 * passed; a take judged bad, or flagged, never.
 */
export function packAdmits(
  record: TakeRecord,
  voiceId: string | null,
  unreviewed: boolean,
): boolean {
  if (voiceId === null || record.provenance.voice_id !== voiceId) return false;
  const verdict = verdictOf(record);
  return verdict === "ok" || (unreviewed && verdict === null && !record.match.flagged);
}

/** A set's pack as the lock and the served files make it now. */
function admittedPack(
  root: string,
  catalog: VoiceCatalog,
  lock: VoiceLock,
  set: string,
  unreviewed: boolean,
  log: (line: string) => void,
): VoicePack {
  return voicePack(catalog, set, (id, take) => {
    const record = lock.takes[id];
    if (!record || !packAdmits(record, currentVoice(catalog, set, take.voice), unreviewed))
      return null;
    const path = served(root, voiceUrl(take, record.served_sha256));
    if (!existsSync(path)) return null;
    const sha256 = sha256Of(path);
    if (sha256 !== record.served_sha256) {
      log(`CHANGED ${id}`);
      return null;
    }
    return { sha256, duration_s: record.duration_s };
  });
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
  let packs = 0;
  let takes = 0;
  for (const set of options.sets ?? Object.keys(catalog.sets).sort()) {
    if (!catalog.sets[set]) throw new Error(`unknown set ${set}`);
    const pack = admittedPack(root, catalog, lock, set, unreviewed, log);
    const path = served(root, packUrl(set));
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

/**
 * Proves the served packs: every packed take is in the lock, spoken by the
 * voice voices.json names now, judged ok on the bytes its URL names, and its
 * file holds those bytes; each pack is the one
 * `generate pack` would write now; and no pack names a set voices.json lacks.
 */
export function checkPacks(root: string, catalog: VoiceCatalog, lock: VoiceLock): string[] {
  const problems: string[] = [];
  const folders = existsSync(join(root, VOICES)) ? readdirSync(join(root, VOICES)) : [];
  const sets = new Set([...Object.keys(catalog.sets), ...folders]);
  for (const set of [...sets].sort()) {
    const path = served(root, packUrl(set));
    const onDisk = existsSync(path) ? readFileSync(path, "utf8") : null;
    if (!catalog.sets[set]) {
      if (onDisk !== null) problems.push(`pack ${set}: set not in voices.json`);
      continue;
    }
    const pack: VoicePack | null = onDisk === null ? null : JSON.parse(onDisk);
    for (const take of pack?.takes ?? []) {
      const where = { set, ...take };
      const id = takeId(where);
      const record = lock.takes[id];
      if (!record) problems.push(`pack ${set}: ${id} is not in the lock`);
      else if (take.url !== voiceUrl(where, record.served_sha256))
        problems.push(`pack ${set}: ${id} names other bytes than the lock`);
      else if (record.provenance.voice_id !== currentVoice(catalog, set, take.voice))
        problems.push(`pack ${set}: ${id} was spoken by another voice than voices.json names`);
      else if (verdictOf(record) !== "ok")
        problems.push(`pack ${set}: ${id} is not judged ok on its bytes`);
      else if (
        !existsSync(served(root, take.url)) ||
        sha256Of(served(root, take.url)) !== record.served_sha256
      )
        problems.push(`pack ${set}: ${id} served file differs`);
    }
    const expected = admittedPack(root, catalog, lock, set, false, () => {});
    const current = expected.takes.length ? `${JSON.stringify(expected, null, 2)}\n` : null;
    if (current !== onDisk) problems.push(`pack ${set}: stale; run \`generate pack\``);
  }
  return problems;
}

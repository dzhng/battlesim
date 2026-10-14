import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import type { Generated } from "../../src/catalog";
import type { Match } from "./transcript";

/** A generated ambience under a voice profile, such as a tank's engine. */
export interface BedRecord {
  /** Raw master, relative to the repo root. */
  raw: string;
  sha256: string;
  provenance: Generated;
}
/** One take, from the line it speaks to the served bytes and what was heard in them. */
export interface TakeRecord {
  text: string;
  generation_key: string;
  /** Raw master, relative to the repo root. */
  raw: string;
  raw_sha256: string;
  processing_key: string;
  /** Served bytes; `voiceUrl` builds their path from this hash. */
  served_sha256: string;
  duration_s: number;
  /** Speech-to-text of the raw master. */
  transcript: string;
  match: Match;
  provenance: Generated;
  /** The reviewer's judgement of the speech: bound to the raw master it was spoken as,
   *  so a re-mix onto another bed keeps it and a new recording is judged again. */
  verdict?: { value: "ok" | "bad"; raw_sha256: string };
}
/** One Voice Design preview, kept as a raw master because its generated id is temporary. */
export interface CandidateRecord {
  /** Valid only for a while after the design; `generate pick` saves it as a permanent voice. */
  generated_voice_id: string;
  /** Raw master, relative to the repo root. */
  raw: string;
  raw_sha256: string;
  /** As Voice Design reported it. */
  duration_s: number;
  /** Speech-to-text of the raw master; null until transcribed. */
  transcript: string | null;
  match: Match | null;
}
/** A voice's design: the candidates one description and audition text produced. */
export interface DesignRecord {
  description: string;
  /** The role's audition text every candidate speaks. */
  text: string;
  model: string;
  retake: number;
  /** Hash of the four above: a design is bought again only when one changes. */
  design_key: string;
  date: string;
  candidates: CandidateRecord[];
  /** The candidate saved as the voice (1-based), and the permanent id it was saved as. */
  picked?: { candidate: number; voice_id: string; date: string };
}
/** `fixtures/voices.lock.json`: written only by the generation tool. */
export interface VoiceLock {
  beds: Record<string, BedRecord>;
  /** Voice designs by `<set>/<voice>`. */
  designs: Record<string, DesignRecord>;
  takes: Record<string, TakeRecord>;
}

export const LOCK = "fixtures/voices.lock.json";

export function readLock(path: string): VoiceLock {
  if (!existsSync(path)) return { beds: {}, designs: {}, takes: {} };
  const lock = JSON.parse(readFileSync(path, "utf8")) as Partial<VoiceLock>;
  return { beds: lock.beds ?? {}, designs: lock.designs ?? {}, takes: lock.takes ?? {} };
}

/** A take's standing verdict: one given to another recording no longer holds. */
export function verdictOf(record: TakeRecord): "ok" | "bad" | null {
  return record.verdict?.raw_sha256 === record.raw_sha256 ? record.verdict.value : null;
}

/**
 * `generate verdict`: judges takes named `<take>@<served sha256>`, the bytes the
 * reviewer heard. A take whose bytes changed since, or that the lock lacks, is
 * refused by name; the others are recorded against the raw master those bytes were
 * made from. True when every one was recorded.
 */
export function recordVerdicts(
  path: string,
  value: string,
  judged: readonly string[],
  log: (line: string) => void,
): boolean {
  if (value !== "ok" && value !== "bad") throw new Error(`verdict ${value}: expected ok or bad`);
  const lock = readLock(path);
  let recorded = 0;
  for (const entry of judged) {
    const at = entry.lastIndexOf("@");
    const id = entry.slice(0, at);
    const sha256 = entry.slice(at + 1);
    const record = at > 0 ? lock.takes[id] : undefined;
    if (!record) log(`UNKNOWN ${entry}`);
    else if (record.served_sha256 !== sha256) log(`CHANGED ${id}`);
    else {
      record.verdict = { value, raw_sha256: record.raw_sha256 };
      recorded++;
      log(`VERDICT ${id} ${value}`);
    }
  }
  if (recorded) writeLock(path, lock);
  return recorded === judged.length;
}

const sorted = <T>(record: Record<string, T>) =>
  Object.fromEntries(Object.entries(record).sort(([a], [b]) => (a < b ? -1 : 1)));

export function writeLock(path: string, lock: VoiceLock): void {
  writeAtomic(
    path,
    `${JSON.stringify(
      { beds: sorted(lock.beds), designs: sorted(lock.designs), takes: sorted(lock.takes) },
      null,
      2,
    )}\n`,
  );
}

/** A temporary file in the same folder, then a rename: a kill never leaves half a file. */
export function writeAtomic(path: string, data: Uint8Array | string): void {
  mkdirSync(dirname(path), { recursive: true });
  const temporary = join(dirname(path), `.${basename(path)}.${process.pid}.tmp`);
  writeFileSync(temporary, data);
  renameSync(temporary, path);
}

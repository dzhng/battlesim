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
  /** The reviewer's judgement, bound to the served bytes it judged. */
  verdict?: { value: "ok" | "bad"; sha256: string };
}
/** `fixtures/voices.lock.json`: written only by the generation tool. */
export interface VoiceLock {
  beds: Record<string, BedRecord>;
  takes: Record<string, TakeRecord>;
}

export const LOCK = "fixtures/voices.lock.json";

export function readLock(path: string): VoiceLock {
  if (!existsSync(path)) return { beds: {}, takes: {} };
  const lock = JSON.parse(readFileSync(path, "utf8")) as Partial<VoiceLock>;
  return { beds: lock.beds ?? {}, takes: lock.takes ?? {} };
}

const sorted = <T>(record: Record<string, T>) =>
  Object.fromEntries(Object.entries(record).sort(([a], [b]) => (a < b ? -1 : 1)));

export function writeLock(path: string, lock: VoiceLock): void {
  writeAtomic(
    path,
    `${JSON.stringify({ beds: sorted(lock.beds), takes: sorted(lock.takes) }, null, 2)}\n`,
  );
}

/** A temporary file in the same folder, then a rename: a kill never leaves half a file. */
export function writeAtomic(path: string, data: Uint8Array | string): void {
  mkdirSync(dirname(path), { recursive: true });
  const temporary = join(dirname(path), `.${basename(path)}.${process.pid}.tmp`);
  writeFileSync(temporary, data);
  renameSync(temporary, path);
}

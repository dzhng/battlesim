import { existsSync, mkdirSync, readFileSync, renameSync } from "node:fs";
import { dirname, join } from "node:path";
import { FixturePublication } from "../../../../apps/fixture-publication/publication";
import type { Intensity, Trigger, VoiceCatalog } from "../../src/voices/voiceCatalog";
import { processingKey, sha256 } from "./cacheKey";
import type { ElevenLabsClient } from "./elevenlabs";
import {
  LOCK,
  readLock,
  writeAtomic,
  writeLock,
  type CandidateRecord,
  type DesignRecord,
  type VoiceLock,
} from "./lock";
import { LIMITS, Pool } from "./pool";
import { matches, words } from "./transcript";
import { bedResolver, type Processor, type VoiceProfiles } from "./voices";

export const DESIGN_MODEL = "eleven_ttv_v3";
/** Voice Design refuses a text shorter than this. */
const MIN_AUDITION = 100;
/**
 * The audition every voice of a role speaks, from that role's own script: steady
 * orders, then stressed calls, then the desperate end. One text per role keeps a
 * role's candidates comparable, and it sounds like the game.
 */
const AUDITION: readonly [Trigger, Intensity][] = [
  ["move", "steady"],
  ["attack", "steady"],
  ["contact_infantry", "steady"],
  ["under_fire", "stressed"],
  ["suppressed", "stressed"],
  ["man_down", "stressed"],
  ["destroyed", "stressed"],
];
const RAW = "assets/source/audio/voices/design";
/** Candidates rendered through their set's profile: scratch, re-rendered locally for free. */
export const PREVIEWS = "throwaway/voice-design";
const VOICES = "fixtures/voices.json";

/** A voice's design identity, as `retakes` (`<set>/<voice>/design`) and the lock key it. */
export const designId = (set: string, voice: string) => `${set}/${voice}`;
export const designRetake = (id: string) => `${id}/design`;
/** A candidate's preview as the set's profile renders it. Candidates count from 1. */
export const previewFile = (id: string, candidate: number) => `${PREVIEWS}/${id}/${candidate}.mp3`;

export function auditionText(catalog: VoiceCatalog, role: string): string {
  const lines = catalog.roles[role]?.lines;
  if (!lines) throw new Error(`role ${role}: not in voices.json`);
  const text = AUDITION.map(([trigger, intensity]) => lines[trigger][intensity][0]).join(" ");
  if (words(text).join(" ").length < MIN_AUDITION)
    throw new Error(`role ${role}: audition shorter than ${MIN_AUDITION} characters`);
  return text;
}

const today = () => new Date().toISOString().slice(0, 10);

/** What a design is a function of: changing any of these buys new candidates. */
function designKey(design: Pick<DesignRecord, "description" | "text" | "model" | "retake">) {
  return sha256(JSON.stringify([design.description, design.text, design.model, design.retake]));
}

/** Voices named `<set>/<voice>`, or by default every voice still waiting for a voice_id. */
function designTargets(catalog: VoiceCatalog, named: readonly string[] | undefined) {
  const all = Object.entries(catalog.sets).flatMap(([set, spec]) =>
    spec.voices.map((voice) => ({ id: designId(set, voice.id), set, voice })),
  );
  if (!named?.length) return all.filter(({ voice }) => voice.voice_id === null);
  return named.map((id) => {
    const found = all.find((target) => target.id === id);
    if (!found) throw new Error(`voice ${id}: not in voices.json`);
    return found;
  });
}

export interface DesignRun {
  root: string;
  catalog: VoiceCatalog;
  client: ElevenLabsClient;
  /** `<set>/<voice>` ids; default every voice whose voice_id is null. */
  voices?: readonly string[];
  log: (line: string) => void;
}

/**
 * `generate design`: three Voice Design candidates per voice, each kept as a raw
 * master and transcribed against the role's audition. A voice whose current
 * design (description, audition, model, retake) already has candidates is not
 * designed again, and a transcript already taken is not taken again, so a killed
 * run is simply rerun.
 */
export async function designVoices(run: DesignRun) {
  const { root, catalog, client, log } = run;
  const lockPath = join(root, LOCK);
  const lock = readLock(lockPath);
  const targets = designTargets(catalog, run.voices);
  const stop = { reason: null as Error | null };
  const designs = new Pool(LIMITS.design, { stop });
  const stt = new Pool(LIMITS.stt, { stop });
  const result = { voices: targets.length, designs: 0, transcripts: 0, failed: 0 };
  let done = 0;

  async function design({ id, set, voice }: (typeof targets)[number]): Promise<string> {
    const text = auditionText(catalog, catalog.sets[set].role);
    const retake = catalog.retakes[designRetake(id)] ?? 0;
    const wanted = { description: voice.description, text, model: DESIGN_MODEL, retake };
    const key = designKey(wanted);
    let record = lock.designs[id];
    if (record?.design_key !== key) {
      const previews = await designs.run(() =>
        client.designVoice(voice.description, text, DESIGN_MODEL),
      );
      result.designs++;
      const candidates = previews.map((preview, i): CandidateRecord => {
        const bytes = Buffer.from(preview.audio_base_64, "base64");
        const raw_sha256 = sha256(bytes);
        const raw = `${RAW}/${id}/candidate-${i + 1}.${raw_sha256.slice(0, 8)}.mp3`;
        writeAtomic(join(root, raw), bytes);
        return {
          generated_voice_id: preview.generated_voice_id,
          raw,
          raw_sha256,
          duration_s: preview.duration_secs,
          transcript: null,
          match: null,
        };
      });
      // A superseded design's masters stay on disk (paid-for source media is never deleted);
      // the lock keeps only the current candidates.
      record = { ...wanted, design_key: key, date: today(), candidates };
      lock.designs[id] = record;
      // The paid-for previews are locked before transcription, so a kill never pays twice.
      writeLock(lockPath, lock);
    }
    const designed = record;
    await Promise.all(
      designed.candidates.map(async (candidate) => {
        if (candidate.transcript !== null) return;
        const bytes = readFileSync(join(root, candidate.raw));
        candidate.transcript = (await stt.run(() => client.stt(bytes))).text;
        candidate.match = matches(text, candidate.transcript, { callsign: "" });
        result.transcripts++;
      }),
    );
    writeLock(lockPath, lock);
    return designed.candidates
      .map((c, i) => `${i + 1}:${c.match?.score.toFixed(2)} ${c.duration_s.toFixed(1)}s`)
      .join("  ");
  }

  await Promise.all(
    targets.map(async (target) => {
      let status: string;
      try {
        status = await design(target);
      } catch (error) {
        result.failed++;
        status = `failed: ${error instanceof Error ? error.message : error}`;
      }
      log(`[${++done}/${targets.length}] ${target.id} ${status}`);
    }),
  );
  writeLock(lockPath, lock);
  return { ...result, stopped: stop.reason };
}

/**
 * `generate preview`: renders each candidate through its set's profile into the
 * scratch folder, only when the profile, bed or master changed. No calls.
 */
export async function renderPreviews(run: {
  root: string;
  catalog: VoiceCatalog;
  processor: Processor;
  profiles: VoiceProfiles;
  voices?: readonly string[];
  log: (line: string) => void;
}): Promise<number> {
  const { root, catalog, profiles } = run;
  const lock = readLock(join(root, LOCK));
  const bedOf = bedResolver(root, lock);
  const local = new Pool(4);
  let rendered = 0;
  const ids = run.voices?.length ? run.voices : Object.keys(lock.designs);
  await Promise.all(
    ids.flatMap((id) => {
      const record = lock.designs[id];
      const spec = catalog.sets[id.split("/")[0]];
      if (!record || !spec) return [];
      const profile = profiles.profiles[spec.processing];
      if (!profile) throw new Error(`profile ${spec.processing}: not defined by assets.py`);
      const bed = bedOf(profile.bed);
      return record.candidates.map(async (candidate, i) => {
        const output = join(root, previewFile(id, i + 1));
        const key = processingKey({
          rawSha: candidate.raw_sha256,
          profile: { name: spec.processing, ...profile },
          bedSha: bed?.sha256 ?? null,
          encoder: profiles.encoder,
        });
        const stamp = `${output}.key`;
        if (existsSync(output) && existsSync(stamp) && readFileSync(stamp, "utf8") === key) return;
        await local.run(async () => {
          mkdirSync(dirname(output), { recursive: true });
          const partial = `${output}.partial.mp3`;
          await run.processor({
            profile: spec.processing,
            input: join(root, candidate.raw),
            bed: bed?.path ?? null,
            output: partial,
          });
          renameSync(partial, output);
          writeAtomic(stamp, key);
        });
        rendered++;
      });
    }),
  );
  run.log(`previews rendered=${rendered}`);
  return rendered;
}

/**
 * The single writer of a saved voice into `voices.json`: the same fixture
 * publication the workbench saves through, so neither overwrites the other.
 */
async function writeVoiceId(
  root: string,
  validate: (value: unknown) => VoiceCatalog,
  set: string,
  voice: string,
  voiceId: string,
) {
  const publication = new FixturePublication(root, "voices", (message) => new Error(message));
  const before = await publication.capture(async () => readFileSync(join(root, VOICES), "utf8"));
  const catalog = JSON.parse(before) as VoiceCatalog;
  catalog.sets[set].voices.find((v) => v.id === voice)!.voice_id = voiceId;
  validate(catalog);
  const after = `${JSON.stringify(catalog, null, 2)}\n`;
  await publication.publish([{ path: VOICES, before, after }], async (phase) => {
    const current = readFileSync(join(root, VOICES), "utf8");
    if (current !== (phase === "before" ? before : after))
      throw new Error("voices.json changed during the pick; outside edits were kept");
  });
}

/**
 * `generate pick`: saves a designed candidate as a permanent voice and writes its
 * voice_id into `voices.json`. A voice that already has one is refused unless
 * `replace`. True when the voice was saved.
 */
export async function pickVoice(run: {
  root: string;
  client: ElevenLabsClient;
  validate: (value: unknown) => VoiceCatalog;
  /** `<set>/<voice>`. */
  voice: string;
  /** 1-based. */
  candidate: number;
  replace?: boolean;
  log: (line: string) => void;
}): Promise<boolean> {
  const { root, voice: id, candidate, log } = run;
  const [set, voiceName] = id.split("/");
  const catalog = run.validate(JSON.parse(readFileSync(join(root, VOICES), "utf8")));
  const voice = catalog.sets[set]?.voices.find((v) => v.id === voiceName);
  if (!voice) throw new Error(`voice ${id}: not in voices.json`);
  if (voice.voice_id !== null && !run.replace) {
    log(`REFUSED ${id} already has voice ${voice.voice_id}; pass --replace to replace it`);
    return false;
  }
  const lockPath = join(root, LOCK);
  const lock = readLock(lockPath);
  const chosen = lock.designs[id]?.candidates[candidate - 1];
  if (!chosen) {
    log(`REFUSED ${id} has no candidate ${candidate}; run \`generate design ${id}\``);
    return false;
  }
  const voiceId = await run.client.saveVoice(
    `battlegame ${set} ${voiceName}`,
    voice.description,
    chosen.generated_voice_id,
    { project: "battlegame", set, voice: voiceName },
  );
  await writeVoiceId(root, run.validate, set, voiceName, voiceId);
  lock.designs[id].picked = { candidate, voice_id: voiceId, date: today() };
  writeLock(lockPath, lock);
  log(`PICKED ${id} candidate ${candidate} as ${voiceId}`);
  return true;
}

const median = (values: number[]) => {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = sorted.length >> 1;
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
};

/**
 * `generate pick --auto`'s choice for every voice still waiting: the candidate
 * whose transcript best matches the audition, tie-broken by the duration nearest
 * the median of every candidate of that role.
 */
export function autoPicks(
  catalog: VoiceCatalog,
  lock: VoiceLock,
): { voice: string; candidate: number }[] {
  const durations = new Map<string, number[]>();
  for (const [id, record] of Object.entries(lock.designs)) {
    const role = catalog.sets[id.split("/")[0]]?.role;
    if (!role) continue;
    durations.set(role, [
      ...(durations.get(role) ?? []),
      ...record.candidates.map((c) => c.duration_s),
    ]);
  }
  const picks: { voice: string; candidate: number }[] = [];
  for (const { id, set } of designTargets(catalog, undefined)) {
    const candidates = lock.designs[id]?.candidates ?? [];
    if (!candidates.length || candidates.some((c) => !c.match)) continue;
    const middle = median(durations.get(catalog.sets[set].role)!);
    const best = candidates
      .map((c, i) => ({ i, score: c.match!.score, off: Math.abs(c.duration_s - middle) }))
      .sort((a, b) => b.score - a.score || a.off - b.off)[0];
    picks.push({ voice: id, candidate: best.i + 1 });
  }
  return picks;
}

import { shippedRules } from "../../../../web/src/shippedRules";
import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  BARK_TRIGGERS,
  lineText,
  takeFile,
  takeId,
  voiceTriggers,
  voiceUrl,
  type Intensity,
  type Trigger,
  type VoiceCatalog,
  type VoiceTake,
} from "../../src/voices/voiceCatalog";
import type { Generated } from "../../src/catalog";
import { generationKey, processingKey, sha256 } from "./cacheKey";
import type { ElevenLabsClient } from "./elevenlabs";
import {
  LOCK,
  readLock,
  verdictOf,
  writeAtomic,
  writeLock,
  type TakeRecord,
  type VoiceLock,
} from "./lock";
import { checkPacks } from "./packs";
import { LIMITS, Pool } from "./pool";
import { matches } from "./transcript";

/** Barks share the simulation's talk time; responses have no simulation event. */
const RESPONSE_TALK_S = 4;
function talkTime(trigger: string): number {
  return BARK_TRIGGERS.some((bark) => bark === trigger)
    ? shippedRules.shouts.talk_s
    : RESPONSE_TALK_S;
}

export const TTS_MODEL = "eleven_v3";
/** v3 takes stability 0, 0.5 or 1; "natural" keeps a voice consistent without flattening it. */
export const VOICE_SETTINGS = { stability: 0.5 };
/** Ambience the game has no recording of, generated once as a raw master. A machine the
 *  game already plays takes its bed from that clip instead (`assets.py` `VOICE_PROFILES`). */
export const BEDS: Record<string, { prompt: string; duration_s: number }> = {
  tank_engine: {
    prompt:
      "Inside a main battle tank: steady diesel engine rumble and turbine whine, metallic rattles, muffled, constant loop",
    duration_s: 12,
  },
  jet_cockpit: {
    prompt:
      "Inside a fighter jet cockpit in flight: steady turbofan roar and air rush over the canopy, muffled, constant loop",
    duration_s: 12,
  },
};
const FLUSH_EVERY = 20;
const LOCAL_JOBS = 4;
const RAW = "assets/source/audio/voices";
const SOUNDS = "fixtures/sounds.json";
const ASSETS = fileURLToPath(new URL("../assets.py", import.meta.url));

/** A profile's bed: the game's own sound for the machine (a `sounds.json` clip id) or, where
 *  the game has none, a generated raw master (a lock `beds` name); either through `filter`. */
export type BedSpec = (
  | { clip: string; generated?: never }
  | { generated: string; clip?: never }
) & {
  filter: string;
  volume: number;
};
/** `assets.py voice-profiles`: the processing owner's definitions, hashed into processing keys. */
export interface VoiceProfiles {
  encoder: object;
  profiles: Record<string, { bed: BedSpec | null; [setting: string]: unknown }>;
}
/** Renders a raw master through a profile into `output`. */
export type Processor = (job: {
  profile: string;
  input: string;
  bed: string | null;
  output: string;
}) => Promise<{ duration_s: number }>;

export interface VoiceSelection {
  sets?: string[];
  voices?: string[];
  triggers?: Trigger[];
  intensities?: Intensity[];
  take?: number;
  /** missing: no take in the lock yet; bad: judged bad; flagged: speech-to-text flagged it. */
  only?: "missing" | "bad" | "flagged";
}
export interface VoiceRun {
  root: string;
  catalog: VoiceCatalog;
  /** Without a client nothing is generated or transcribed: processing only. */
  client: ElevenLabsClient | null;
  processor: Processor;
  profiles: VoiceProfiles;
  select: VoiceSelection;
  /** Characters of new speech this run may send. */
  maxChars?: number;
  dryRun?: boolean;
  log: (line: string) => void;
}
export interface VoiceRunResult {
  takes: number;
  calls: number;
  characters: number;
  flagged: number;
  failed: number;
  /** Why the run stopped early: a bad key, a quota, or failures in a row. */
  stopped: Error | null;
}

const today = () => new Date().toISOString().slice(0, 10);

/**
 * The file a profile's bed is mixed from and its sha256, the bed's identity in the
 * processing key: a catalog clip's prepared runtime file, or a generated raw master.
 * Throws when the bed is missing or its bytes differ from their record.
 */
export function bedResolver(root: string, lock: VoiceLock) {
  let clips: Record<string, { url: string; sha256: string }> | undefined;
  // Each bed is proven once per run, not once per take.
  const proven = new Map<string, { path: string; sha256: string }>();
  return (bed: BedSpec | null): { path: string; sha256: string } | null => {
    if (!bed) return null;
    const known = proven.get(bed.clip ?? bed.generated!);
    if (known) return known;
    const held = bed.clip
      ? (clips ??= JSON.parse(readFileSync(join(root, SOUNDS), "utf8")).clips)[bed.clip]
      : lock.beds[bed.generated!];
    if (!held)
      throw new Error(
        bed.clip
          ? `bed clip ${bed.clip}: not in sounds.json`
          : `bed ${bed.generated}: run \`generate beds\``,
      );
    const path = join(root, "url" in held ? join("assets/runtime", held.url) : held.raw);
    if (!existsSync(path) || sha256(readFileSync(path)) !== held.sha256)
      throw new Error(
        `bed ${bed.clip ?? bed.generated}: file differs from its sha256 (pull Git LFS)`,
      );
    const resolved = { path, sha256: held.sha256 };
    proven.set(bed.clip ?? bed.generated!, resolved);
    return resolved;
  };
}
const servedPath = (root: string, take: VoiceTake, sha: string) =>
  join(root, "assets/runtime", voiceUrl(take, sha));

/** Every take the selection names whose voice is saved, in catalog order. */
export function selectTakes(catalog: VoiceCatalog, lock: VoiceLock, select: VoiceSelection) {
  const takes: VoiceTake[] = [];
  let unvoiced = 0;
  for (const set of select.sets ?? Object.keys(catalog.sets)) {
    if (!catalog.sets[set]) throw new Error(`set ${set}: not in voices.json`);
    for (const voice of catalog.sets[set].voices) {
      if (select.voices && !select.voices.includes(voice.id)) continue;
      for (const trigger of voiceTriggers(voice)) {
        if (select.triggers && !select.triggers.includes(trigger)) continue;
        for (const intensity of select.intensities ?? (["steady", "stressed"] as const))
          for (const take of select.take === undefined ? [0, 1] : [select.take]) {
            const candidate = { set, voice: voice.id, trigger, intensity, take };
            const record = lock.takes[takeId(candidate)];
            if (select.only === "missing" && record) continue;
            if (select.only === "flagged" && !record?.match.flagged) continue;
            if (select.only === "bad" && !(record && verdictOf(record) === "bad")) continue;
            if (voice.voice_id) takes.push(candidate);
            else unvoiced++;
          }
      }
    }
  }
  return { takes, unvoiced };
}

/**
 * Brings each selected take up to date: raw master (cached by generation key),
 * served file (cached by processing key), transcript and match. A take already
 * paid for is never paid for again, even when the lock lost its row in a kill.
 */
export async function generateVoices(run: VoiceRun): Promise<VoiceRunResult> {
  const { root, catalog, client, profiles, log } = run;
  const lockPath = join(root, LOCK);
  const lock = readLock(lockPath);
  const selected = selectTakes(catalog, lock, run.select);
  const { unvoiced } = selected;
  // Without a client a run only re-renders what it already has.
  const takes = client ? selected.takes : selected.takes.filter((take) => lock.takes[takeId(take)]);
  if (client && unvoiced)
    log(`${unvoiced} selected takes wait for a designed voice (voice_id null)`);
  const stop = { reason: null as Error | null };
  const tts = new Pool(LIMITS.tts, { stop });
  const stt = new Pool(LIMITS.stt, { stop });
  const local = new Pool(LOCAL_JOBS, { stop });
  const bedOf = bedResolver(root, lock);
  const result: VoiceRunResult = {
    takes: takes.length,
    calls: 0,
    characters: 0,
    flagged: 0,
    failed: 0,
    stopped: null,
  };
  let done = 0;
  let unflushed = 0;
  const line = (take: VoiceTake, status: string) =>
    log(`[${++done}/${takes.length}] ${takeId(take)} ${status}`);

  async function produce(take: VoiceTake): Promise<string> {
    const set = catalog.sets[take.set];
    const voice = set.voices.find((candidate) => candidate.id === take.voice)!;
    const voiceId = voice.voice_id!;
    const id = takeId(take);
    const text = lineText(catalog, take.set, take.voice, take.trigger, take.intensity, take.take);
    const generation = generationKey({
      text,
      voiceId,
      model: TTS_MODEL,
      settings: VOICE_SETTINGS,
      retake: catalog.retakes[id] ?? 0,
    });
    const rawFile = `${RAW}/${take.set}/${takeFile(take, generation)}`;
    const previous = lock.takes[id];
    let raw: Uint8Array;
    if (existsSync(join(root, rawFile))) raw = readFileSync(join(root, rawFile));
    else {
      if (!client) return "needs generation";
      if (result.characters + text.length > (run.maxChars ?? Infinity)) return "over budget";
      result.characters += text.length;
      if (run.dryRun) return `would generate (${text.length} chars)`;
      raw = await tts.run(() => client.tts(voiceId, text, VOICE_SETTINGS, TTS_MODEL));
      result.calls++;
      writeAtomic(join(root, rawFile), raw);
    }
    if (run.dryRun) return "cached";
    const rawSha = sha256(raw);
    // The lock row still describes these raw bytes, so its later steps may hold too.
    const kept =
      previous?.generation_key === generation && previous.raw_sha256 === rawSha
        ? previous
        : undefined;

    const profile = profiles.profiles[set.processing];
    if (!profile) throw new Error(`profile ${set.processing}: not defined by assets.py`);
    const bed = bedOf(profile.bed);
    const processing = processingKey({
      rawSha,
      profile: { name: set.processing, ...profile },
      bedSha: bed?.sha256 ?? null,
      encoder: profiles.encoder,
    });
    const served =
      kept?.processing_key === processing && existsSync(servedPath(root, take, kept.served_sha256))
        ? { sha: kept.served_sha256, duration: kept.duration_s }
        : await local.run(async () => {
            const output = join(dirname(servedPath(root, take, processing)), `.${processing}.mp3`);
            mkdirSync(dirname(output), { recursive: true });
            const { duration_s } = await run.processor({
              profile: set.processing,
              input: join(root, rawFile),
              bed: bed?.path ?? null,
              output,
            });
            const sha = sha256(readFileSync(output));
            renameSync(output, servedPath(root, take, sha));
            return { sha, duration: duration_s };
          });

    let transcript = kept?.transcript ?? null;
    if (transcript === null) {
      if (!client) return "needs transcription";
      transcript = (await stt.run(() => client.stt(raw))).text;
      result.calls++;
    }
    const match = matches(text, transcript, {
      callsign: set.callsign,
      length: { durationS: served.duration, talkS: talkTime(take.trigger) },
    });
    const provenance: Generated = {
      kind: "Generated",
      generator: "ElevenLabs text-to-speech",
      model: TTS_MODEL,
      text,
      voice_id: voiceId,
      voice_description: voice.description,
      date: kept?.provenance.date ?? today(),
      plan: "Duet ElevenLabs",
    };
    const record: TakeRecord = {
      text,
      generation_key: generation,
      raw: rawFile,
      raw_sha256: rawSha,
      processing_key: processing,
      served_sha256: served.sha,
      duration_s: served.duration,
      transcript,
      match,
      provenance,
      ...(previous?.verdict ? { verdict: previous.verdict } : {}),
    };
    // A superseded raw master stays (paid-for source media is never deleted); its served
    // file is a re-renderable derivation that ships with every deploy, so it goes.
    if (previous && previous.served_sha256 !== served.sha)
      rmSync(servedPath(root, take, previous.served_sha256), { force: true });
    lock.takes[id] = record;
    if (++unflushed >= FLUSH_EVERY) {
      writeLock(lockPath, lock);
      unflushed = 0;
    }
    if (match.flagged) result.flagged++;
    const notes = [
      ...(match.missing.length ? [`missing ${match.missing.join(",")}`] : []),
      ...(match.overlong ? ["overlong"] : []),
    ];
    return `${match.flagged ? "flagged" : "ok"} ${served.duration.toFixed(1)}s match ${match.score.toFixed(2)}${notes.map((note) => ` ${note}`).join("")}`;
  }

  await Promise.all(
    takes.map(async (take) => {
      try {
        line(take, await produce(take));
      } catch (error) {
        result.failed++;
        // A stopped pool refuses everything queued; say why once.
        if (error === stop.reason && result.stopped) return;
        result.stopped = stop.reason;
        line(take, `failed: ${error instanceof Error ? error.message : error}`);
      }
    }),
  );
  if (!run.dryRun) writeLock(lockPath, lock);
  return result;
}

/** Generates each bed once; a bed whose prompt is unchanged is never paid for again. */
export async function generateBeds(run: {
  root: string;
  client: ElevenLabsClient;
  log: (line: string) => void;
}): Promise<{ calls: number }> {
  const lockPath = join(run.root, LOCK);
  const lock = readLock(lockPath);
  const pool = new Pool(LIMITS.effects);
  let calls = 0;
  await Promise.all(
    Object.entries(BEDS).map(async ([name, bed]) => {
      const held = lock.beds[name];
      if (
        held?.provenance.prompt === bed.prompt &&
        existsSync(join(run.root, held.raw)) &&
        sha256(readFileSync(join(run.root, held.raw))) === held.sha256
      )
        return run.log(`bed ${name} cached`);
      const bytes = await pool.run(() => run.client.sound(bed.prompt, bed.duration_s, true));
      calls++;
      const sha = sha256(bytes);
      const raw = `${RAW}/beds/${name}.${sha.slice(0, 8)}.mp3`;
      writeAtomic(join(run.root, raw), bytes);
      lock.beds[name] = {
        raw,
        sha256: sha,
        provenance: {
          kind: "Generated",
          generator: "ElevenLabs sound effects",
          model: "eleven_text_to_sound_v2",
          prompt: bed.prompt,
          date: today(),
          plan: "Duet ElevenLabs",
        },
      };
      run.log(`bed ${name} generated ${bytes.length} bytes`);
    }),
  );
  writeLock(lockPath, lock);
  return { calls };
}

/**
 * Proves the lock against the files by hash: every raw master and served file
 * exists with its recorded sha, and was processed with today's profile. It never
 * re-renders, because MP3 encoders differ between ffmpeg versions. Then proves
 * the served packs (`checkPacks`).
 */
export function checkVoices(run: {
  root: string;
  catalog: VoiceCatalog;
  profiles: VoiceProfiles;
}): string[] {
  const { root, catalog, profiles } = run;
  const lock = readLock(join(root, LOCK));
  const problems: string[] = [];
  const holds = (path: string, sha: string) =>
    existsSync(path) && sha256(readFileSync(path)) === sha;
  const bedOf = bedResolver(root, lock);
  const bedShas = new Map<string, string | null>();
  for (const [name, profile] of Object.entries(profiles.profiles))
    try {
      bedShas.set(name, bedOf(profile.bed)?.sha256 ?? null);
    } catch (error) {
      problems.push(`profile ${name}: ${(error as Error).message}`);
    }
  const named = new Set(Object.values(profiles.profiles).map((profile) => profile.bed?.generated));
  for (const name of Object.keys(lock.beds))
    if (!named.has(name)) problems.push(`bed ${name}: no profile uses it`);
  for (const [id, design] of Object.entries(lock.designs))
    design.candidates.forEach((candidate, i) => {
      if (!holds(join(root, candidate.raw), candidate.raw_sha256))
        problems.push(`design ${id} candidate ${i + 1}: raw master differs`);
    });
  for (const [id, record] of Object.entries(lock.takes)) {
    const [set, voice, trigger, intensity, take] = id.split("/");
    const where: VoiceTake = {
      set,
      voice,
      trigger: trigger as Trigger,
      intensity: intensity as Intensity,
      take: Number(take),
    };
    const spec = catalog.sets[set];
    if (!spec) {
      problems.push(`${id}: set not in voices.json`);
      continue;
    }
    if (!holds(join(root, record.raw), record.raw_sha256))
      problems.push(`${id}: raw master differs`);
    if (!holds(servedPath(root, where, record.served_sha256), record.served_sha256))
      problems.push(`${id}: served file differs`);
    const profile = profiles.profiles[spec.processing];
    const current =
      profile &&
      bedShas.has(spec.processing) &&
      processingKey({
        rawSha: record.raw_sha256,
        profile: { name: spec.processing, ...profile },
        bedSha: bedShas.get(spec.processing)!,
        encoder: profiles.encoder,
      });
    if (current !== record.processing_key)
      problems.push(`${id}: processed with an older profile; run \`generate process\``);
  }
  return [...problems, ...checkPacks(root, catalog, lock)];
}

/** The processing owner's voice profiles and encoder. */
export function readProfiles(): VoiceProfiles {
  const run = spawnSync("python3", [ASSETS, "voice-profiles"], { encoding: "utf8" });
  if (run.status !== 0) throw new Error(`assets.py voice-profiles: ${run.stderr.trim()}`);
  return JSON.parse(run.stdout);
}

/** Processing through `assets.py voice`, the only owner of ffmpeg. */
export const assetsProcessor: Processor = ({ profile, input, bed, output }) =>
  new Promise((resolve, reject) => {
    const args = [ASSETS, "voice", "--profile", profile, "--input", input, "--output", output];
    const child = spawn("python3", bed ? [...args, "--bed", bed] : args);
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", reject);
    child.on("close", (code) =>
      code === 0
        ? resolve(JSON.parse(stdout))
        : reject(new Error(`assets.py voice ${profile}: ${stderr.trim()}`)),
    );
  });

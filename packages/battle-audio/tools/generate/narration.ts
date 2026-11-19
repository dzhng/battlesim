// `generate narration`: every narration line (`voices.json` `narration`) as a
// take in the lock, beside the units' takes and under the same rules. A raw
// master is bought only when its line's text or voice changed (its generation
// key), from `imports` when a review take of that text exists, else through
// Bento; it is processed through its speaker's profile and transcribed, and
// each speaker's pack lists the takes judged ok on their current bytes.
import { copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import type { Generated } from "../../src/catalog";
import type { NarrationPack, VoiceCatalog } from "../../src/voices/voiceCatalog";
import { narrationPackUrl } from "../../src/voices/voiceCatalog";
import type { SpeechClient } from "./bento";
import { generationKey, processingKey, sha256 } from "./cacheKey";
import { LOCK, readLock, verdictOf, writeAtomic, writeLock, type TakeRecord } from "./lock";
import { matches } from "./transcript";
import type { Processor, VoiceProfiles } from "./voices";

const RAW = "assets/source/audio/voices/narration";
const SERVED = "assets/runtime/audio/voices/narration";
const today = () => new Date().toISOString().slice(0, 10);

export interface NarrationRun {
  root: string;
  catalog: VoiceCatalog;
  /** Null re-renders only what the lock already has. */
  client: SpeechClient | null;
  profiles: VoiceProfiles;
  processor: Processor;
  /** A folder of review takes named `<line>.mp3`, spoken in the speaker's
   *  current voice, model and seed: imported in place of a purchase. */
  imports?: string;
  log: (line: string) => void;
}

export async function generateNarration(
  run: NarrationRun,
): Promise<{ bought: number; flagged: number }> {
  const { root, catalog, client, profiles, log } = run;
  const lockPath = join(root, LOCK);
  const lock = readLock(lockPath);
  const { speakers, lines } = catalog.narration;
  let bought = 0;
  let flagged = 0;
  for (const [line, { speaker: who, text }] of Object.entries(lines)) {
    const speaker = speakers[who];
    const id = `narration/${who}/${line}`;
    const generation = generationKey({
      text,
      voiceId: speaker.voice_id,
      model: speaker.model,
      settings: { seed: speaker.seed },
      retake: catalog.retakes[id] ?? 0,
    });
    const rawFile = `${RAW}/${who}/${line}.${generation.slice(0, 8)}.mp3`;
    const previous = lock.takes[id];
    if (!existsSync(join(root, rawFile))) {
      const review = run.imports && join(run.imports, `${line}.mp3`);
      mkdirSync(dirname(join(root, rawFile)), { recursive: true });
      if (review && existsSync(review)) copyFileSync(review, join(root, rawFile));
      else if (client) {
        writeAtomic(
          join(root, rawFile),
          await client.tts(speaker.voice_id, text, speaker.model, speaker.seed),
        );
        bought++;
      } else {
        log(`${id} needs generation`);
        continue;
      }
    }
    const raw = readFileSync(join(root, rawFile));
    const rawSha = sha256(raw);
    const kept =
      previous?.generation_key === generation && previous.raw_sha256 === rawSha
        ? previous
        : undefined;
    const profile = profiles.profiles[speaker.processing];
    if (!profile) throw new Error(`profile ${speaker.processing}: not defined by assets.py`);
    const processing = processingKey({
      rawSha,
      profile: { name: speaker.processing, ...profile },
      bedSha: null,
      encoder: profiles.encoder,
    });
    const servedFile = (sha: string) => join(root, SERVED, who, `${line}.${sha.slice(0, 12)}.mp3`);
    let served = { sha: kept?.served_sha256 ?? "", duration: kept?.duration_s ?? 0 };
    if (kept?.processing_key !== processing || !existsSync(servedFile(kept.served_sha256))) {
      const output = join(root, SERVED, who, `.${processing}.mp3`);
      mkdirSync(dirname(output), { recursive: true });
      const { duration_s } = await run.processor({
        profile: speaker.processing,
        input: join(root, rawFile),
        bed: null,
        output,
      });
      const sha = sha256(readFileSync(output));
      renameSync(output, servedFile(sha));
      served = { sha, duration: duration_s };
    }
    let transcript = kept?.transcript ?? null;
    if (transcript === null) {
      if (!client) {
        log(`${id} needs transcription`);
        continue;
      }
      transcript = (await client.stt(raw)).text;
    }
    const match = matches(text, transcript, { callsign: who });
    const provenance: Generated = {
      kind: "Generated",
      generator: "ElevenLabs text-to-speech",
      model: speaker.model,
      text,
      voice_id: speaker.voice_id,
      voice_description: speaker.description,
      date: kept?.provenance.date ?? today(),
      plan: "Bento ElevenLabs",
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
    if (previous && previous.served_sha256 !== served.sha)
      rmSync(servedFile(previous.served_sha256), { force: true });
    lock.takes[id] = record;
    if (match.flagged) flagged++;
    log(`${id} ${match.flagged ? "FLAGGED" : "ok"} (${served.duration.toFixed(1)} s)`);
  }
  writeLock(lockPath, lock);
  writeNarrationPacks(root, catalog, lock.takes);
  return { bought, flagged };
}

/** Each speaker's pack: its lines' takes judged ok on their current bytes. */
export function writeNarrationPacks(
  root: string,
  catalog: VoiceCatalog,
  takes: Record<string, TakeRecord>,
) {
  for (const speaker of Object.keys(catalog.narration.speakers)) {
    const pack: NarrationPack = { speaker, lines: [] };
    for (const [line, spoken] of Object.entries(catalog.narration.lines)) {
      const record = takes[`narration/${speaker}/${line}`];
      if (spoken.speaker !== speaker || !record || verdictOf(record) !== "ok") continue;
      const url = `${dirname(narrationPackUrl(speaker))}/${line}.${record.served_sha256.slice(0, 12)}.mp3`;
      pack.lines.push({ line, url, duration: record.duration_s });
    }
    const path = join(root, "assets/runtime", narrationPackUrl(speaker));
    mkdirSync(dirname(path), { recursive: true });
    writeAtomic(path, `${JSON.stringify(pack, null, 2)}\n`);
  }
}

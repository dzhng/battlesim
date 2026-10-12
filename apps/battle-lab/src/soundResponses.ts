// Units answering their owner in the sound lab: a selection, an order given
// to a group (asked twice, as two units would) and a refused order, mapped by
// `responseFor` and played through the battle's own voice lines
// (`BattleVoices`: the voice bank and the chatter) on an offline render of the
// battle's mixer, voices bus alone.
//
// No pack is published before the user's review, so the lab stands one in:
// the pilot set's takes the lock holds, every one the transcript check passed
// and nobody judged bad (what `generate pack --unreviewed` would list),
// served from their real files.
import lock from "@fixtures/voices.lock.json";
import voices from "@fixtures/voices.json";
import { BattleVoices } from "@packages/battle-audio/src/voices/battleVoices";
import {
  responseFor,
  type ResponseEvent,
  type ResponseUnit,
} from "@packages/battle-audio/src/voices/responses";
import { voicePack, type VoiceCatalog } from "@packages/battle-audio/src/voices/voiceCatalog";
import type { VoiceLine } from "@packages/battle-audio/src/voices/chatter";
import { loadPresentation } from "@web/battle/sim/presentation";
import { offlineSound } from "./soundFirefight";
import { gameAudio } from "./soundFeed";

export const SET = "us_tank_crew";
const SECONDS = 8.5;
const FPS = 60;

/** The stand-in pack: the lock's unflagged takes of `SET` not judged bad on their bytes. */
export function standInPack() {
  const takes = lock.takes as Record<
    string,
    {
      served_sha256: string;
      duration_s: number;
      match: { flagged: boolean };
      verdict?: { value: string; sha256: string };
    }
  >;
  return voicePack(voices as unknown as VoiceCatalog, SET, (id) => {
    const take = takes[id];
    const bad = take?.verdict?.value === "bad" && take.verdict.sha256 === take.served_sha256;
    return take && !take.match.flagged && !bad
      ? { sha256: take.served_sha256, duration_s: take.duration_s }
      : null;
  });
}

const TANK: ResponseUnit = { id: 1, set: SET, value: 300, stressed: false };
const WINGMAN: ResponseUnit = { id: 2, set: SET, value: 300, stressed: false };
/** What the player does, and when (seconds). */
const SCRIPT: { at: number; event: ResponseEvent }[] = [
  { at: 0.3, event: { kind: "selected", units: [TANK, WINGMAN], lead: 1 } },
  { at: 3, event: { kind: "accepted", order: { kind: "move" }, units: [TANK, WINGMAN], id: 1 } },
  { at: 3.05, event: { kind: "accepted", order: { kind: "move" }, units: [WINGMAN], id: 1 } },
  {
    at: 5.7,
    event: { kind: "refused", reason: "no_valid_destination", units: [TANK, WINGMAN], id: 2 },
  },
];

export interface ResponsesRender {
  sampleRate: number;
  /** Interleaved stereo, voices bus alone. */
  samples: Float32Array;
  /** Each line played: what, by whom, and when (seconds) for how long. */
  lines: {
    trigger: string;
    speaker: string;
    voice: string;
    take: number;
    at: number;
    duration: number;
  }[];
  dropped: { trigger: string; reason: string }[];
  /** When each answer was asked for. */
  asked: { trigger: string; at: number }[];
  errors: string[];
}

export async function renderResponses(): Promise<ResponsesRender> {
  await loadPresentation();
  const { sampleRate, ctx, sink } = await offlineSound(SECONDS, "voices");
  const pack = standInPack();
  const lines = new BattleVoices(sink, gameAudio.voices, {
    packs: async () => pack,
  });
  lines.want([SET]);
  await lines.start();
  const played: VoiceLine[] = [];
  const dropped: ResponsesRender["dropped"] = [];
  const asked: ResponsesRender["asked"] = [];
  let next = 0;
  for (let f = 0; f <= SECONDS * FPS; f++) {
    sink.t = f / FPS;
    while (next < SCRIPT.length && SCRIPT[next].at <= sink.t) {
      const request = responseFor(SCRIPT[next++].event);
      if (request) {
        asked.push({ trigger: request.trigger, at: sink.t });
        lines.respond(request);
      }
    }
    const update = lines.update([0, 0, 0]);
    played.push(...update.started);
    dropped.push(...update.dropped.map((d) => ({ trigger: d.request.trigger, reason: d.reason })));
  }
  const buffer = await ctx.startRendering();
  lines.dispose();
  const samples = new Float32Array(buffer.length * 2);
  const l = buffer.getChannelData(0);
  const r = buffer.getChannelData(1);
  for (let i = 0; i < buffer.length; i++) {
    samples[2 * i] = l[i];
    samples[2 * i + 1] = r[i];
  }
  return {
    sampleRate,
    samples,
    lines: played.map((line) => ({
      trigger: line.request.trigger,
      speaker: line.request.speaker,
      voice: line.voice,
      take: line.take,
      at: line.at,
      duration: line.duration,
    })),
    dropped,
    asked,
    errors: [...lines.errors].map(([what, why]) => `${what}: ${why}`),
  };
}

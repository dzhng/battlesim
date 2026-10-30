import { audioMix } from "@web/audioMix";
// Units answering their owner in the sound lab: a selection, an order given
// to a group (asked twice, as two units would) and a refused order, mapped by
// `responseFor` and played through the battle's own voice lines
// (`BattleVoices`: the voice bank and the chatter) on an offline render of the
// battle's mixer, voices bus alone, from the tank crew's served pack.
import { BattleVoices } from "@packages/battle-audio/src/voices/battleVoices";
import {
  responseFor,
  type ResponseEvent,
  type ResponseUnit,
} from "@packages/battle-audio/src/voices/responses";
import type { VoiceLine } from "@packages/battle-audio/src/voices/chatter";
import { loadPresentation } from "@web/battle/sim/presentation";
import { interleave, offlineSound } from "./soundFirefight";

/** The set the lab's units speak with, and a unit kind that speaks with it. */
export const SET = "us_tank_crew";
export const KIND = "us_m1_abrams_sep_v2";
const SECONDS = 8.5;
const FPS = 60;

const TANK: ResponseUnit = { id: 1, set: SET, value: 300, stressed: false };
const WINGMAN: ResponseUnit = { id: 2, set: SET, value: 300, stressed: false };
/** What the player does, and when (seconds). */
const SCRIPT: { at: number; event: ResponseEvent }[] = [
  { at: 0.3, event: { kind: "selected", units: [TANK, WINGMAN], lead: 1 } },
  { at: 3, event: { kind: "accepted", order: { kind: "move" }, units: [TANK, WINGMAN], id: 1 } },
  { at: 3.05, event: { kind: "accepted", order: { kind: "move" }, units: [WINGMAN], id: 1 } },
  {
    at: 5.7,
    event: {
      kind: "refused",
      reason: "no_valid_destination",
      units: [TANK, WINGMAN],
      group: "order/2",
    },
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
  const lines = new BattleVoices(sink, audioMix.voices);
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
    // A line's take decodes as it starts: let it, while the clock stands.
    await lines.settled();
    played.push(...update.started);
    dropped.push(...update.dropped.map((d) => ({ trigger: d.request.trigger, reason: d.reason })));
  }
  const buffer = await ctx.startRendering();
  lines.dispose();
  return {
    sampleRate,
    samples: interleave(buffer),
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

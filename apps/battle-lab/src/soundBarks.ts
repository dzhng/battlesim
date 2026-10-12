// Barks and the score in the sound lab. A seen enemy, then one of the
// side's own tanks, shout "under fire" in the open; the battle's sound frame
// hears them, the voice lines (`BattleVoices`) speak them where they were
// shouted, and the battle's score (`BattleMusic`) follows the side's own
// barks and dips under every line. Rendered offline, voices bus alone; the
// score streams from a media element, which an offline render has no form
// of, so its player here records what it was asked to do.
//
// No bark take is generated yet, so the lab stands one in: the pilot set's
// own takes (`standInPack`), relabelled as the barks it shouts.
//
// It also samples what loading voice packs costs a battle: how long a fresh
// battle's sounds take to be ready, with packs served and without.
import game from "@fixtures/game.json";
import { AppAudio } from "@packages/battle-audio/src/appAudio";
import { BattleMusic, type BattleMusicRules } from "@packages/battle-audio/src/music/battleMusic";
import type { BattleMood } from "@packages/battle-audio/src/music/musicDirector";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { HeardShout } from "@packages/battle-audio/src/soundFrame";
import { BattleVoices } from "@packages/battle-audio/src/voices/battleVoices";
import type { VoicePack } from "@packages/battle-audio/src/voices/voiceCatalog";
import { loadPresentation } from "@web/battle/sim/presentation";
import { offlineSound } from "./soundFirefight";
import { createBattleAudio, gameAudio } from "./soundFeed";
import { SET, standInPack } from "./soundResponses";

const SECONDS = 7;
const FPS = 60;
const HZ = game.tick_hz;
const KIND = "stand_in_tank";
const BARKS = "stand_in_barks";

/** The pilot set's takes, as a set whose one voice shouts them. */
function barkPack(): VoicePack {
  const takes = standInPack().takes.filter((t) => t.intensity === "stressed");
  return {
    set: BARKS,
    voices: [{ id: "lead", speaks: "all" }],
    takes: takes.slice(0, 2).map((t, take) => ({ ...t, trigger: "under_fire", take })),
  };
}

/** Who shouts, and when (seconds): a seen enemy off to the right, then an
 *  own tank nearer, off to the left; the listener stands at the origin
 *  looking north. */
const SCRIPT: { at: number; shout: HeardShout }[] = [
  {
    at: 0.5,
    shout: {
      own: false,
      id: 9,
      kind: KIND,
      position: [180, 120, 0],
      trigger: "b5_under_fire",
      intensity: "stressed",
    },
  },
  {
    at: 3.5,
    shout: {
      own: true,
      id: 1,
      kind: KIND,
      position: [-50, 30, 0],
      trigger: "b5_under_fire",
      intensity: "stressed",
    },
  },
];

export interface BarksRender {
  sampleRate: number;
  /** Interleaved stereo, voices bus alone. */
  samples: Float32Array;
  /** Each line played: its tier, who shouted, where, and when for how long. */
  lines: {
    tier: string;
    speaker: string;
    position: readonly number[] | null;
    at: number;
    duration: number;
  }[];
  /** When each shout was published. */
  shouted: { speaker: string; at: number }[];
  /** The score's moods as it moved between them, and its dips. */
  moods: { at: number; mood: BattleMood }[];
  ducks: { at: number; depth: number }[];
  errors: string[];
}

export async function renderBarks(): Promise<BarksRender> {
  await loadPresentation();
  const { sampleRate, ctx, sink, frame } = await offlineSound(SECONDS, "voices");
  frame.speakers({ own: { [KIND]: BARKS }, enemy: { [KIND]: BARKS } });
  const pack = barkPack();
  const voices = new BattleVoices(sink, gameAudio.voices, { packs: async () => pack });
  voices.want([BARKS]);
  await voices.start();

  const moods: BarksRender["moods"] = [];
  const ducks: BarksRender["ducks"] = [];
  const moodOf = (track: string): BattleMood =>
    gameSounds.music.moods.combat.includes(track) ? "combat" : "tension";
  const score = new BattleMusic(
    game.presentation.audio.music as BattleMusicRules,
    gameSounds.music.moods,
    {
      play: (track: string) => void moods.push({ at: sink.t, mood: moodOf(track) }),
      crossfadeTo: (track: string) => void moods.push({ at: sink.t, mood: moodOf(track) }),
      duck: (depth: number) => void ducks.push({ at: sink.t, depth }),
    } as never,
    () => 0,
  );

  const lines: BarksRender["lines"] = [];
  const shouted: BarksRender["shouted"] = [];
  const listener = { position: [0, 0, 0], forward: [0, 1, 0] };
  let next = 0;
  for (let f = 0; f <= SECONDS * FPS; f++) {
    sink.t = f / FPS;
    frame.update(sink.t, sink.t, { vehicles: [], soldiers: [] }, listener);
    while (next < SCRIPT.length && SCRIPT[next].at <= sink.t) {
      const { shout } = SCRIPT[next++];
      const tick = Math.round(sink.t * HZ) + 1;
      frame.note({
        effects: { tick, segments: [], blasts: [], smokes: [] },
        launches: [],
        audible: [],
        shouts: [shout],
      });
      shouted.push({ speaker: `${shout.own ? "own" : "seen"}/${shout.id}`, at: sink.t });
    }
    const update = voices.hear(frame);
    for (const line of update.started)
      lines.push({
        tier: line.request.tier,
        speaker: line.request.speaker,
        position: line.request.position ?? null,
        at: line.at,
        duration: line.duration,
      });
    score.update(sink.t, update.heard, voices.speaking(sink.t));
  }
  const buffer = await ctx.startRendering();
  voices.dispose();
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
    lines,
    shouted,
    moods,
    ducks,
    errors: [...voices.errors].map(([what, why]) => `${what}: ${why}`),
  };
}

export interface ReadySample {
  packs: boolean;
  /** Wall milliseconds from creating the battle's sound to its `ready`. */
  readyMs: number;
  /** Voice files asked for before, and after, `ready`. */
  voiceFetchesBefore: number;
  voiceFetchesAfter: number;
}

/** A fresh battle's sound, from creation to `ready`, with the pilot set's
 *  pack served (`packs`) or absent; its own tanks speak with that set. */
export async function sampleReady(packs: boolean): Promise<ReadySample> {
  await loadPresentation();
  const original = window.fetch;
  let battle: ReturnType<typeof createBattleAudio> | null = null;
  const asked = { before: 0, after: 0 };
  const pack = standInPack();
  window.fetch = (input, init) => {
    const url = String(input instanceof Request ? input.url : input);
    if (url.includes("/audio/voices/")) {
      asked[battle?.stats()?.loading === false ? "after" : "before"]++;
      if (url.endsWith("/pack.json"))
        return Promise.resolve(
          packs ? Response.json(pack) : new Response(null, { status: 404 }),
        );
    }
    return original(input, init);
  };
  const app = new AppAudio({ presentation: gameAudio, catalog: gameSounds });
  try {
    const started = performance.now();
    const sound = (battle = createBattleAudio(HZ, app));
    sound.speakers({ own: { m1a2: SET }, enemy: {} });
    while (sound.stats()?.loading !== false) {
      if (performance.now() - started > 60000) throw new Error("battle sound never got ready");
      await new Promise((r) => setTimeout(r, 5));
    }
    const readyMs = performance.now() - started;
    const error = sound.stats()?.error;
    if (error) throw new Error(error);
    // Let the packs, now asked for, arrive.
    const until = performance.now() + 3000;
    while (performance.now() < until && asked.after < (packs ? pack.takes.length + 1 : 1))
      await new Promise((r) => setTimeout(r, 20));
    return {
      packs,
      readyMs,
      voiceFetchesBefore: asked.before,
      voiceFetchesAfter: asked.after,
    };
  } finally {
    app.dispose();
    window.fetch = original;
  }
}

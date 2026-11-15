// The sound frame's test scene: the shipped mix with stand-in recordings, a
// sink that records what starts, and firefights to hear. Shared by the
// sound tests so each keeps to its own subject.
import type { AudioMix } from "@packages/battle-audio/src/audioMix";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import {
  SoundFrame,
  type HeardPublication,
  type Listener,
  type SoundCue,
  type SoundMotion,
  type VoiceSpec,
} from "@packages/battle-audio/src/soundFrame";
import { SOUNDS } from "@packages/battle-audio/src/synthSounds.gen";
import { audioMix } from "@web/audioMix";
import { shippedRules } from "@web/shippedRules";
import { FakeSink } from "./soundSink";
import { standInPresentationRecordings } from "./soundStandIns";
import { testWeaponLooks } from "./weaponLooks";

export const HZ = 30;
export const DT = 1 / HZ;
export const AUDIO: AudioMix = {
  ...audioMix,
  effects: {},
  weapon_defaults: {
    blast: { near: "explosion", far: "explosion_far" },
    motor: "motor",
    impacts: {
      ...audioMix.weapon_defaults.impacts,
      ricochet: "ricochet",
    },
  },
};
export const documents = testWeaponLooks();
export const shot = (kind: keyof typeof shippedRules.weapons) =>
  documents.looks.weapon_looks[shippedRules.weapons[kind].appearance].sound;

/** A publication as the presentation hears it: what the effects read, and
 *  the launches it shows (the presentation derives them; here, given). */
export type Effects = HeardPublication["effects"] & { launches?: HeardPublication["launches"] };
export const heard = ({ launches = [], ...effects }: Effects): HeardPublication => ({
  effects,
  launches,
});
export const LISTENER: Listener = { position: [0, -20, 20], forward: [0, 1, 0] };
export const STILL: SoundMotion = { vehicles: [], soldiers: [] };

/** Every synthesized baseline as a recipe, as the shipped catalog holds them: a
 *  battle prepares, and so can play, only what Rust's list names. */
export const BASELINES: SoundCatalog["sounds"] = Object.fromEntries(
  Object.entries(SOUNDS).map(([name, { loop }]) => [
    name,
    { label: name, clips: [], synth: name, synth_gain: 1, gain: 1, loop },
  ]),
);

export function setup(presentation: AudioMix = AUDIO) {
  const sink = new FakeSink();
  const catalog: SoundCatalog = {
    sources: {},
    clips: {},
    sounds: { ...BASELINES },
    music: { moods: { menu: [], tension: [], combat: [] }, tracks: {} },
  };
  standInPresentationRecordings(catalog, presentation);
  const frame = new SoundFrame(
    {
      tickHz: HZ,
      ...documents,
      presentation,
      catalog,
    },
    sink,
  );
  return { sink, frame };
}

export const launch = (
  shooter: number,
  soldier: number | null,
  at: number[],
  kind: string,
  mount = 0,
) => ({
  shooter,
  mount,
  soldier,
  x: at[0],
  y: at[1],
  z: at[2],
  kind,
});

/** A publication where each shooter's one soldier fires a new round at
 *  tick `tick`: a squad's round is heard from the third tick it is seen,
 *  as the launches name it (its counter rises the tick after it is first
 *  seen, and the round leaves the tick after that). */
export function firefight(tick: number, shooters: { key: number; at: number[] }[]): Effects {
  return {
    tick,
    launches: tick >= 3 ? shooters.map((s) => launch(s.key, s.key * 10, s.at, "rifle")) : [],
    segments: shooters.map((s) => ({
      path: [s.at, [s.at[0] + 10, s.at[1], s.at[2]]],
      ricochets: [],
      kind: "rifle",
      shooter: s.key * 10,
      hit: "ground",
      normal: [0, 0, 1],
    })),
    blasts: [],
    smokes: [],
  };
}

/** Run ticks `from..to`, each noted and heard at its own presentation time. */
export function run(
  frame: SoundFrame,
  sink: FakeSink,
  from: number,
  to: number,
  pub: (tick: number) => Effects,
  audible: (tick: number) => SoundCue[] = () => [],
  motion: SoundMotion = STILL,
) {
  for (let tick = from; tick <= to; tick++) {
    frame.note({ ...heard(pub(tick)), audible: audible(tick) });
    sink.time = tick * DT;
    frame.update(tick * DT, tick * DT, motion, LISTENER);
  }
}

export const near = (spec: VoiceSpec, at: number[], within: number) =>
  spec.position !== null && Math.hypot(spec.position[0] - at[0], spec.position[1] - at[1]) < within;

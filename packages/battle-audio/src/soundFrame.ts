// The battle's sound, as the page plays it. What is heard, when, where and
// how loud is decided in Rust (`presentation::sound::frame`, through the
// presentation module): it reads the publication as the effects take it,
// the frame's emitters and the side's hearing cues, and gives commands
// (start, glide, stop a voice; listen from here; dip the ambience under a
// loud sound). This carries them out on a `VoiceSink` (`webAudioSink.ts`),
// so the mix runs, and is tested, without an audio device. The presentation
// module must be loaded first (`loadPresentation`).
import {
  SoundFrame as RustSoundFrame,
  sector_pan,
  vehicle_loops,
  type Presentation,
} from "@wasm/renderer_browser";
import type { AudioMix, Bus, VehicleSound } from "./audioMix";
import type { SoundCatalog } from "./catalog";
import type { Chatter, ChatterUpdate } from "./voices/chatter";

type P3 = readonly [number, number, number] | readonly number[];

/** A hearing cue, as the observation publishes it. */
export interface SoundCue {
  category: string;
  /** 0 = east, counter-clockwise in 45° steps, from the listening unit. */
  sector: number;
  band: string;
  moving: boolean;
}

type Point = readonly number[];

/** A scripted publication as the presentation hears it: what the effects
 *  read of it, and the launches it shows. */
export interface HeardPublication {
  effects: {
    tick: number;
    segments: readonly {
      path: readonly Point[];
      ricochets: readonly { point: number; normal: Point }[];
      kind: string;
      shooter: number | null;
      hit: string;
      normal: Point | null;
    }[];
    blasts: readonly { point: Point; radius: number; kind: string }[];
    smokes: readonly { key: string; kind: string; center: Point; yaw: number; half: Point }[];
    cookOffs?: readonly { center: Point; height: number; landing: Point | null }[];
    flareDeploys?: readonly Point[];
  };
  launches: readonly {
    shooter: number;
    mount: number;
    soldier: number | null;
    x: number;
    y: number;
    z: number;
    kind: string;
  }[];
}

/** A shout heard in the open, as the observation publishes it: an own
 *  unit's (`own`, by unit id) or an identified enemy's (by handle). */
export interface HeardShout {
  own: boolean;
  id: number;
  /** The speaker's unit kind. */
  kind: string;
  position: Point;
  /** The contract's bark tag, `b5_under_fire`. */
  trigger: string;
  intensity: string;
}

/** One scripted publication, as sound reads it. */
export interface SoundPublication extends HeardPublication {
  audible: readonly SoundCue[];
  shouts?: readonly HeardShout[];
}

/** Which voice set each unit kind speaks with: for the side's own faction,
 *  and for its enemy's (`voices.json` assignments). */
export interface Speakers {
  own: Readonly<Record<string, string>>;
  enemy: Readonly<Record<string, string>>;
}

/** A drawn vehicle, as a script poses it. */
export interface SoundVehicle {
  /** Unique across both sides (the effects' shooter key). */
  key: number;
  /** Its presentation class (the catalog's `vehicle_class`): the `vehicles` row it sounds as. */
  vehicleClass: string;
  position: P3;
  /** Metres the left and right running gear have rolled. */
  travelL: number;
  travelR: number;
  /** Turret heading relative to the hull, radians. */
  turret: number;
  /** Backing up: a reverse move, or a turn's reversing leg. */
  reverse?: boolean;
}

/** A drawn soldier, as a script poses it. */
export interface SoundSoldier {
  id: number;
  position: P3;
}

/** What a script draws moving this frame. */
export interface SoundMotion {
  vehicles: readonly SoundVehicle[];
  soldiers: readonly SoundSoldier[];
}

/** The listener: where it stands and which way it looks (horizontal). */
export interface Listener {
  position: P3;
  forward: P3;
}

/** A voice to start. `position` null is not positional: `pan` places it
 *  (a cue's direction), or it plays centred (the ambience bed). */
export interface VoiceSpec {
  sound: string;
  /** Stable event-derived variation, interpreted by the prepared bank. */
  variant: number;
  bus: Bus;
  /** Sink time to start at, seconds. */
  at: number;
  /** Explicit source offset in seconds; omitted loops stagger their phases. */
  offset?: number;
  gain: number;
  rate: number;
  /** Low-pass cutoff, Hz. */
  lowpass: number;
  /** Reverb send, a share of the dry level. */
  wet: number;
  /** Seconds the onset ramps up over (0: as synthesised). */
  attack: number;
  position: P3 | null;
  pan: number | null;
  loop: boolean;
}

/** A running loop's live parameters. */
export interface VoiceParams {
  gain: number;
  rate: number;
  lowpass: number;
  wet: number;
  position: P3 | null;
}

/** The audio graph, as the frame drives it. */
export interface VoiceSink {
  /** The sink's clock, seconds. */
  now(): number;
  /** A sound's length at rate 1, seconds. */
  duration(sound: string, variant?: number): number;
  start(id: number, voice: VoiceSpec): void;
  set(id: number, params: VoiceParams, at: number): void;
  /** Fade out over `fade` seconds from `at`, then free. */
  stop(id: number, at: number, fade: number): void;
  listen(listener: Listener, at: number): void;
  /** Dip the ambience bus by `depth`, a share of its level (0 restores
   *  it), from `at`, gliding there with time constant `glide` seconds. */
  duck(depth: number, glide: number, at: number): void;
}

export interface SoundStats {
  /** Voices sounding now: transients and loops. */
  transients: number;
  loops: number;
  /** Transients waiting for the clock. */
  pending: number;
  /** Started, dropped for the budget, and stolen by louder ones, since the last reset. */
  started: number;
  dropped: number;
  stolen: number;
  /** Positional transients started, and cue sounds started, since the last reset. */
  positional: number;
  cues: number;
  /** Holding: the clock stands still. */
  held: boolean;
  /** The last publication taken. */
  tick: number;
  /** Voices started, loops included, by sound name since this frame's last
   *  `reset()`: which causes were heard (an air alarm, a jet's approach), not
   *  only how many. Counted as the voices reach the sink, so a publication
   *  that rewinds the battle without a reset does not clear it. */
  bySound: Record<string, number>;
}

export interface SoundFrameOptions {
  tickHz: number;
  presentation: AudioMix;
  catalog: SoundCatalog;
  /** The battle's weapon rows, whose appearances name admitted looks. */
  weapons: Record<string, object>;
  looks: object;
  /** How long each smoke kind burns and smoulders (`presentation.effects.smoke`). */
  smokeTimes: Record<string, { burn_s: number; smoulder_s: number }>;
  /** When a cook-off's fireballs go and its turret lands (`presentation.effects.cook_off`). */
  cookOff: object;
}

type Command =
  | ({ op: "start"; id: number } & VoiceSpec)
  | ({ op: "set"; id: number; at: number } & VoiceParams)
  | { op: "stop"; id: number; at: number; fade: number }
  | ({ op: "listen"; at: number } & Listener)
  | { op: "duck"; at: number; bus: "ambience"; depth: number; glide: number };

/** Stereo position of a direction sector for a listener looking along `forward`. */
export const sectorPan = (sector: number, forward: P3): number =>
  sector_pan(sector, Float64Array.from(forward));

/** One of a vehicle's sounding loops. */
export interface VehicleLoop {
  slot: "engine" | "running" | "turret" | "reverse";
  sound: string;
  gain: number;
  rate: number;
}

/** The loops a vehicle of `row` sounds at `load` and `traverse` (shares of
 *  full), reversing or not, as the battle mixes them. */
export function vehicleLoops(
  row: VehicleSound,
  load: number,
  traverse: number,
  reversing: boolean,
): VehicleLoop[] {
  return JSON.parse(vehicle_loops(JSON.stringify(row), load, traverse, reversing)) as VehicleLoop[];
}

const rustFrame = (o: SoundFrameOptions) =>
  new RustSoundFrame(
    JSON.stringify({
      audio: o.presentation,
      catalog: o.catalog,
      weapons: o.weapons,
      presentation: o.looks,
      tick_hz: o.tickHz,
      effects: { smoke: o.smokeTimes, cook_off: o.cookOff },
    }),
  );

/** Every recipe a battle with `options` can play, as the Rust frame names
 *  them: the synthesized baselines, each assigned or replacing recipe and
 *  each sound its presentation names. The rest of the library is for
 *  audition, prepared only when auditioned. The module must be loaded. */
export function battleSounds(options: SoundFrameOptions): string[] {
  const frame = rustFrame(options);
  try {
    return JSON.parse(frame.sounds()) as string[];
  } finally {
    frame.free();
  }
}

export class SoundFrame {
  private frame: RustSoundFrame | null = null;
  /** Voices started since the last reset, by sound name (`SoundStats.bySound`). */
  private bySound: Record<string, number> = {};

  constructor(
    private readonly options: SoundFrameOptions,
    private readonly sink: VoiceSink,
  ) {}

  /** The Rust frame, made on first use (the module is loaded by then),
   *  told every prepared sound's length. */
  private get rust(): RustSoundFrame {
    if (this.frame) return this.frame;
    const o = this.options;
    const frame = rustFrame(o);
    const durations: Record<string, number[]> = {};
    for (const name of JSON.parse(frame.sounds()) as string[]) {
      const variants = Math.max(1, o.catalog.sounds[name]?.clips.length ?? 0);
      durations[name] = Array.from({ length: variants }, (_, k) => this.sink.duration(name, k));
    }
    frame.durations(JSON.stringify(durations));
    return (this.frame = frame);
  }

  private run(commands: string) {
    for (const c of JSON.parse(commands) as Command[]) {
      if (c.op === "start") {
        const { op: _, id, ...voice } = c;
        this.sink.start(id, voice);
        this.bySound[voice.sound] = (this.bySound[voice.sound] ?? 0) + 1;
      } else if (c.op === "set") {
        const { op: _, id, at, ...params } = c;
        this.sink.set(id, params, at);
      } else if (c.op === "stop") this.sink.stop(c.id, c.at, c.fade);
      else if (c.op === "duck") this.sink.duck(c.depth, c.glide, c.at);
      else this.sink.listen({ position: c.position, forward: c.forward }, c.at);
    }
  }

  /** The enemy faction's shout pool as far as it has loaded: the takes an
   *  unseen shout is drawn from (the sink plays them by `sound`). */
  shouts(takes: readonly { sound: string; duration: number }[]) {
    this.rust.shouts(JSON.stringify(takes));
  }

  /** Which set each unit kind speaks with: a shout heard in the open asks
   *  the voice lines for a bark in its kind's set. */
  speakers(speakers: Speakers) {
    this.rust.speakers(JSON.stringify(speakers));
  }

  /** Silence and forget everything (a new battle). */
  reset() {
    if (this.frame) this.run(this.frame.reset(this.sink.now()));
    this.bySound = {};
  }

  /** Take `presentation`'s last publication. */
  notePresented(presentation: Presentation) {
    this.run(this.rust.note(presentation, this.sink.now()));
  }

  /** Hear `presentation`'s last frame, from its camera, at wall time `wall` (seconds). */
  hear(presentation: Presentation, wall: number) {
    this.run(this.rust.hear(presentation, wall, this.sink.now()));
  }

  /** Take one scripted publication; the same tick again is ignored. */
  note(pub: SoundPublication) {
    this.run(this.rust.note_scripted(JSON.stringify(pub), this.sink.now()));
  }

  /** Advance to presentation time `clock` at wall time `wall` (seconds),
   *  hearing scripted `motion` at `listener`. */
  update(clock: number, wall: number, motion: SoundMotion, listener: Listener) {
    const soldiers = new Float64Array(motion.soldiers.length * 4);
    motion.soldiers.forEach((s, i) => {
      soldiers[i * 4] = s.id;
      soldiers[i * 4 + 1] = s.position[0];
      soldiers[i * 4 + 2] = s.position[1];
      soldiers[i * 4 + 3] = s.position[2];
    });
    const vehicles = new Float64Array(motion.vehicles.length * 8);
    motion.vehicles.forEach((v, i) => {
      vehicles.set(
        [v.key, v.position[0], v.position[1], v.position[2], v.travelL, v.travelR, v.turret],
        i * 8,
      );
      vehicles[i * 8 + 7] = v.reverse ? 1 : 0;
    });
    this.run(
      this.rust.update_scripted(
        clock,
        wall,
        this.sink.now(),
        soldiers,
        vehicles,
        motion.vehicles.map((v) => v.vehicleClass).join(" "),
        Float64Array.from([...listener.position, ...listener.forward]),
      ),
    );
  }

  /** Arbitrates `chatter`'s voice lines with the barks this frame heard,
   *  heard from where it last heard. */
  hearLines(chatter: Chatter): ChatterUpdate {
    return chatter.hear(this.rust, this.sink.now());
  }

  stats(): SoundStats {
    const bySound = { ...this.bySound };
    return this.frame
      ? { ...(JSON.parse(this.frame.stats()) as Omit<SoundStats, "bySound">), bySound }
      : {
          transients: 0,
          loops: 0,
          pending: 0,
          started: 0,
          dropped: 0,
          stolen: 0,
          positional: 0,
          cues: 0,
          held: false,
          tick: -1,
          bySound,
        };
  }
}

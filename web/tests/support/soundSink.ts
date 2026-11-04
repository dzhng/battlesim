import type { VoiceSink, VoiceSpec, VoiceParams } from "@packages/battle-audio/src/soundFrame";
/** A sink that keeps what it was told, on a clock the test sets. */
export class FakeSink implements VoiceSink {
  time = 0;
  readonly voices = new Map<number, { spec: VoiceSpec; end: number }>();
  readonly started: VoiceSpec[] = [];
  now() {
    return this.time;
  }
  duration() {
    return 0.5;
  }
  start(id: number, spec: VoiceSpec) {
    this.started.push(spec);
    // A voice plays its length at rate 1 over its rate, as a player plays it.
    this.voices.set(id, {
      spec,
      end: spec.loop ? Infinity : spec.at + this.duration() / spec.rate,
    });
  }
  set(id: number, p: VoiceParams) {
    const v = this.voices.get(id);
    if (v) v.spec = { ...v.spec, ...p };
  }
  stop(id: number) {
    this.voices.delete(id);
  }
  listen() {}
  readonly ducks: { depth: number; glide: number; at: number }[] = [];
  duck(depth: number, glide: number, at: number) {
    this.ducks.push({ depth, glide, at });
  }
  /** Voices sounding now. */
  live(loop?: boolean) {
    return [...this.voices.values()].filter(
      (v) => v.end > this.time && (loop === undefined || v.spec.loop === loop),
    );
  }
}

/** A named transient whose transport is controlled by the sink. */
export const testRecipe = (name: string) => ({
  label: name,
  clips: [],
  synth: "rifle",
  synth_gain: 1,
  gain: 1,
  loop: false,
});

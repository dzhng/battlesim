// A battle's voice lines: the takes of the sets it asks for (`VoiceBank`),
// the one arbitration (`Chatter`, in Rust), and playing what it starts.
// Packs load only once `start` is called (the battle's own sounds are
// ready), and each set once. A line plays dry and centred on the mixer's
// voices bus at the level Rust gives it, after its take's calibration: an
// answer is the owner's radio, never placed in the world.
import { sourceNormalization } from "../soundBank";
import type { SoundFrame } from "../soundFrame";
import { Chatter, type ChatterRules, type ChatterUpdate } from "./chatter";
import type { ResponseRequest } from "./responses";
import { VoiceBank, type PackSource } from "./voiceBank";

/** The mixer a battle's lines sound through, on its clock. */
export interface LineOutput {
  readonly context: BaseAudioContext;
  readonly buses: { readonly voices: AudioNode };
  now(): number;
}

export class BattleVoices {
  private readonly bank: VoiceBank;
  private readonly chatter: Chatter;
  private readonly wanted = new Set<string>();
  private readonly asked = new Set<string>();
  private started = false;
  private readonly sounding = new Set<AudioBufferSourceNode>();
  private readonly calibration = new WeakMap<AudioBuffer, number>();

  constructor(
    private readonly output: LineOutput,
    rules: ChatterRules,
    options: { fetcher?: typeof fetch; packs?: PackSource; seed?: number } = {},
  ) {
    this.chatter = new Chatter(rules, options.seed ?? 0);
    this.bank = new VoiceBank(output.context, options.fetcher, options.packs, {
      listener: this.chatter,
    });
  }

  /** Sets this battle speaks with; they load once started. */
  want(sets: Iterable<string>): Promise<void> {
    for (const set of sets) this.wanted.add(set);
    return this.load();
  }

  /** Starts loading every set wanted so far, and any wanted later. Resolves when those have loaded or failed. */
  start(): Promise<void> {
    this.started = true;
    return this.load();
  }

  /** Asks for a line now; the next arbitration decides whether it plays. */
  respond(request: ResponseRequest) {
    const now = this.output.now();
    this.chatter.request({ ...request, at: now }, now);
  }

  /** Starts what may start, heard from where battle sound `frame` last heard. */
  hear(frame: SoundFrame): ChatterUpdate {
    return this.play(frame.hearLines(this.chatter));
  }

  /** Starts what may start, heard from `listener` (a script's). */
  update(listener: readonly number[]): ChatterUpdate {
    return this.play(this.chatter.update(this.output.now(), listener));
  }

  /** Forgets every line waiting; lines sounding play out. */
  reset() {
    this.chatter.reset();
  }

  /** Sets, or takes, that failed to load, with why. */
  get errors(): ReadonlyMap<string, string> {
    return this.bank.errors;
  }

  dispose() {
    this.bank.dispose();
    for (const source of this.sounding) {
      source.onended = null;
      source.stop();
      source.disconnect();
    }
    this.sounding.clear();
  }

  private load(): Promise<void> {
    if (!this.started) return Promise.resolve();
    const sets = [...this.wanted].filter((set) => !this.asked.has(set));
    for (const set of sets) this.asked.add(set);
    return sets.length ? this.bank.load(sets) : Promise.resolve();
  }

  /** Carries out an arbitration's start commands. */
  private play(update: ChatterUpdate): ChatterUpdate {
    const { context, buses } = this.output;
    for (const line of update.commands) {
      const buffer = this.bank.line(line.sound, line.variant);
      if (!buffer) continue;
      let calibration = this.calibration.get(buffer);
      if (calibration === undefined) {
        calibration = sourceNormalization(buffer);
        this.calibration.set(buffer, calibration);
      }
      const source = new AudioBufferSourceNode(context, { buffer });
      const gain = new GainNode(context, { gain: line.gain * calibration });
      source.connect(gain).connect(buses.voices);
      source.onended = () => {
        this.sounding.delete(source);
        source.disconnect();
        gain.disconnect();
      };
      this.sounding.add(source);
      source.start(line.at);
    }
    return update;
  }
}

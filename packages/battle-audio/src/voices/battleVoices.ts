// A battle's voice lines: the takes of the sets it asks for (`VoiceBank`),
// the one arbitration (`Chatter`, in Rust), and playing what it starts.
// Packs load only once `start` is called (the battle's own sounds are
// ready), and each set once. A line plays on the mixer's voices bus as Rust
// gives it, after its take's calibration: an answer dry and centred, the
// owner's radio; a battle's bark where it was shouted.
import type { SoundFrame, VoiceSpec } from "../soundFrame";
import { Chatter, type ChatterRules, type ChatterUpdate } from "./chatter";
import type { ResponseRequest } from "./responses";
import { VoiceBank, type PackSource } from "./voiceBank";

/** The mixer a battle's lines sound through, on its clock (`WebAudioSink`). */
export interface LineOutput {
  readonly context: BaseAudioContext;
  now(): number;
  /** Plays a line of take `buffer`, calibrated as every take is. */
  line(voice: VoiceSpec, buffer: AudioBuffer): void;
}

export class BattleVoices {
  private readonly bank: VoiceBank;
  private readonly chatter: Chatter;
  private readonly wanted = new Set<string>();
  private readonly asked = new Set<string>();
  private started = false;
  /** When the last line started ends, on the player's clock. */
  private until = -Infinity;

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

  /** Starts what may start with the barks battle sound `frame` heard,
   *  heard from where it last heard. */
  hear(frame: SoundFrame): ChatterUpdate {
    return this.play(frame.hearLines(this.chatter));
  }

  /** Starts what may start, heard from `listener` (a script's). */
  update(listener: readonly number[]): ChatterUpdate {
    return this.play(this.chatter.update(this.output.now(), listener));
  }

  /** A line is sounding at player time `now`. */
  speaking(now = this.output.now()): boolean {
    return now < this.until;
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
  }

  private load(): Promise<void> {
    if (!this.started) return Promise.resolve();
    const sets = [...this.wanted].filter((set) => !this.asked.has(set));
    for (const set of sets) this.asked.add(set);
    return sets.length ? this.bank.load(sets) : Promise.resolve();
  }

  /** Carries out an arbitration's start commands. */
  private play(update: ChatterUpdate): ChatterUpdate {
    for (const { op: _, id: __, ...line } of update.commands) {
      const buffer = this.bank.line(line.sound, line.variant);
      if (buffer) this.output.line(line, buffer);
    }
    for (const line of update.started) this.until = Math.max(this.until, line.at + line.duration);
    return update;
  }
}

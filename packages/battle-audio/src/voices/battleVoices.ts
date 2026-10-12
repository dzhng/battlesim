// A battle's voice lines: the takes of the sets it asks for (`VoiceBank`),
// the one arbitration (`Chatter`), and where a line sounds. Packs load only
// once `start` is called (the battle's own sounds are ready), and each set
// once. A line plays dry and centred on the mixer's voices bus: an answer is
// the owner's radio, never placed in the world.
import { sourceNormalization } from "../loudness";
import { Chatter, type ChatterRules, type LineSink, type VoiceLine } from "./chatter";
import type { ResponseRequest } from "./responses";
import { VoiceBank, type PackSource } from "./voiceBank";

/** A line's level on the voices bus, after the calibration every take gets. */
export const LINE_GAIN = 0.5;

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
    this.bank = new VoiceBank(output.context, options.fetcher, options.packs);
    const sink: LineSink = { now: () => output.now(), play: (line) => this.play(line) };
    this.chatter = new Chatter(rules, this.bank, sink, options.seed ?? 0);
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

  /** Asks for a line now; the next `update` decides whether it plays. */
  respond(request: ResponseRequest) {
    this.chatter.request({ ...request, at: this.output.now() });
  }

  /** Starts what may start, heard from `listener`. */
  update(listener: readonly number[]) {
    return this.chatter.update(listener);
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

  private play(line: VoiceLine) {
    const { context, buses } = this.output;
    let calibration = this.calibration.get(line.buffer);
    if (calibration === undefined) {
      calibration = sourceNormalization(line.buffer);
      this.calibration.set(line.buffer, calibration);
    }
    const source = new AudioBufferSourceNode(context, { buffer: line.buffer });
    const gain = new GainNode(context, { gain: LINE_GAIN * calibration });
    source.connect(gain).connect(buses.voices);
    source.onended = () => {
      this.sounding.delete(source);
      source.disconnect();
      gain.disconnect();
    };
    this.sounding.add(source);
    source.start(line.at);
  }
}

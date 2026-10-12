// What an unseen enemy's shout can sound like: the generic pool of stressed
// barks from every voice set the enemy faction fields, never the speaker's own
// voice (the hearing cue says nothing of who shouted). This only loads the
// takes; which take a shout plays, where and how muffled, is Rust's
// (`presentation::sound::frame`). A set without a pack adds nothing, so an
// empty pool is a silent shout, never an error.
import { VoiceBank, type PackSource } from "./voiceBank";
import { BARK_TRIGGERS, type PackTake, type VoicePack } from "./voiceCatalog";

/** One loaded take of the pool, as the sound frame plays it. */
export interface ShoutTake {
  sound: string;
  buffer: AudioBuffer;
}

/** A pack's share of the pool: up to `perVoice` stressed barks from each
 *  voice that shouts, every trigger's first take before any second. */
export function shoutTakes(pack: VoicePack, perVoice: number): PackTake[] {
  const shouting = new Set(pack.voices.filter((v) => v.speaks !== "responses").map((v) => v.id));
  const barks: readonly string[] = BARK_TRIGGERS;
  const order = (t: PackTake) => t.take * barks.length + barks.indexOf(t.trigger);
  const pool = pack.takes
    .filter((t) => shouting.has(t.voice) && t.intensity === "stressed" && barks.includes(t.trigger))
    .sort((a, b) => order(a) - order(b));
  return [...shouting].flatMap((voice) => pool.filter((t) => t.voice === voice).slice(0, perVoice));
}

/** The enemy faction's pool, loaded once for one audio context. */
export class ShoutPool {
  private readonly bank: VoiceBank;

  constructor(
    context: BaseAudioContext,
    perVoice: number,
    options: { fetcher?: typeof fetch; packs?: PackSource } = {},
  ) {
    this.bank = new VoiceBank(context, options.fetcher, options.packs, {
      select: (pack) => shoutTakes(pack, perVoice),
    });
  }

  /** Loads `sets`' share of the pool; resolves with every take loaded, never rejects. */
  async load(sets: readonly string[]): Promise<ShoutTake[]> {
    await this.bank.load(sets);
    return this.bank.loaded().map(([sound, buffer]) => ({ sound, buffer }));
  }

  dispose() {
    this.bank.dispose();
  }
}

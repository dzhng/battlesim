// A battle's voice takes, decoded for one audio context. Separate from the
// sound bank, which knows only `sounds.json` clip ids: voices load by URL from
// each set's `pack.json`, in the background, and never hold up the battle's
// readiness. Until a take is decoded it is absent, and a line asking for it is
// dropped rather than waited for: the bank tells its listener (the line
// arbitration, `Chatter`) each set's voices and each take as they arrive.
import type { Intensity, PackTake, Trigger, VoicePack } from "./voiceCatalog";
import { packUrl } from "./voiceCatalog";

/** A take's name in the bank: `set/voice/trigger/intensity/take`. */
const key = (set: string, voice: string, trigger: Trigger, intensity: Intensity, take: number) =>
  `${set}/${voice}/${trigger}/${intensity}/${take}`;

/** Where a set's pack comes from; the battle reads the served `pack.json`. */
export type PackSource = (set: string, signal: AbortSignal) => Promise<VoicePack>;

/** The served pack of a set, through `fetcher`. */
export const servedPacks =
  (fetcher: typeof fetch = fetch): PackSource =>
  async (set, signal) => {
    const response = await fetcher(packUrl(set), { signal });
    if (!response.ok) throw new Error(`${packUrl(set)}: HTTP ${response.status}`);
    return (await response.json()) as VoicePack;
  };

/** Told what the bank holds as it arrives: a set's voices, then each decoded take. */
export interface BankListener {
  voices(set: string, voices: VoicePack["voices"]): void;
  loaded(
    set: string,
    voice: string,
    trigger: Trigger,
    intensity: Intensity,
    take: number,
    duration: number,
  ): void;
}

export class VoiceBank {
  private readonly buffers = new Map<string, AudioBuffer>();
  /** Decoded takes by URL, so a take is fetched once however often it is asked for. */
  private readonly decoded = new Map<string, Promise<AudioBuffer>>();
  private readonly controller = new AbortController();
  /** Sets, or takes, that failed to load, with why. They stay silent. */
  readonly errors = new Map<string, string>();

  constructor(
    private readonly context: BaseAudioContext,
    private readonly fetcher: typeof fetch = fetch,
    private readonly source: PackSource = servedPacks(fetcher),
    private readonly options: {
      /** Told each set's voices and each take as it arrives (the line arbitration). */
      listener?: BankListener;
      /** Which of a pack's takes to load: by default, all of them. */
      select?: (pack: VoicePack) => readonly PackTake[];
    } = {},
  ) {}

  /** Loads `sets` in the background. Resolves when every set has loaded or failed; never rejects. */
  async load(sets: readonly string[], signal?: AbortSignal): Promise<void> {
    const admitted = signal
      ? AbortSignal.any([signal, this.controller.signal])
      : this.controller.signal;
    const failed = (what: string, error: unknown) => {
      if (!admitted.aborted)
        this.errors.set(what, error instanceof Error ? error.message : String(error));
    };
    await Promise.all(
      sets.map(async (set) => {
        let pack: VoicePack;
        try {
          pack = await this.source(set, admitted);
        } catch (error) {
          return failed(set, error);
        }
        if (admitted.aborted) return;
        this.options.listener?.voices(set, pack.voices);
        await Promise.all(
          (this.options.select?.(pack) ?? pack.takes).map(async (take) => {
            const id = key(set, take.voice, take.trigger, take.intensity, take.take);
            try {
              const buffer = await this.decode(take.url, admitted);
              if (admitted.aborted) return;
              this.buffers.set(id, buffer);
              this.options.listener?.loaded(
                set,
                take.voice,
                take.trigger,
                take.intensity,
                take.take,
                buffer.duration,
              );
            } catch (error) {
              failed(id, error);
            }
          }),
        );
      }),
    );
  }

  /** The audio a line's start command names: take `variant` of `sound`
   *  (`set/voice/trigger/intensity`), or null when it isn't loaded. */
  line(sound: string, variant: number): AudioBuffer | null {
    return this.buffers.get(`${sound}/${variant}`) ?? null;
  }

  /** Every take loaded so far, by its name (`set/voice/trigger/intensity/take`). */
  loaded(): [string, AudioBuffer][] {
    return [...this.buffers];
  }

  dispose() {
    this.controller.abort();
    this.buffers.clear();
    this.decoded.clear();
  }

  private decode(url: string, signal: AbortSignal): Promise<AudioBuffer> {
    let decoded = this.decoded.get(url);
    if (!decoded) {
      // Called unbound: a browser's fetch refuses any other receiver.
      const fetcher = this.fetcher;
      decoded = (async () => {
        const response = await fetcher(url, { signal });
        if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
        return this.context.decodeAudioData(await response.arrayBuffer());
      })();
      decoded.catch(() => this.decoded.delete(url));
      this.decoded.set(url, decoded);
    }
    return decoded;
  }
}

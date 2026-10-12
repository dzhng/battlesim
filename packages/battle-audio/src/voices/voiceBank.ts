// A battle's voice takes, for one audio context. Separate from the sound bank,
// which knows only `sounds.json` clip ids: voices load by URL from each set's
// `pack.json`, in the background, and never hold up the battle's readiness.
// Until a take is fetched it is absent, and a line asking for it is dropped
// rather than waited for: the bank tells its listener (the line arbitration,
// `Chatter`) each set's voices and each take as it arrives, with the length
// its pack records. A take is held as fetched (MP3, about a twentieth of its
// decoded size) and decoded only when a line plays it; the last few decoded
// stay decoded.
import type { Intensity, PackTake, Trigger, VoicePack } from "./voiceCatalog";
import { packUrl } from "./voiceCatalog";

/** A take's name in the bank: `set/voice/trigger/intensity/take`. */
const key = (set: string, voice: string, trigger: Trigger, intensity: Intensity, take: number) =>
  `${set}/${voice}/${trigger}/${intensity}/${take}`;

/** Takes fetched at once. A skirmish asks for hundreds, and a browser refuses
 *  requests past a few hundred outstanding (`ERR_INSUFFICIENT_RESOURCES`), the
 *  battle's own downloads with them. */
export const TAKES_IN_FLIGHT = 8;

/** Takes kept decoded, the most recently played: a few lines sound at once,
 *  and a unit's next line is often one just heard. */
export const DECODED_TAKES = 32;

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

/** Told what the bank holds as it arrives: a set's voices, then each take. */
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
  /** Each loaded take's MP3 bytes, by name. */
  private readonly bytes = new Map<string, ArrayBuffer>();
  /** Each take's download by URL, so a take is fetched once however often it is asked for. */
  private readonly fetched = new Map<string, Promise<ArrayBuffer>>();
  /** The takes kept decoded, least recently played first. */
  private readonly decoded = new Map<string, Promise<AudioBuffer>>();
  /** Lines waiting for their take to decode. */
  private readonly lines = new Set<Promise<void>>();
  private readonly controller = new AbortController();
  /** Sets, or takes, that failed to load, with why. They stay silent. */
  readonly errors = new Map<string, string>();
  /** Free download slots, and the takes waiting for one in the order asked. */
  private free = TAKES_IN_FLIGHT;
  private readonly waiting: (() => void)[] = [];

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
              const bytes = await this.fetch(take.url, admitted);
              if (admitted.aborted) return;
              this.bytes.set(id, bytes);
              this.options.listener?.loaded(
                set,
                take.voice,
                take.trigger,
                take.intensity,
                take.take,
                take.duration_s,
              );
            } catch (error) {
              failed(id, error);
            }
          }),
        );
      }),
    );
  }

  /** Every take loaded so far, by name (`set/voice/trigger/intensity/take`). */
  names(): string[] {
    return [...this.bytes.keys()];
  }

  /** Take `name`'s audio, decoded now or kept from a recent line; null when it isn't loaded. */
  decode(name: string): Promise<AudioBuffer> | null {
    const decoded = this.decoded.get(name) ?? this.decodeFresh(name);
    if (!decoded) return null;
    // Most recently played last; past the limit, the least recent goes.
    this.decoded.delete(name);
    this.decoded.set(name, decoded);
    for (const oldest of this.decoded.keys()) {
      if (this.decoded.size <= DECODED_TAKES) break;
      this.decoded.delete(oldest);
    }
    return decoded;
  }

  /** Plays the line a start command names, take `variant` of `sound`
   *  (`set/voice/trigger/intensity`), through `play` once its take is decoded:
   *  at `at`, or when decoded if that is later (`now`), unless that is more
   *  than `wait` seconds past `at`, when it is dropped. False when the take
   *  isn't loaded. */
  play(
    sound: string,
    variant: number,
    at: number,
    wait: number,
    now: () => number,
    play: (buffer: AudioBuffer, at: number) => void,
  ): boolean {
    const decoded = this.decode(`${sound}/${variant}`);
    if (!decoded) return false;
    const line: Promise<void> = decoded.then(
      (buffer) => {
        this.lines.delete(line);
        const time = now();
        if (!this.controller.signal.aborted && time - at <= wait) play(buffer, Math.max(at, time));
      },
      (error: unknown) => {
        this.lines.delete(line);
        if (!this.controller.signal.aborted)
          this.errors.set(
            `${sound}/${variant}`,
            error instanceof Error ? error.message : String(error),
          );
      },
    );
    this.lines.add(line);
    return true;
  }

  /** Resolves once every decode asked for so far has finished, and its line played or dropped. */
  async settled(): Promise<void> {
    while (this.lines.size) await Promise.all(this.lines);
  }

  /** What the bank holds: takes loaded, their MP3 bytes, and takes kept decoded. */
  held(): { takes: number; bytes: number; decoded: number } {
    let bytes = 0;
    for (const take of this.bytes.values()) bytes += take.byteLength;
    return { takes: this.bytes.size, bytes, decoded: this.decoded.size };
  }

  dispose() {
    this.controller.abort();
    this.bytes.clear();
    this.fetched.clear();
    this.decoded.clear();
  }

  private decodeFresh(name: string): Promise<AudioBuffer> | null {
    const bytes = this.bytes.get(name);
    if (!bytes) return null;
    // A decode takes its bytes; the bank keeps its own.
    const decoded = this.context.decodeAudioData(bytes.slice(0));
    decoded.catch(() => {
      if (this.decoded.get(name) === decoded) this.decoded.delete(name);
    });
    return decoded;
  }

  private fetch(url: string, signal: AbortSignal): Promise<ArrayBuffer> {
    let fetched = this.fetched.get(url);
    if (!fetched) {
      // Called unbound: a browser's fetch refuses any other receiver.
      const fetcher = this.fetcher;
      fetched = (async () => {
        if (this.free > 0) this.free--;
        else await new Promise<void>((resolve) => this.waiting.push(resolve));
        try {
          signal.throwIfAborted();
          const response = await fetcher(url, { signal });
          if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
          return await response.arrayBuffer();
        } finally {
          // A finished take hands its slot straight to the next waiting one.
          const next = this.waiting.shift();
          if (next) next();
          else this.free++;
        }
      })();
      fetched.catch(() => this.fetched.delete(url));
      this.fetched.set(url, fetched);
    }
    return fetched;
  }
}

// A narrator's lines, for one audio context: its pack fetched once, each
// take when first asked for, decoded when it plays. The coach of an authored
// scenario says a line; it plays once, unplaced, on the voices bus, and a
// line asked for while another speaks replaces it, as a radio does.
import type { VoiceSpec } from "../soundFrame";
import { narrationPackUrl, type NarrationPack } from "./voiceCatalog";

export interface NarrationOutput {
  readonly context: BaseAudioContext;
  now(): number;
  line(voice: VoiceSpec, buffer: AudioBuffer): void;
  /** Stops every line playing: the one a newer line replaces. */
  clearLines?(): void;
}

export class Narration {
  private readonly packs = new Map<string, Promise<NarrationPack>>();
  private readonly takes = new Map<string, Promise<AudioBuffer>>();
  /** The line asked for last; an older one still loading never plays. */
  private latest = 0;

  constructor(
    private readonly output: NarrationOutput,
    private readonly fetcher: typeof fetch = (input, init) => fetch(input, init),
  ) {}

  /** Speak `speaker`'s `line`; resolves to its length in seconds, or null
   *  when the speaker has no judged take of it (the coach carries on). */
  async say(speaker: string, line: string): Promise<number | null> {
    const asked = ++this.latest;
    const take = (await this.pack(speaker)).lines.find((l) => l.line === line);
    if (!take) return null;
    let buffer = this.takes.get(take.url);
    if (!buffer) {
      buffer = this.fetcher(take.url)
        .then((r) => r.arrayBuffer())
        .then((bytes) => this.output.context.decodeAudioData(bytes));
      this.takes.set(take.url, buffer);
    }
    const audio = await buffer;
    if (asked !== this.latest) return null;
    this.output.clearLines?.();
    this.output.line(
      {
        sound: take.url,
        variant: 0,
        bus: "voices",
        at: this.output.now(),
        gain: 1,
        rate: 1,
        lowpass: 20000,
        wet: 0,
        attack: 0,
        position: null,
        pan: null,
        loop: false,
      },
      audio,
    );
    return take.duration;
  }

  private pack(speaker: string): Promise<NarrationPack> {
    let pack = this.packs.get(speaker);
    if (!pack) {
      pack = this.fetcher(narrationPackUrl(speaker)).then((r) =>
        r.ok ? (r.json() as Promise<NarrationPack>) : { speaker, lines: [] },
      );
      this.packs.set(speaker, pack);
    }
    return pack;
  }
}

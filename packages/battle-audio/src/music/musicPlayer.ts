// Music streams from a media element into the Web Audio graph: each playing
// track is an element → its source node → the track's gain (fades, its
// catalog gain), and every track sums into one level gain (the player's music
// setting) and then one duck gain (dips under voice lines) before the
// destination. Level and duck are separate nodes so neither overwrites the
// other. Outside `WebAudioSink`: `MediaElementAudioSourceNode` has no
// `OfflineAudioContext` form.
import { musicUrl, type MusicTrack } from "./musicCatalog";

interface Playing {
  media: HTMLMediaElement;
  source: MediaElementAudioSourceNode;
  gain: GainNode;
}

export class MusicPlayer {
  /** The music setting × the fixture's music bus. */
  readonly level: GainNode;
  /** 1 − the current duck depth. */
  readonly ducking: GainNode;
  private readonly playing = new Set<Playing>();

  /** `destination` is the master the rest of the mix shares, so master volume covers music. */
  constructor(
    private readonly context: AudioContext,
    private readonly tracks: Record<string, MusicTrack>,
    destination: AudioNode,
  ) {
    this.level = new GainNode(context, { gain: 1 });
    this.ducking = new GainNode(context, { gain: 1 });
    this.level.connect(this.ducking).connect(destination);
  }

  setLevel(level: number) {
    this.level.gain.setTargetAtTime(level, this.context.currentTime, 0.02);
  }

  /** Starts `trackId` looping, fading in over `fade` seconds. */
  play(trackId: string, { fade = 0 }: { fade?: number } = {}): Playing {
    const track = this.tracks[trackId];
    if (!track) throw new Error(`Unknown music track ${trackId}`);
    // One element per play: a source node binds its element for life, and a
    // finished track releases both (`release`), so nothing accumulates.
    const media = new Audio();
    media.loop = true;
    media.preload = "auto";
    media.src = musicUrl(trackId, track);
    const source = new MediaElementAudioSourceNode(this.context, { mediaElement: media });
    const now = this.context.currentTime;
    const gain = new GainNode(this.context, { gain: fade > 0 ? 0 : track.gain });
    if (fade > 0) {
      gain.gain.setValueAtTime(0, now);
      gain.gain.linearRampToValueAtTime(track.gain, now + fade);
    }
    source.connect(gain).connect(this.level);
    const item = { media, source, gain };
    this.playing.add(item);
    this.resume();
    return item;
  }

  /** Retries playback autoplay refused; call it from inside the first gesture. */
  resume() {
    for (const { media } of this.playing) if (media.paused) void media.play()?.catch(() => {});
  }

  /** Fades every playing track out and `trackId` in over the same `seconds`. */
  crossfadeTo(trackId: string, seconds: number) {
    const outgoing = [...this.playing];
    const next = this.play(trackId, { fade: seconds });
    for (const item of outgoing) this.fadeOut(item, seconds);
    return next;
  }

  /** Dips the music by `depth` (0..1) over `attack`, or with depth 0 restores it over `release`. */
  duck(depth: number, attack: number, release: number) {
    const now = this.context.currentTime;
    const d = Math.min(1, Math.max(0, depth));
    this.ducking.gain.cancelScheduledValues(now);
    this.ducking.gain.setTargetAtTime(1 - d, now, Math.max(0.001, d > 0 ? attack : release));
  }

  stop(fade = 0) {
    for (const item of this.playing) this.fadeOut(item, fade);
  }

  dispose() {
    for (const item of this.playing) this.release(item);
    this.ducking.disconnect();
    this.level.disconnect();
  }

  private fadeOut(item: Playing, seconds: number) {
    if (seconds <= 0) return this.release(item);
    const now = this.context.currentTime;
    item.gain.gain.cancelScheduledValues(now);
    item.gain.gain.setValueAtTime(item.gain.gain.value, now);
    item.gain.gain.linearRampToValueAtTime(0, now + seconds);
    setTimeout(() => this.release(item), seconds * 1000);
  }

  private release(item: Playing) {
    if (!this.playing.delete(item)) return;
    item.media.pause();
    // Dropping the source ends the element's network stream.
    item.media.removeAttribute("src");
    item.media.load();
    item.source.disconnect();
    item.gain.disconnect();
  }
}

// The Web Audio graph `SoundFrame` drives: one voice is a buffer source
// through its gain (the frame's level at the listener, its onset ramped
// by the frame's attack), the air's low-pass, and a panner (equal-power, by
// direction only: the frame owns distance) or, for a hearing cue, a stereo
// pan; voices sum into their bus, and each also sends (by the frame's wet)
// into its bus's distance reverb, whose tail joins the bus; the buses sum
// into the master, and the master through a limiter to the output. The
// listener is the camera's (`Listener`). Works on an `AudioContext` or an
// `OfflineAudioContext` alike: every change is scheduled at a sink time.
import { BUSES, type AudioPresentation, type Bus } from "./audioPresentation";
import type { Listener, VoiceParams, VoiceSink, VoiceSpec } from "./soundFrame";
import { reverbImpulse } from "./synth";
import { SoundBank, sourceNormalization } from "./soundBank";
import type { SoundCatalog } from "./catalog";

/** Seconds a live loop's parameters glide to a new value over (time constant). */
const GLIDE_S = 0.06;

interface LiveVoice {
  normalization: number;
  source: AudioBufferSourceNode;
  gain: GainNode;
  filter: BiquadFilterNode;
  panner: PannerNode | null;
  stereo: StereoPannerNode | null;
  release(): void;
  send: GainNode;
}

export class WebAudioSink implements VoiceSink {
  readonly bank: SoundBank;
  readonly master: GainNode;
  readonly buses: Record<Bus, GainNode>;
  /** Each bus's distance-reverb input: voices send here by their wet. */
  private readonly reverbs: Record<Bus, GainNode>;
  private readonly voices = new Map<number, LiveVoice>();
  /** Voice takes played by name (the enemy's shout pool). */
  private readonly takes = new Map<string, AudioBuffer>();
  /** Each voice take's calibration, measured once. */
  private readonly calibrations = new WeakMap<AudioBuffer, number>();
  private readonly sounding = new Set<LiveVoice>();
  private readonly graph: AudioNode[] = [];
  private readonly ownsBank: boolean;

  constructor(
    readonly context: BaseAudioContext,
    presentation: AudioPresentation,
    catalog: SoundCatalog,
    bank?: SoundBank,
  ) {
    this.ownsBank = !bank;
    this.bank = bank ?? new SoundBank(context, catalog);
    const limiter = new DynamicsCompressorNode(context, {
      threshold: -6,
      knee: 3,
      ratio: 20,
      attack: 0.002,
      release: 0.2,
    });
    this.graph.push(limiter);
    limiter.connect(context.destination);
    this.master = new GainNode(context, { gain: presentation.buses.master });
    this.graph.push(this.master);
    this.master.connect(limiter);
    this.buses = Object.fromEntries(
      BUSES.map((b) => {
        const g = new GainNode(context, { gain: presentation.buses[b] });
        this.graph.push(g);
        g.connect(this.master);
        return [b, g];
      }),
    ) as Record<Bus, GainNode>;
    const ir = reverbImpulse(presentation.air.reverb_s, context.sampleRate);
    const impulse = context.createBuffer(2, ir[0].length, context.sampleRate);
    ir.forEach((c, i) => impulse.copyToChannel(c as Float32Array<ArrayBuffer>, i));
    this.reverbs = Object.fromEntries(
      BUSES.map((b) => {
        const input = new GainNode(context, { gain: 1 });
        const convolver = new ConvolverNode(context, {
          buffer: impulse,
          disableNormalization: true,
        });
        this.graph.push(input, convolver);
        input.connect(convolver).connect(this.buses[b]);
        return [b, input];
      }),
    ) as Record<Bus, GainNode>;
  }

  now() {
    return this.context.currentTime;
  }

  duration(sound: string, variant = 0) {
    return this.bank.get(sound, variant).duration;
  }

  /** Voice takes the frame may start by name, calibrated as every take is. */
  addTakes(takes: Iterable<{ sound: string; buffer: AudioBuffer }>) {
    for (const { sound, buffer } of takes) if (!this.takes.has(sound)) this.takes.set(sound, buffer);
  }

  start(id: number, v: VoiceSpec) {
    const take = this.takes.get(v.sound);
    const buffer = take ?? this.bank.get(v.sound, v.variant);
    const normalization = take ? this.calibration(take) : this.bank.normalizationGain(buffer);
    this.voices.set(id, this.play(v, buffer, normalization, id));
  }

  /** A voice line (`Chatter`'s start command) of take `buffer`, calibrated
   *  as every take is. It plays out: nothing glides or stops it but `clear`. */
  line(v: VoiceSpec, buffer: AudioBuffer) {
    this.play(v, buffer, this.calibration(buffer), null);
  }

  private calibration(take: AudioBuffer): number {
    let gain = this.calibrations.get(take);
    if (gain === undefined) {
      gain = sourceNormalization(take);
      this.calibrations.set(take, gain);
    }
    return gain;
  }

  /** One voice's graph, started; voice `id`'s when it has one. */
  private play(
    v: VoiceSpec,
    buffer: AudioBuffer,
    normalization: number,
    id: number | null,
  ): LiveVoice {
    const ctx = this.context;
    const level = v.gain * normalization;
    const source = new AudioBufferSourceNode(ctx, {
      buffer,
      loop: v.loop,
      playbackRate: v.rate,
    });
    const gain = new GainNode(ctx, { gain: level });
    if (v.attack > 0) {
      // A far sound's onset arrives softened: ramp in rather than snap.
      gain.gain.setValueAtTime(0, v.at);
      gain.gain.linearRampToValueAtTime(level, v.at + v.attack);
    }
    const filter = new BiquadFilterNode(ctx, { type: "lowpass", frequency: v.lowpass, Q: 0.5 });
    source.connect(gain).connect(filter);
    let panner: PannerNode | null = null;
    let stereo: StereoPannerNode | null = null;
    let out: AudioNode = filter;
    if (v.position) {
      panner = new PannerNode(ctx, {
        panningModel: "equalpower",
        rolloffFactor: 0,
        positionX: v.position[0],
        positionY: v.position[1],
        positionZ: v.position[2],
      });
      out = filter.connect(panner);
    } else if (v.pan !== null) {
      stereo = new StereoPannerNode(ctx, { pan: v.pan });
      out = filter.connect(stereo);
    }
    out.connect(this.buses[v.bus]);
    const send = new GainNode(ctx, { gain: v.wet });
    out.connect(send).connect(this.reverbs[v.bus]);
    // Stagger repeated ambience/engine loops; authored music starts at its intro.
    const offset =
      v.offset ?? (v.loop && id !== null ? ((id * 0.6180339887) % 1) * buffer.duration : 0);
    source.start(v.at, offset);
    const voice: LiveVoice = {
      source,
      gain,
      filter,
      panner,
      stereo,
      send,
      normalization,
      release: () => {
        if (id !== null && this.voices.get(id) === voice) this.voices.delete(id);
        this.sounding.delete(voice);
        source.onended = null;
        for (const node of [source, gain, filter, panner, stereo, send]) node?.disconnect();
      },
    };
    this.sounding.add(voice);
    source.onended = voice.release;
    return voice;
  }

  set(id: number, p: VoiceParams, at: number) {
    const v = this.voices.get(id);
    if (!v) return;
    v.gain.gain.setTargetAtTime(p.gain * v.normalization, at, GLIDE_S);
    v.source.playbackRate.setTargetAtTime(p.rate, at, GLIDE_S);
    v.filter.frequency.setTargetAtTime(p.lowpass, at, GLIDE_S);
    v.send.gain.setTargetAtTime(p.wet, at, GLIDE_S);
    if (v.panner && p.position) {
      v.panner.positionX.setValueAtTime(p.position[0], at);
      v.panner.positionY.setValueAtTime(p.position[1], at);
      v.panner.positionZ.setValueAtTime(p.position[2], at);
    }
  }

  stop(id: number, at: number, fade: number) {
    const v = this.voices.get(id);
    if (!v) return;
    this.voices.delete(id);
    v.gain.gain.cancelScheduledValues(at);
    if (fade > 0) v.gain.gain.setTargetAtTime(0, at, fade / 3);
    else v.gain.gain.setValueAtTime(0, at);
    try {
      v.source.stop(at + fade);
    } catch {
      // Already stopped.
    }
    if (fade === 0) v.release();
  }

  listen(l: Listener, at: number) {
    const L = this.context.listener;
    L.positionX.setValueAtTime(l.position[0], at);
    L.positionY.setValueAtTime(l.position[1], at);
    L.positionZ.setValueAtTime(l.position[2], at);
    L.forwardX.setValueAtTime(l.forward[0], at);
    L.forwardY.setValueAtTime(l.forward[1], at);
    L.forwardZ.setValueAtTime(l.forward[2], at);
    L.upX.setValueAtTime(0, at);
    L.upY.setValueAtTime(0, at);
    L.upZ.setValueAtTime(1, at);
  }

  /** Remove every voice, including sources still fading after a stop. */
  clear() {
    for (const voice of this.sounding) {
      try {
        voice.source.stop();
      } catch {
        /* Already stopped. */
      }
      voice.release();
    }
  }

  /** Cut the mixer output too, so reverb tails cannot outlive a battle.
   * A borrowed bank belongs to the page and outlives this mixer. */
  dispose() {
    this.clear();
    for (const node of this.graph) node.disconnect();
    this.graph.length = 0;
    if (this.ownsBank) this.bank.dispose();
  }

  /** Voices the graph holds now. */
  get live() {
    return this.sounding.size;
  }
}

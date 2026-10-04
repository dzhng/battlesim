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
import { SoundBank } from "./soundBank";
import type { SoundCatalog } from "./catalog";

/** Seconds a live loop's parameters glide to a new value over (time constant). */
const GLIDE_S = 0.06;

interface LiveVoice {
  normalization: number;
  source: AudioBufferSourceNode;
  gain: GainNode;
  filter: BiquadFilterNode;
  panner: PannerNode | null;
  send: GainNode;
}

export class WebAudioSink implements VoiceSink {
  readonly bank: SoundBank;
  readonly master: GainNode;
  readonly buses: Record<Bus, GainNode>;
  /** Each bus's distance-reverb input: voices send here by their wet. */
  private readonly reverbs: Record<Bus, GainNode>;
  private readonly voices = new Map<number, LiveVoice>();

  constructor(
    readonly context: BaseAudioContext,
    presentation: AudioPresentation,
    catalog?: SoundCatalog,
  ) {
    this.bank = new SoundBank(context, catalog);
    const limiter = new DynamicsCompressorNode(context, {
      threshold: -6,
      knee: 3,
      ratio: 20,
      attack: 0.002,
      release: 0.2,
    });
    limiter.connect(context.destination);
    this.master = new GainNode(context, { gain: presentation.buses.master });
    this.master.connect(limiter);
    this.buses = Object.fromEntries(
      BUSES.map((b) => {
        const g = new GainNode(context, { gain: presentation.buses[b] });
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
        input
          .connect(new ConvolverNode(context, { buffer: impulse, disableNormalization: true }))
          .connect(this.buses[b]);
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

  start(id: number, v: VoiceSpec) {
    const ctx = this.context;
    const buffer = this.bank.get(v.sound, v.variant);
    const normalization = this.bank.normalizationGain(buffer);
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
    } else if (v.pan !== null) out = filter.connect(new StereoPannerNode(ctx, { pan: v.pan }));
    out.connect(this.buses[v.bus]);
    const send = new GainNode(ctx, { gain: v.wet });
    out.connect(send).connect(this.reverbs[v.bus]);
    // A loop starts part-way in, so two of the same sound never phase.
    const offset = v.loop ? (id * 0.6180339887) % 1 : 0;
    source.start(v.at, offset * source.buffer!.duration);
    const voice = { source, gain, filter, panner, send, normalization };
    this.voices.set(id, voice);
    source.onended = () => {
      if (this.voices.get(id) === voice) this.voices.delete(id);
      source.disconnect();
      filter.disconnect();
      panner?.disconnect();
      send.disconnect();
    };
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
    v.gain.gain.setTargetAtTime(0, at, fade / 3);
    try {
      v.source.stop(at + fade);
    } catch {
      // Already stopped.
    }
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

  /** Voices the graph holds now. */
  get live() {
    return this.voices.size;
  }
}

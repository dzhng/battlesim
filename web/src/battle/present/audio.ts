/** Sound presentation for one side's cues. Each cue becomes a short locally
 * synthesized placeholder, panned by its direction sector relative to the
 * camera and scaled by its near/far band, plus a caption carrying exactly the
 * same information. The camera changes only the stereo mix, never what is
 * heard: cues come from the simulation's listeners. */
import type { SoundCueView } from "../sim/observation";

const DIRECTIONS = [
  "east",
  "north-east",
  "north",
  "north-west",
  "west",
  "south-west",
  "south",
  "south-east",
];

export interface Caption {
  tick: number;
  text: string;
}

export function describeCue(cue: SoundCueView, listenerName: string): string {
  const what =
    cue.category === "shot"
      ? "gunfire"
      : cue.category === "vehicle"
        ? cue.moving
          ? "engine, moving"
          : "engine, idling"
        : cue.moving
          ? "footsteps"
          : "voices";
  return `Heard ${what}, ${cue.band}, ${DIRECTIONS[cue.sector]} of ${listenerName}`;
}

/** Stereo position of a sector as heard by a camera looking along `cameraYaw`
 *  (camera3d convention: the eye sits at `yaw` around its target). */
export function sectorPan(sector: number, cameraYaw: number): number {
  const a = (sector * Math.PI) / 4;
  // Screen-right in world space for this camera.
  const right = [-Math.sin(cameraYaw), Math.cos(cameraYaw)];
  return Math.max(-1, Math.min(1, Math.cos(a) * right[0] + Math.sin(a) * right[1]));
}

export class CueAudio {
  private context: AudioContext | null = null;
  /** Sounds scheduled so far (diagnostic; audible or not). */
  scheduled = 0;

  /** Audio needs a user gesture to start; call from one. */
  enable() {
    this.context ??= new AudioContext();
    void this.context.resume();
  }

  get enabled() {
    return this.context !== null;
  }

  play(cue: SoundCueView, cameraYaw: number) {
    this.scheduled++;
    const ctx = this.context;
    if (!ctx) return;
    const now = ctx.currentTime;
    const gain = ctx.createGain();
    const level = cue.band === "near" ? 0.35 : 0.12;
    const pan = ctx.createStereoPanner();
    pan.pan.value = sectorPan(cue.sector, cameraYaw);
    gain.connect(pan).connect(ctx.destination);
    if (cue.category === "vehicle") {
      const osc = ctx.createOscillator();
      const low = ctx.createBiquadFilter();
      osc.type = "sawtooth";
      osc.frequency.value = cue.moving ? 72 : 48;
      low.type = "lowpass";
      low.frequency.value = cue.moving ? 420 : 260;
      gain.gain.setValueAtTime(0, now);
      gain.gain.linearRampToValueAtTime(level, now + 0.08);
      gain.gain.linearRampToValueAtTime(0, now + 0.45);
      osc.connect(low).connect(gain);
      osc.start(now);
      osc.stop(now + 0.5);
      return;
    }
    // Infantry and gunfire are shaped noise: footfalls, or one sharp crack.
    const bursts = cue.category === "shot" ? 1 : cue.moving ? 3 : 1;
    for (let k = 0; k < bursts; k++) {
      const t = now + k * 0.14;
      const length = cue.category === "shot" ? 0.12 : 0.05;
      const buffer = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * length), ctx.sampleRate);
      const data = buffer.getChannelData(0);
      for (let i = 0; i < data.length; i++)
        data[i] = (Math.random() * 2 - 1) * (1 - i / data.length) ** 3;
      const source = ctx.createBufferSource();
      const band = ctx.createBiquadFilter();
      band.type = "bandpass";
      band.frequency.value = cue.category === "shot" ? 1800 : 600;
      source.buffer = buffer;
      source.connect(band).connect(gain);
      gain.gain.setValueAtTime(cue.category === "shot" ? level * 1.6 : level * 0.6, now);
      source.start(t);
    }
  }

  dispose() {
    void this.context?.close();
    this.context = null;
  }
}

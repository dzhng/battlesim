// A battle's sound in the browser: the `SoundFrame` over a live
// `AudioContext`, heard from the camera. Audio starts on the first user
// gesture (the browser's rule), or at once when the page has already had
// one (the click that opened the battle, such as the benchmark's Short
// run), unless muted, once the bank is prepared; muting suspends the context
// and the frame, and unmuting starts afresh (loops restart, nothing stale
// plays). The mute and volume are `soundSettings`', shared with the menu.
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { vec3 } from "math";
import {
  SoundFrame,
  type Listener,
  type SoundFrameOptions,
  type SoundMotion,
  type SoundPublication,
  type SoundStats,
} from "./soundFrame";
import { soundSettings } from "./settings";
import { WebAudioSink } from "./webAudioSink";

/** Where the listener stands for a camera: `share` of the way from the
 *  point looked at to the eye, facing the way the camera looks (level). */
export function cameraListener(
  camera: Pick<Camera3DParams, "target" | "distance" | "pitch" | "yaw">,
  share: number,
): Listener {
  const eye = eyePosition(vec3.create(), camera as Camera3DParams);
  const t = camera.target;
  const position = vec3.lerp(vec3.create(), t, eye, share);
  const forward = vec3.fromValues(t[0] - eye[0], t[1] - eye[1], 0);
  if (vec3.length(forward) < 1e-6)
    vec3.set(forward, -Math.cos(camera.yaw), -Math.sin(camera.yaw), 0);
  vec3.normalize(forward, forward);
  return { position, forward };
}

export interface BattleAudioStats extends SoundStats {
  /** The context is running (a gesture came and sound is not muted). */
  running: boolean;
  /** Voices the graph holds. */
  graph: number;
  /** Main-thread milliseconds spent synthesizing and composing prepared buffers. */
  bankMs: number;
  loading: boolean;
  error: string | null;
}

export class BattleAudio {
  private context: AudioContext | null = null;
  private sink: WebAudioSink | null = null;
  private frame: SoundFrame | null = null;
  /** Recorded and synthetic buffers are prepared: the frame may run. */
  private ready = false;
  private error: string | null = null;
  private readonly unsubscribe: () => void;
  private readonly onGesture = () => this.start();

  constructor(private readonly options: SoundFrameOptions) {
    this.unsubscribe = soundSettings.subscribe(() => this.applySettings());
    for (const type of ["pointerdown", "keydown"] as const)
      window.addEventListener(type, this.onGesture, { capture: true });
    // The gesture may have come before this battle existed (the button that
    // mounted it): the page's sticky activation lets the context run now.
    if (navigator.userActivation?.hasBeenActive) this.start();
  }

  /** Start (or resume) sound: call from a user gesture. Muted, it stays off. */
  start() {
    if (soundSettings.get().muted) return;
    if (!this.context) {
      this.context = new AudioContext({ latencyHint: "interactive" });
      this.sink = new WebAudioSink(this.context, this.options.presentation, this.options.catalog);
      this.frame = new SoundFrame(this.options, this.sink);
      const sink = this.sink;
      void sink.bank
        .prepare()
        .then(() => {
          if (this.sink !== sink) return;
          this.ready = true;
          this.frame?.reset();
        })
        .catch((error: unknown) => {
          if (this.sink !== sink) return;
          this.error = error instanceof Error ? error.message : String(error);
          console.error("Battle sound could not be prepared:", this.error);
        });
      this.applySettings();
    }
    if (this.context.state !== "running") {
      this.frame?.reset();
      void this.context.resume();
    }
  }

  private applySettings() {
    const { muted, volume } = soundSettings.get();
    if (!this.context || !this.sink) {
      if (!muted) this.start();
      return;
    }
    this.sink.master.gain.setTargetAtTime(
      this.options.presentation.buses.master * volume,
      this.context.currentTime,
      0.02,
    );
    if (muted && this.context.state === "running") void this.context.suspend();
    if (!muted && this.context.state !== "running") this.start();
  }

  private get live() {
    return this.ready && this.context?.state === "running" ? this.frame : null;
  }

  note(pub: SoundPublication) {
    this.live?.note(pub);
  }

  /** Each animation frame: the presentation clock, what is drawn moving,
   *  and the camera the listener follows. */
  update(clock: number, motion: SoundMotion, camera: Camera3DParams) {
    this.live?.update(
      clock,
      performance.now() / 1000,
      motion,
      cameraListener(camera, this.options.presentation.listener_eye_share),
    );
  }

  reset() {
    this.frame?.reset();
  }

  stats(): BattleAudioStats | null {
    if (!this.frame) return null;
    return {
      ...this.frame.stats(),
      running: this.context?.state === "running",
      graph: this.sink?.live ?? 0,
      bankMs: this.sink?.bank.ms ?? 0,
      loading: !this.ready && this.error === null,
      error: this.error,
    };
  }

  dispose() {
    this.unsubscribe();
    for (const type of ["pointerdown", "keydown"] as const)
      window.removeEventListener(type, this.onGesture, { capture: true });
    this.sink?.bank.dispose();
    this.ready = false;
    void this.context?.close();
    this.context = null;
    this.frame = null;
    this.sink = null;
  }
}

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
import { battleSounds } from "./catalog";
import type { AppAudio } from "./appAudio";
import type { SoundBank } from "./soundBank";
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

/** One battle's observed causes. The page owns context and prepared buffers;
 * this observer owns its evidence and disposable mixer, including reverb tails. */
export class BattleAudio {
  private sink: WebAudioSink | null = null;
  private frame: SoundFrame | null = null;
  private ready = false;
  private error: string | null = null;
  private disposed = false;

  constructor(
    private readonly options: SoundFrameOptions,
    private readonly app: AppAudio,
  ) {}

  /** The app connects a fresh graph once its context exists. */
  connect(context: AudioContext, bank: SoundBank) {
    if (this.disposed || this.sink) return;
    const sink = (this.sink = new WebAudioSink(
      context,
      this.options.presentation,
      this.options.catalog,
      bank,
    ));
    this.frame = new SoundFrame(this.options, sink);
    void bank
      .prepare(battleSounds(this.options.catalog))
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
  }

  start() {
    if (!this.disposed) this.app.start();
  }
  private get live() {
    return this.ready && this.app.stats().running ? this.frame : null;
  }
  note(pub: SoundPublication) {
    this.live?.note(pub);
  }
  update(clock: number, motion: SoundMotion, camera: Camera3DParams) {
    if (!this.live) return;
    this.live.update(
      clock,
      performance.now() / 1000,
      motion,
      cameraListener(camera, this.options.presentation.listener_eye_share),
    );
    this.app.battleStarted(this);
  }
  reset() {
    this.frame?.reset();
  }
  setVolume(volume: number) {
    if (!this.sink) return;
    this.sink.master.gain.setTargetAtTime(
      this.options.presentation.buses.master * volume,
      this.sink.now(),
      0.02,
    );
  }
  stats(): BattleAudioStats | null {
    if (!this.frame) return null;
    return {
      ...this.frame.stats(),
      running: this.app.stats().running,
      graph: this.sink?.live ?? 0,
      bankMs: this.sink?.bank.ms ?? 0,
      loading: !this.ready && this.error === null,
      error: this.error,
    };
  }
  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    this.frame?.reset();
    this.sink?.dispose();
    this.sink = null;
    this.frame = null;
    this.app.releaseBattle(this);
  }
}

import { camera_listener, Presentation } from "@wasm/presentation/presentation_wasm.js";
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

/** A frame a script draws: its presentation time, what moves, and where it is heard. */
export interface ScriptedHearing {
  clock: number;
  motion: SoundMotion;
  listener: Listener;
}
import { battleSounds } from "./catalog";
import type { AppAudio } from "./appAudio";
import type { SoundBank } from "./soundBank";
import { WebAudioSink } from "./webAudioSink";

/** Where a script's listener stands for a camera, as the battle's does
 *  (`Listener::of_camera`): `share` of the way from the point looked at to
 *  the eye, facing the way the camera looks (level). */
export function cameraListener(
  camera: Pick<Camera3DParams, "target" | "distance" | "pitch" | "yaw">,
  share: number,
): Listener {
  const eye = eyePosition(vec3.create(), camera as Camera3DParams);
  const s = Math.sin(camera.pitch);
  const up = [-s * Math.cos(camera.yaw), -s * Math.sin(camera.yaw), Math.cos(camera.pitch)];
  const l = camera_listener(Float64Array.from(eye), Float64Array.from(camera.target), Float64Array.from(up), share);
  return { position: [l[0], l[1], l[2]], forward: [l[3], l[4], l[5]] };
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
  /** Hear the battle's last publication, or a script's. */
  note(feed: Presentation | SoundPublication) {
    const frame = this.live;
    if (!frame) return;
    if (feed instanceof Presentation) frame.notePresented(feed);
    else frame.note(feed);
  }
  /** Hear the battle's last frame from its camera, or a script's frame. */
  update(feed: Presentation | ScriptedHearing) {
    const frame = this.live;
    if (!frame) return;
    const wall = performance.now() / 1000;
    if (feed instanceof Presentation) frame.hear(feed, wall);
    else frame.update(feed.clock, wall, feed.motion, feed.listener);
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

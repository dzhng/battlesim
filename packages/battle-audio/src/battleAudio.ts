import { camera_listener, Presentation } from "@wasm/renderer_browser";
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { vec3 } from "math";
import {
  SoundFrame,
  battleSounds,
  type Listener,
  type SoundFrameOptions,
  type SoundMotion,
  type SoundPublication,
  type SoundStats,
  type Speakers,
} from "./soundFrame";

/** A frame a script draws: its presentation time, what moves, and where it is heard. */
export interface ScriptedHearing {
  clock: number;
  motion: SoundMotion;
  listener: Listener;
}
import type { AppAudio } from "./appAudio";
import { loadSoundModule, type SoundBank } from "./soundBank";
import { WebAudioSink } from "./webAudioSink";
import { BattleVoices } from "./voices/battleVoices";
import { ShoutPool } from "./voices/shoutPool";
import type { ResponseRequest } from "./voices/responses";

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
  const l = camera_listener(
    Float64Array.from(eye),
    Float64Array.from(camera.target),
    Float64Array.from(up),
    share,
  );
  return { position: [l[0], l[1], l[2]], forward: [l[3], l[4], l[5]] };
}

interface BattleGraph {
  readonly sink: WebAudioSink;
  readonly frame: SoundFrame;
  readonly voices: BattleVoices;
  shouts: ShoutPool | null;
  state: "preparing" | "ready" | "failed";
  error: string | null;
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
  private graph: BattleGraph | null = null;
  private speakerSets: Speakers | null = null;
  /** Enemy unit kinds the side has identified: their sets load. */
  private readonly enemyKinds = new Set<string>();
  private disposed = false;

  constructor(
    private readonly options: SoundFrameOptions,
    private readonly app: AppAudio,
  ) {}

  /** The app connects a fresh graph once its context exists. */
  connect(context: AudioContext, bank: SoundBank) {
    if (this.disposed || this.graph) return;
    const sink = new WebAudioSink(context, this.options.presentation, this.options.catalog, bank);
    const graph: BattleGraph = {
      sink,
      frame: new SoundFrame(this.options, sink),
      voices: new BattleVoices(sink, this.options.presentation.voices),
      shouts: null,
      state: "preparing",
      error: null,
    };
    this.graph = graph;
    void loadSoundModule()
      .then(() => bank.prepare(battleSounds(this.options)))
      .then(() => {
        if (this.graph !== graph) return;
        graph.state = "ready";
        graph.frame.reset();
        void graph.voices.start();
        this.hearSpeakers();
      })
      .catch((error: unknown) => {
        if (this.graph !== graph) return;
        if (graph.state === "preparing") graph.state = "failed";
        graph.error = error instanceof Error ? error.message : String(error);
        console.error("Battle sound could not be prepared:", graph.error);
      });
  }

  start() {
    if (!this.disposed) this.app.start();
  }
  private get live() {
    return this.graph?.state === "ready" && this.app.stats().running ? this.graph.frame : null;
  }
  /** Hear the battle's last publication, or a script's. */
  note(feed: Presentation | SoundPublication) {
    const frame = this.live;
    if (!frame) return;
    if (feed instanceof Presentation) frame.notePresented(feed);
    else frame.note(feed);
  }
  /** A unit's answer to its owner (`responseFor`); unheard before sound is live. */
  respond(request: ResponseRequest) {
    if (this.live) this.graph?.voices.respond(request);
  }
  /** The voice sets the side's own units, and its purchases on their way,
   *  speak with; they load once its sounds are ready. */
  loadVoices(sets: Iterable<string>) {
    void this.graph?.voices.want(sets);
  }
  /** Enemy unit kinds the side identifies: from the first sight of a kind,
   *  the set it speaks with (`speakers`) loads, in the background. */
  identified(kinds: Iterable<string>) {
    let fresh = false;
    for (const kind of kinds)
      if (!this.enemyKinds.has(kind)) {
        this.enemyKinds.add(kind);
        fresh = true;
      }
    if (fresh) this.wantEnemies();
  }
  private wantEnemies() {
    const enemy = this.speakerSets?.enemy;
    if (enemy)
      void this.graph?.voices.want([...this.enemyKinds].flatMap((kind) => enemy[kind] ?? []));
  }
  /** Which voice set each unit kind speaks with, for the side's own faction
   *  and its enemy's: a shout heard in the open is a bark in its kind's set,
   *  and an unseen enemy's is drawn from the enemy's sets. Only the pool of
   *  unseen shouts loads from every enemy set; a kind's own set loads as the
   *  side's units field it (`loadVoices`) or identify it (`identified`). */
  speakers(speakers: Speakers) {
    this.speakerSets = speakers;
    this.hearSpeakers();
  }
  private hearSpeakers() {
    const graph = this.graph;
    const { speakerSets } = this;
    if (graph?.state !== "ready" || !speakerSets || graph.shouts) return;
    const { sink, frame } = graph;
    frame.speakers(speakerSets);
    this.wantEnemies();
    const enemySets = [...new Set(Object.values(speakerSets.enemy))].sort();
    const pool = (graph.shouts = new ShoutPool(
      sink.context,
      this.options.presentation.cues.shout.takes_per_voice,
    ));
    void pool.load(enemySets).then((takes) => {
      if (this.graph !== graph) return;
      sink.addTakes(takes);
      frame.shouts(takes.map(({ sound, buffer }) => ({ sound, duration: buffer.duration })));
    });
  }
  /** Hear the battle's last frame from its camera, or a script's frame. */
  update(feed: Presentation | ScriptedHearing) {
    const frame = this.live;
    if (!frame) return;
    const wall = performance.now() / 1000;
    if (feed instanceof Presentation) frame.hear(feed, wall);
    else frame.update(feed.clock, wall, feed.motion, feed.listener);
    // Lines are heard from where the frame just heard, in Rust, with the
    // barks it heard; the score hears them too, and dips under any line.
    const voices = this.graph?.voices;
    const lines = voices?.hear(frame);
    this.app.battleHeard(this, lines?.heard ?? [], voices?.speaking() ?? false);
  }
  reset() {
    this.graph?.frame.reset();
    this.graph?.voices.reset();
  }
  setVolume(volume: number, voices = 1) {
    const sink = this.graph?.sink;
    if (!sink) return;
    const at = sink.now();
    sink.master.gain.setTargetAtTime(this.options.presentation.buses.master * volume, at, 0.02);
    sink.buses.voices.gain.setTargetAtTime(
      this.options.presentation.buses.voices * voices,
      at,
      0.02,
    );
  }
  stats(): BattleAudioStats | null {
    const graph = this.graph;
    if (!graph) return null;
    return {
      ...graph.frame.stats(),
      running: this.app.stats().running,
      graph: graph.sink.live,
      bankMs: graph.sink.bank.ms,
      loading: graph.state === "preparing",
      error: graph.error,
    };
  }
  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    const graph = this.graph;
    if (graph) {
      graph.frame.reset();
      graph.voices.dispose();
      graph.shouts?.dispose();
      graph.sink.dispose();
    }
    this.graph = null;
    this.app.releaseBattle(this);
  }
}

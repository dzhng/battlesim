import { camera_listener, Presentation } from "@wasm/presentation/presentation_wasm.js";
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
  type VoiceSink,
  type VoiceSpec,
  type VoiceParams,
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

/** A recorded sound command for offline rendering. */
export type RecordedSoundCommand =
  | { type: "start"; id: number; spec: VoiceSpec; virtualTime: number }
  | { type: "set"; id: number; params: VoiceParams; at: number; virtualTime: number }
  | { type: "stop"; id: number; at: number; fade: number; virtualTime: number }
  | { type: "listen"; listener: Listener; at: number; virtualTime: number };

/** A recording sink that intercepts commands for offline rendering. */
class RecordingSink implements VoiceSink {
  private commands: RecordedSoundCommand[] = [];
  private virtualTime = 0;

  constructor(private readonly real: VoiceSink) {}

  setVirtualTime(time: number) {
    this.virtualTime = time;
  }

  getCommands(): RecordedSoundCommand[] {
    return this.commands;
  }

  clearCommands() {
    this.commands = [];
  }

  now(): number {
    return this.virtualTime;
  }

  duration(sound: string, variant?: number): number {
    return this.real.duration(sound, variant);
  }

  start(id: number, spec: VoiceSpec): void {
    this.commands.push({ type: "start", id, spec: { ...spec }, virtualTime: this.virtualTime });
    // Don't play real-time in recording mode - virtual time doesn't match wall time
  }

  set(id: number, params: VoiceParams, at: number): void {
    this.commands.push({
      type: "set",
      id,
      params: { ...params },
      at,
      virtualTime: this.virtualTime,
    });
  }

  stop(id: number, at: number, fade: number): void {
    this.commands.push({ type: "stop", id, at, fade, virtualTime: this.virtualTime });
  }

  listen(listener: Listener, at: number): void {
    this.commands.push({
      type: "listen",
      listener: {
        position: [...listener.position] as [number, number, number],
        forward: [...listener.forward] as [number, number, number],
      },
      at,
      virtualTime: this.virtualTime,
    });
  }

  clear(): void {
    // Don't clear real sink in recording mode
  }
}

/** One battle's observed causes. The page owns context and prepared buffers;
 * this observer owns its evidence and disposable mixer, including reverb tails. */
export class BattleAudio {
  private sink: WebAudioSink | null = null;
  private frame: SoundFrame | null = null;
  private voices: BattleVoices | null = null;
  private shouts: ShoutPool | null = null;
  private speakerSets: Speakers | null = null;
  /** Enemy unit kinds the side has identified: their sets load. */
  private readonly enemyKinds = new Set<string>();
  private ready = false;
  private error: string | null = null;
  private disposed = false;
  /** Recording sink for deterministic capture. */
  private recordingSink: RecordingSink | null = null;
  /** Whether recording mode is active. */
  private recording = false;

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
    this.voices = new BattleVoices(sink, this.options.presentation.voices);
    void loadSoundModule()
      .then(() => bank.prepare(battleSounds(this.options)))
      .then(() => {
        if (this.sink !== sink) return;
        this.ready = true;
        this.frame?.reset();
        void this.voices?.start();
        this.hearSpeakers();
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
  /** A unit's answer to its owner (`responseFor`); unheard before sound is live. */
  respond(request: ResponseRequest) {
    if (this.live) this.voices?.respond(request);
  }
  /** The voice sets the side's own units, and its purchases on their way,
   *  speak with; they load once its sounds are ready. */
  loadVoices(sets: Iterable<string>) {
    void this.voices?.want(sets);
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
    if (enemy) void this.voices?.want([...this.enemyKinds].flatMap((kind) => enemy[kind] ?? []));
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
    const { sink, frame, speakerSets } = this;
    if (!this.ready || !sink || !frame || !speakerSets || this.shouts) return;
    frame.speakers(speakerSets);
    this.wantEnemies();
    const enemySets = [...new Set(Object.values(speakerSets.enemy))].sort();
    const pool = (this.shouts = new ShoutPool(
      sink.context,
      this.options.presentation.cues.shout.takes_per_voice,
    ));
    void pool.load(enemySets).then((takes) => {
      if (this.shouts !== pool) return;
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
    const voices = this.voices;
    const lines = voices?.hear(frame);
    this.app.battleHeard(this, lines?.heard ?? [], voices?.speaking() ?? false);
  }
  /** Hear the battle's last frame from an explicit listener position. Use when
   *  the drawn camera differs from the presentation's stored camera (e.g. a
   *  capture pilot following a unit). */
  updateWithListener(presentation: Presentation, listener: Listener) {
    const frame = this.live;
    if (!frame) return;
    const wall = performance.now() / 1000;
    frame.hearAt(presentation, wall, listener);
    const voices = this.voices;
    const lines = voices?.hear(frame);
    this.app.battleHeard(this, lines?.heard ?? [], voices?.speaking() ?? false);
  }
  reset() {
    this.frame?.reset();
    this.voices?.reset();
  }
  /** Get the audio output latency in seconds (outputLatency + baseLatency).
   *  Returns 0 if context is not an AudioContext or latency is not available. */
  getOutputLatency(): number {
    const ctx = this.sink?.context;
    if (!ctx || !(ctx instanceof AudioContext)) return 0;
    // outputLatency: time from audio being rendered to reaching the speakers
    // baseLatency: time from when audio enters the system to when it's rendered
    const output = ctx.outputLatency ?? 0;
    const base = ctx.baseLatency ?? 0;
    return output + base;
  }

  setVolume(volume: number, voices = 1) {
    if (!this.sink) return;
    const at = this.sink.now();
    this.sink.master.gain.setTargetAtTime(
      this.options.presentation.buses.master * volume,
      at,
      0.02,
    );
    this.sink.buses.voices.gain.setTargetAtTime(
      this.options.presentation.buses.voices * voices,
      at,
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

  /** Create a MediaStream tap for recording the battle's audio output.
   *  Zero-cost when not called: the destination node is only created on demand.
   *  Returns the MediaStream that can be passed to MediaRecorder, or null
   *  if audio is not ready yet. */
  createCaptureTap(): MediaStream | null {
    return this.sink?.createCaptureTap() ?? null;
  }

  /** Release the capture tap when recording is done. */
  releaseCaptureTap(): void {
    this.sink?.releaseCaptureTap();
  }

  /** Enable recording mode for deterministic capture.
   *  In this mode, sounds are not played in real-time but recorded for offline rendering.
   *  Call setVirtualTime() before each audio update to set the current virtual time. */
  enableRecording(): void {
    if (!this.sink) return;
    this.recording = true;
    this.recordingSink = new RecordingSink(this.sink);
    // Replace the frame's sink with the recording sink
    if (this.frame) {
      this.frame = new SoundFrame(this.options, this.recordingSink);
      this.frame.reset();
    }
  }

  /** Disable recording mode and return to real-time audio. */
  disableRecording(): void {
    if (!this.sink || !this.recording) return;
    this.recording = false;
    // Restore the real sink
    if (this.frame) {
      this.frame = new SoundFrame(this.options, this.sink);
      this.frame.reset();
    }
    this.recordingSink = null;
  }

  /** Set the current virtual time for recording mode (seconds). */
  setVirtualTime(time: number): void {
    this.recordingSink?.setVirtualTime(time);
  }

  /** Get all recorded sound commands (for offline rendering). */
  getRecordedCommands(): RecordedSoundCommand[] {
    return this.recordingSink?.getCommands() ?? [];
  }

  /** Clear recorded commands. */
  clearRecordedCommands(): void {
    this.recordingSink?.clearCommands();
  }

  /** Whether recording mode is active. */
  isRecording(): boolean {
    return this.recording;
  }

  /** Get the sound bank for offline rendering. */
  getBank(): SoundBank | null {
    return this.sink?.bank ?? null;
  }

  /** Get the audio presentation options for offline rendering. */
  getPresentation(): SoundFrameOptions["presentation"] {
    return this.options.presentation;
  }

  /** Get the sound catalog for offline rendering. */
  getCatalog(): SoundFrameOptions["catalog"] {
    return this.options.catalog;
  }

  /** Render recorded commands offline and return as WAV blob.
   *  This creates an OfflineAudioContext, replays all recorded commands,
   *  and renders them through the same audio graph (buses → master → clipper). */
  async renderOffline(durationS: number, sampleRate = 48000): Promise<Blob | null> {
    const commands = this.getRecordedCommands();
    if (commands.length === 0) return null;

    console.log(`[audio] Rendering ${commands.length} commands offline for ${durationS.toFixed(2)}s`);

    // Create offline context
    const channelCount = 2;
    const length = Math.ceil(durationS * sampleRate);
    const offline = new OfflineAudioContext(channelCount, length, sampleRate);

    // Create a new WebAudioSink connected to the offline context
    const offlineSink = new WebAudioSink(
      offline,
      this.options.presentation,
      this.options.catalog,
    );

    // Collect all unique sounds used in the commands
    const soundsUsed = new Set<string>();
    for (const cmd of commands) {
      if (cmd.type === "start") {
        soundsUsed.add(cmd.spec.sound);
      }
    }

    // Prepare the bank with sounds used in the commands
    console.log(`[audio] Preparing ${soundsUsed.size} sounds for offline rendering`);
    await offlineSink.bank.prepare([...soundsUsed]);

    // Replay all commands through the offline sink
    console.log(`[audio] Replaying ${commands.length} commands`);
    for (const cmd of commands) {
      switch (cmd.type) {
        case "start":
          offlineSink.start(cmd.id, cmd.spec);
          break;
        case "set":
          offlineSink.set(cmd.id, cmd.params, cmd.at);
          break;
        case "stop":
          offlineSink.stop(cmd.id, cmd.at, cmd.fade);
          break;
        case "listen":
          offlineSink.listen(cmd.listener, cmd.at);
          break;
      }
    }

    // Render offline
    console.log(`[audio] Rendering offline...`);
    const rendered = await offline.startRendering();
    console.log(`[audio] Rendered ${rendered.length} samples`);

    // Convert to WAV blob
    const wavBlob = this.audioBufferToWav(rendered);

    // Clean up
    offlineSink.dispose();

    return wavBlob;
  }

  /** Convert an AudioBuffer to a WAV file blob. */
  private audioBufferToWav(buffer: AudioBuffer): Blob {
    const numChannels = buffer.numberOfChannels;
    const sampleRate = buffer.sampleRate;
    const format = 1; // PCM
    const bitDepth = 16;
    const bytesPerSample = bitDepth / 8;
    const blockAlign = numChannels * bytesPerSample;

    // Interleave channels
    const samples = buffer.length;
    const dataLength = samples * blockAlign;
    const bufferLength = 44 + dataLength;
    const arrayBuffer = new ArrayBuffer(bufferLength);
    const view = new DataView(arrayBuffer);

    // WAV header
    let offset = 0;
    const writeString = (str: string) => {
      for (let i = 0; i < str.length; i++) {
        view.setUint8(offset++, str.charCodeAt(i));
      }
    };
    const writeUint32 = (value: number) => {
      view.setUint32(offset, value, true);
      offset += 4;
    };
    const writeUint16 = (value: number) => {
      view.setUint16(offset, value, true);
      offset += 2;
    };

    writeString("RIFF");
    writeUint32(bufferLength - 8);
    writeString("WAVE");
    writeString("fmt ");
    writeUint32(16); // fmt chunk size
    writeUint16(format);
    writeUint16(numChannels);
    writeUint32(sampleRate);
    writeUint32(sampleRate * blockAlign);
    writeUint16(blockAlign);
    writeUint16(bitDepth);
    writeString("data");
    writeUint32(dataLength);

    // Interleaved samples
    const channels: Float32Array[] = [];
    for (let ch = 0; ch < numChannels; ch++) {
      channels.push(buffer.getChannelData(ch));
    }

    for (let i = 0; i < samples; i++) {
      for (let ch = 0; ch < numChannels; ch++) {
        const sample = Math.max(-1, Math.min(1, channels[ch][i]));
        const intSample = sample < 0 ? sample * 0x8000 : sample * 0x7fff;
        view.setInt16(offset, intSample, true);
        offset += 2;
      }
    }

    return new Blob([arrayBuffer], { type: "audio/wav" });
  }

  /** Play a calibration click through the same audio graph as battle sounds.
   *  Used to measure the exact audio pipeline latency by comparing the click's
   *  timestamp in the recorded audio against a simultaneous visual marker.
   *  The click is 20ms long at 2kHz with a sharp attack for easy transient detection.
   *  Returns false if audio is not ready. */
  playCalibrationClick(): boolean {
    const ctx = this.sink?.context;
    if (!ctx || !(ctx instanceof AudioContext)) return false;
    
    // Create a loud, sharp click: 20ms of 2kHz sine at full amplitude
    const sampleRate = ctx.sampleRate;
    const duration = 0.02; // 20ms - long enough to detect reliably
    const samples = Math.ceil(sampleRate * duration);
    const buffer = ctx.createBuffer(1, samples, sampleRate);
    const data = buffer.getChannelData(0);
    
    // Generate a sharp impulse with instant attack, quick decay
    // First 1ms at full amplitude, then exponential decay
    const attackSamples = Math.ceil(sampleRate * 0.001); // 1ms attack
    for (let i = 0; i < samples; i++) {
      const t = i / sampleRate;
      const freq = 2000; // 2kHz for sharp transient
      let envelope: number;
      if (i < attackSamples) {
        // Instant attack to full amplitude
        envelope = 1.0;
      } else {
        // Exponential decay over remaining time
        const decayT = (i - attackSamples) / (samples - attackSamples);
        envelope = Math.exp(-3 * decayT); // decay to ~5% by end
      }
      data[i] = Math.sin(2 * Math.PI * freq * t) * envelope;
    }
    
    // Create a gain node for extra loudness (the click should be unmistakable)
    const gain = ctx.createGain();
    gain.gain.value = 2.0; // +6dB boost
    
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    source.connect(gain);
    gain.connect(this.sink!.master);
    source.start();
    return true;
  }
  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    this.frame?.reset();
    this.voices?.dispose();
    this.shouts?.dispose();
    this.shouts = null;
    this.sink?.dispose();
    this.sink = null;
    this.frame = null;
    this.voices = null;
    this.app.releaseBattle(this);
  }
}

/**
 * Offline audio rendering for deterministic capture mode.
 * 
 * Records all sound commands with virtual timestamps, then renders them
 * through an OfflineAudioContext using the identical audio graph
 * (buses → master → limiter → clipper).
 */

import type { VoiceSink, VoiceSpec, VoiceParams, Listener } from "./soundFrame";
import { WebAudioSink } from "./webAudioSink";
import type { AudioPresentation, Bus } from "./audioPresentation";
import type { SoundCatalog } from "./catalog";
import { SoundBank } from "./soundBank";

/** A recorded sound command with its virtual timestamp. */
export type SoundCommand =
  | { type: "start"; id: number; spec: VoiceSpec; at: number }
  | { type: "set"; id: number; params: VoiceParams; at: number }
  | { type: "stop"; id: number; at: number; fade: number }
  | { type: "listen"; listener: Listener; at: number };

/** Records sound commands without playing them, for later offline rendering. */
export class SoundRecorder implements VoiceSink {
  private commands: SoundCommand[] = [];
  private virtualTime = 0;
  private startTime: number | null = null;

  /** Set the current virtual time (seconds from start). */
  setVirtualTime(seconds: number) {
    if (this.startTime === null) this.startTime = seconds;
    this.virtualTime = seconds - this.startTime;
  }

  /** Get all recorded commands. */
  getCommands(): SoundCommand[] {
    return this.commands;
  }

  /** VoiceSink implementation: returns current virtual time. */
  now(): number {
    return this.virtualTime;
  }

  /** VoiceSink implementation: record start command. */
  start(id: number, spec: VoiceSpec): void {
    this.commands.push({ type: "start", id, spec: { ...spec }, at: spec.at });
  }

  /** VoiceSink implementation: record set command. */
  set(id: number, params: VoiceParams, at: number): void {
    this.commands.push({ type: "set", id, params: { ...params }, at });
  }

  /** VoiceSink implementation: record stop command. */
  stop(id: number, at: number, fade: number): void {
    this.commands.push({ type: "stop", id, at, fade });
  }

  /** VoiceSink implementation: record listen command. */
  listen(listener: Listener, at: number): void {
    this.commands.push({
      type: "listen",
      listener: {
        position: [...listener.position] as [number, number, number],
        forward: [...listener.forward] as [number, number, number],
      },
      at,
    });
  }

  /** VoiceSink implementation: returns 0 for recorder. */
  duration(_sound: string, _variant?: number): number {
    return 0;
  }
}

/** Renders recorded sound commands offline. */
export async function renderOfflineAudio(
  commands: SoundCommand[],
  duration: number,
  sampleRate: number,
  presentation: AudioPresentation,
  catalog: SoundCatalog,
  liveBank?: SoundBank,
): Promise<AudioBuffer> {
  // Create offline context
  const channelCount = 2;
  const length = Math.ceil(duration * sampleRate);
  const offline = new OfflineAudioContext(channelCount, length, sampleRate);

  // Create a sound bank for the offline context
  // We need to copy buffers from the live bank if available
  const bank = new SoundBank(offline, catalog);
  
  // If we have a live bank, we need to prepare the same sounds
  // The bank will decode the same audio files for the offline context
  // This is necessary because AudioBuffers are context-specific
  
  // Create sink with the same graph structure
  const sink = new WebAudioSink(offline, presentation, catalog, bank);

  // Prepare the bank with sounds used in the commands
  const soundsUsed = new Set<string>();
  for (const cmd of commands) {
    if (cmd.type === "start") {
      soundsUsed.add(cmd.spec.sound);
    }
  }
  
  // Wait for bank to prepare (decode audio files)
  await bank.prepare([...soundsUsed]);

  // Replay all commands through the sink
  for (const cmd of commands) {
    switch (cmd.type) {
      case "start":
        sink.start(cmd.id, cmd.spec);
        break;
      case "set":
        sink.set(cmd.id, cmd.params, cmd.at);
        break;
      case "stop":
        sink.stop(cmd.id, cmd.at, cmd.fade);
        break;
      case "listen":
        sink.listen(cmd.listener, cmd.at);
        break;
    }
  }

  // Render offline
  const rendered = await offline.startRendering();
  
  // Clean up
  sink.dispose();
  bank.dispose();

  return rendered;
}

/** Convert an AudioBuffer to a WAV file blob. */
export function audioBufferToWav(buffer: AudioBuffer): Blob {
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

/** Sound event with position and listener pose for later correlation. */
export interface RecordedSoundEvent {
  /** Virtual time in seconds. */
  time: number;
  /** Sound category (fire, impact, explosion, etc.). */
  category: string;
  /** Sound name. */
  sound: string;
  /** Position in world coordinates. */
  position: [number, number, number];
  /** Listener position at this time. */
  listenerPosition: [number, number, number];
  /** Listener forward direction at this time. */
  listenerForward: [number, number, number];
  /** Additional metadata. */
  meta?: Record<string, unknown>;
}

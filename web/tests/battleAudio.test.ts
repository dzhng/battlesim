// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import game from "@fixtures/game.json";
import sounds from "@fixtures/sounds.json";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { BattleAudio } from "@packages/battle-audio/src/battleAudio";
import { soundSettings } from "@packages/battle-audio/src/settings";

const param = () => ({
  setTargetAtTime() {},
  setValueAtTime() {},
  linearRampToValueAtTime() {},
  cancelScheduledValues() {},
});
class Node {
  gain = param();
  connect(node: unknown) {
    return node;
  }
  disconnect() {}
}
function buffer(channels: number, length: number, sampleRate: number): AudioBuffer {
  const data = Array.from({ length: channels }, () => new Float32Array(length));
  return {
    numberOfChannels: channels,
    length,
    sampleRate,
    duration: length / sampleRate,
    getChannelData: (channel: number) => data[channel],
    copyToChannel: (source: Float32Array, channel: number) => data[channel].set(source),
  } as unknown as AudioBuffer;
}
const empty = (tick: number) => ({
  effects: { tick, shooters: [], segments: [], blasts: [], smokes: [] },
  audible: [],
});
const active: BattleAudio[] = [];
afterEach(() => {
  for (const audio of active.splice(0)) audio.dispose();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function setupDecode(decode: () => Promise<AudioBuffer>) {
  class Context {
    state = "suspended";
    sampleRate = 8000;
    currentTime = 0;
    destination = new Node();
    createBuffer = buffer;
    decodeAudioData = decode;
    async resume() {
      this.state = "running";
    }
    async suspend() {
      this.state = "suspended";
    }
    async close() {
      this.state = "closed";
    }
  }
  for (const name of ["DynamicsCompressorNode", "GainNode", "ConvolverNode"])
    vi.stubGlobal(name, Node);
  vi.stubGlobal("AudioContext", Context);
  vi.stubGlobal("fetch", async () => new Response(Uint8Array.of(1)));
  soundSettings.set({ muted: false, volume: 1 });
  const catalog = structuredClone(sounds) as SoundCatalog;
  catalog.clips.pending = { url: "/pending.wav", loop: false } as SoundCatalog["clips"][string];
  catalog.sounds.pending = {
    label: "Pending",
    clips: ["pending"],
    synth: null,
    synth_gain: 0,
    gain: 1,
    loop: false,
  };
  const audio = new BattleAudio({
    tickHz: 30,
    presentation: game.presentation.audio as unknown as AudioPresentation,
    smokeTimes: {},
    catalog,
  });
  active.push(audio);
  audio.start();
  return audio;
}

test("live audio drops publications received before asynchronous preparation and starts from fresh evidence", async () => {
  let finish!: (value: AudioBuffer) => void;
  const audio = setupDecode(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  audio.note(empty(8));
  expect(audio.stats()).toMatchObject({ loading: true, tick: -1 });
  await vi.waitFor(() => expect(finish).toBeTypeOf("function"));
  finish(buffer(1, 2, 8000));
  await vi.waitFor(() => expect(audio.stats()?.loading).toBe(false));
  expect(audio.stats()).toMatchObject({ tick: -1, pending: 0, error: null });
  audio.note(empty(9));
  expect(audio.stats()).toMatchObject({ tick: 9, pending: 0 });
});

test("failed audio preparation is reported and does not admit publications", async () => {
  vi.spyOn(console, "error").mockImplementation(() => {});
  const audio = setupDecode(async () => {
    throw new Error("Invalid WAV");
  });
  await vi.waitFor(() => expect(audio.stats()?.error).toBe("Invalid WAV"));
  expect(audio.stats()).toMatchObject({ loading: false, tick: -1 });
  audio.note(empty(8));
  expect(audio.stats()?.tick).toBe(-1);
});

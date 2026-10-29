// @vitest-environment jsdom
import { presentationSettings } from "@web/presentationSettings";
import { audioMix } from "@web/audioMix";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import { SOUNDS } from "@packages/battle-audio/src/synthSounds.gen";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { standInPresentationRecordings } from "./support/soundStandIns";
import { AppAudio } from "@packages/battle-audio/src/appAudio";
import { soundSettings } from "@packages/battle-audio/src/settings";
import { MusicPlayer } from "@packages/battle-audio/src/music/musicPlayer";

beforeAll(() => loadPresentation());

const COOK_OFF = presentationSettings.effects.cook_off;

const param = () => ({
  setTargetAtTime() {},
  setValueAtTime() {},
  linearRampToValueAtTime() {},
  cancelScheduledValues() {},
});
class Node {
  gain = param();
  frequency = param();
  pan = param();
  positionX = param();
  positionY = param();
  positionZ = param();
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
  effects: { tick, segments: [], blasts: [], smokes: [] },
  launches: [],
  audible: [],
});
const active: AppAudio[] = [];
afterEach(() => {
  for (const audio of active.splice(0)) audio.dispose();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function setupDecode(
  decode: (bytes: ArrayBuffer) => Promise<AudioBuffer>,
  fetcher: (url: string) => Promise<Response> = async () => new Response(Uint8Array.of(1)),
) {
  class Context {
    state = "suspended";
    sampleRate = 8000;
    currentTime = 0;
    destination = new Node();
    listener = Object.fromEntries(
      [
        "positionX",
        "positionY",
        "positionZ",
        "forwardX",
        "forwardY",
        "forwardZ",
        "upX",
        "upY",
        "upZ",
      ].map((axis) => [axis, param()]),
    );
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
  for (const name of ["DynamicsCompressorNode", "GainNode", "ConvolverNode", "WaveShaperNode"])
    vi.stubGlobal(name, Node);
  vi.stubGlobal("AudioContext", Context);
  vi.stubGlobal("fetch", fetcher);
  soundSettings.set({ muted: false, volume: 1 });
  const catalog: SoundCatalog = {
    sources: {
      controlled: {
        label: "Controlled recording",
        author: "Test",
        license: "CC0-1.0",
        url: "https://example.invalid/controlled.wav",
        path: "assets/third-party/audio/controlled.wav",
        sha256: "0".repeat(64),
        notes: "Fetch and decoding are controlled by this test.",
      },
    },
    clips: {
      pending: {
        label: "Pending recording",
        category: "report",
        role: "shot",
        source: "controlled",
        source_rate: 8000,
        source_frames: [0, 2],
        processing: "none",
        url: "/audio/clips/pending.wav",
        sha256: "0".repeat(64),
        sample_rate: 8000,
        frames: 2,
        loop: false,
        notes: "",
      },
    },
    sounds: {
      ...Object.fromEntries(
        Object.entries(SOUNDS).map(([name, sound]) => [
          name,
          {
            label: name,
            clips: [],
            synth: name,
            synth_gain: 1,
            gain: 1,
            loop: sound.loop,
          },
        ]),
      ),
      pending: {
        label: "Pending",
        clips: ["pending"],
        synth: null,
        synth_gain: 0,
        gain: 1,
        loop: false,
      },
    },
    // A battle prepares only what it can play: the controlled recording fires.
    defaults: { default: { near: "pending", far: "pending", gain: 1 } },
    impacts: {},
    effects: {},
    music: { moods: { menu: [], tension: [], combat: [] }, tracks: {} },
  };
  const presentation = audioMix as unknown as AudioPresentation;
  standInPresentationRecordings(catalog, presentation);
  const app = new AppAudio({ presentation, catalog });
  const audio = app.createBattle(
    {
      tickHz: 30,
      presentation: audioMix as unknown as AudioPresentation,
      smokeTimes: {},
      cookOff: COOK_OFF,
    },
    1,
  );
  active.push(app);
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

test("document departure cancels pending preparation and BFCache return requires manual recovery", async () => {
  let finish: ((value: AudioBuffer) => void) | undefined;
  const errors = vi.spyOn(console, "error").mockImplementation(() => {});
  const audio = setupDecode(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  await vi.waitFor(() => expect(finish).toBeTypeOf("function"));
  window.dispatchEvent(new Event("pagehide"));
  expect(audio.stats()).toBeNull();
  finish!(buffer(1, 2, 8000));
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(audio.stats()).toBeNull();
  expect(errors.mock.calls).toEqual([]);

  window.dispatchEvent(new Event("pageshow"));
  audio.start();
  expect(audio.stats()).toBeNull();
});

test("a departed battle cannot admit late decoding and its replacement shares the pending bank", async () => {
  let finish!: (value: AudioBuffer) => void;
  let decodes = 0;
  const first = setupDecode(() => {
    decodes++;
    return new Promise((resolve) => {
      finish = resolve;
    });
  });
  await vi.waitFor(() => expect(finish).toBeTypeOf("function"));
  first.dispose();
  const next = active.at(-1)!.createBattle(
    {
      tickHz: 30,
      presentation: audioMix as unknown as AudioPresentation,
      smokeTimes: {},
      cookOff: COOK_OFF,
    },
    1,
  );
  next.note(empty(5));
  finish(buffer(1, 2, 8000));
  await vi.waitFor(() => expect(next.stats()?.loading).toBe(false));
  expect(decodes).toBe(1);
  expect(first.stats()).toBeNull();
  expect(next.stats()).toMatchObject({ tick: -1, pending: 0 });
  next.note(empty(6));
  expect(next.stats()?.tick).toBe(6);
});

test("a battle loads its voice sets only once its sounds are ready, and its units' answers play from them", async () => {
  // Takes decode to a buffer of their own; the battle's recording waits until released.
  let release!: () => void;
  const ready = new Promise<void>((resolve) => (release = resolve));
  const take = buffer(1, 8000, 8000);
  const fetched: string[] = [];
  const fetcher = async (url: string) => {
    fetched.push(url);
    if (url === "/audio/voices/tank/pack.json")
      return Response.json({
        set: "tank",
        voices: [{ id: "lead", speaks: "responses" }],
        takes: [
          {
            voice: "lead",
            trigger: "move",
            intensity: "steady",
            take: 0,
            url: "/audio/voices/tank/lead-move-steady-0.abcd1234.mp3",
            duration_s: 1,
          },
        ],
      });
    if (url.startsWith("/audio/voices/")) return new Response(Uint8Array.of(9));
    await ready;
    return new Response(Uint8Array.of(1));
  };
  const played: AudioBuffer[] = [];
  class Source extends Node {
    buffer: AudioBuffer;
    playbackRate = param();
    onended: (() => void) | null = null;
    constructor(_: unknown, options: { buffer: AudioBuffer }) {
      super();
      this.buffer = options.buffer;
    }
    start() {
      played.push(this.buffer);
    }
    stop() {}
  }
  vi.stubGlobal("AudioBufferSourceNode", Source);
  for (const name of ["BiquadFilterNode", "PannerNode", "StereoPannerNode"])
    vi.stubGlobal(name, Node);
  const audio = setupDecode(
    async (bytes) => (new Uint8Array(bytes)[0] === 9 ? take : buffer(1, 2, 8000)),
    fetcher,
  );

  const answer = {
    tier: "response",
    speaker: "own/1",
    set: "tank",
    trigger: "move",
    intensity: "steady",
    groupKey: "order/1",
  } as const;
  const frame = () =>
    audio.update({
      clock: 0,
      motion: { vehicles: [], soldiers: [] },
      listener: { position: [0, 0, 0], forward: [0, 1, 0] },
    });
  audio.loadVoices(["tank"]);
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(fetched.filter((url) => url.startsWith("/audio/voices/"))).toEqual([]);
  // Before readiness an answer is not heard, and not kept for later.
  audio.respond(answer);
  release();
  await vi.waitFor(() => expect(audio.stats()?.loading).toBe(false));
  await vi.waitFor(() =>
    expect(fetched).toContain("/audio/voices/tank/lead-move-steady-0.abcd1234.mp3"),
  );
  await new Promise((resolve) => setTimeout(resolve, 0));
  frame();
  expect(played).not.toContain(take);
  audio.respond({ ...answer, groupKey: "order/2" });
  frame();
  await vi.waitFor(() => expect(played.filter((b) => b === take)).toHaveLength(1));
  // Only the set asked for was fetched.
  expect(fetched.filter((url) => url.endsWith("pack.json"))).toEqual([
    "/audio/voices/tank/pack.json",
  ]);
});

test("a battle's own shout plays its kind's bark where it was shouted, and the music dips under it", async () => {
  const take = buffer(1, 8000, 8000);
  const fetcher = async (url: string) => {
    if (url === "/audio/voices/tank/pack.json")
      return Response.json({
        set: "tank",
        voices: [{ id: "gunner", speaks: "barks" }],
        takes: [
          {
            voice: "gunner",
            trigger: "under_fire",
            intensity: "stressed",
            take: 0,
            url: "/audio/voices/tank/gunner-under_fire-stressed-0.abcd1234.mp3",
            duration_s: 1,
          },
        ],
      });
    if (url.endsWith("pack.json")) return new Response(null, { status: 404 });
    return new Response(Uint8Array.of(url.startsWith("/audio/voices/") ? 9 : 1));
  };
  const played: AudioBuffer[] = [];
  const placed: number[][] = [];
  class Source extends Node {
    buffer: AudioBuffer;
    playbackRate = param();
    onended: (() => void) | null = null;
    constructor(_: unknown, options: { buffer: AudioBuffer }) {
      super();
      this.buffer = options.buffer;
    }
    start() {
      played.push(this.buffer);
    }
    stop() {}
  }
  class Panner extends Node {
    constructor(_: unknown, o: { positionX: number; positionY: number; positionZ: number }) {
      super();
      placed.push([o.positionX, o.positionY, o.positionZ]);
    }
  }
  vi.stubGlobal("AudioBufferSourceNode", Source);
  vi.stubGlobal("PannerNode", Panner);
  for (const name of ["BiquadFilterNode", "StereoPannerNode"]) vi.stubGlobal(name, Node);
  const ducks = vi.spyOn(MusicPlayer.prototype, "duck");
  const audio = setupDecode(
    async (bytes) => (new Uint8Array(bytes)[0] === 9 ? take : buffer(1, 2, 8000)),
    fetcher,
  );
  audio.speakers({ own: { m1a2: "tank" }, enemy: { t90: "ru_tank" } });
  audio.loadVoices(["tank"]);
  await vi.waitFor(() => expect(audio.stats()?.loading).toBe(false));
  await new Promise((resolve) => setTimeout(resolve, 0));
  const frame = () =>
    audio.update({
      clock: 0,
      motion: { vehicles: [], soldiers: [] },
      listener: { position: [0, 0, 0], forward: [0, 1, 0] },
    });
  frame();
  audio.note({
    ...empty(1),
    shouts: [
      {
        own: true,
        id: 4,
        kind: "m1a2",
        position: [30, 10, 1],
        trigger: "b5_under_fire",
        intensity: "stressed",
      },
    ],
  });
  frame();
  await vi.waitFor(() => expect(played.filter((b) => b === take)).toHaveLength(1));
  expect(placed).toContainEqual([30, 10, 1]);
  const { depth } = audioMix.music.duck;
  expect(ducks).toHaveBeenCalledWith(depth, audioMix.music.duck.attack_s);
});

test("a skirmish loads the sets of its fielded units and of enemy kinds once identified; the rest only lend their shout pool", async () => {
  // Each set answers with its lead and barks with its gunner; the unseen
  // shout pool takes a stressed bark from every enemy set.
  const pack = (set: string) => ({
    set,
    voices: [
      { id: "lead", speaks: "responses" },
      { id: "gunner", speaks: "barks" },
    ],
    takes: [
      ["lead", "move", "steady"],
      ["gunner", "under_fire", "stressed"],
    ].map(([voice, trigger, intensity]) => ({
      voice,
      trigger,
      intensity,
      take: 0,
      url: `/audio/voices/${set}/${voice}-${trigger}-${intensity}-0.abcd1234.mp3`,
      duration_s: 1,
    })),
  });
  const fetched: string[] = [];
  const fetcher = async (url: string) => {
    fetched.push(url);
    const set = /^\/audio\/voices\/(\w+)\/pack\.json$/.exec(url)?.[1];
    if (set) return Response.json(pack(set));
    return new Response(Uint8Array.of(1));
  };
  const audio = setupDecode(async () => buffer(1, 2, 8000), fetcher);
  const takes = () =>
    fetched
      .filter((url) => url.endsWith(".mp3"))
      .map((url) => url.split("/").slice(3, 5).join("/").split("-").slice(0, 1).join())
      .sort();
  audio.speakers({
    own: { m1a2: "tank", m2: "ifv" },
    enemy: { t90: "ru_tank", bmp: "ru_ifv" },
  });
  // The side fields tanks only (`UnitResponses` asks for its units' sets).
  audio.loadVoices(["tank"]);
  await vi.waitFor(() => expect(audio.stats()?.loading).toBe(false));
  await vi.waitFor(() => expect(takes()).toHaveLength(4));
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(takes()).toEqual(["ru_ifv/gunner", "ru_tank/gunner", "tank/gunner", "tank/lead"]);
  // A T-90 comes into sight: its set loads; the unseen BMP's still doesn't.
  audio.identified(["t90"]);
  await vi.waitFor(() => expect(takes()).toContain("ru_tank/lead"));
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(takes()).toEqual([
    "ru_ifv/gunner",
    "ru_tank/gunner",
    "ru_tank/gunner",
    "ru_tank/lead",
    "tank/gunner",
    "tank/lead",
  ]);
});

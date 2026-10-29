// @vitest-environment jsdom
import { presentationSettings } from "@web/presentationSettings";
import { audioMix } from "@web/audioMix";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import { cameraListener } from "@packages/battle-audio/src/battleAudio";
import { AppAudio } from "@packages/battle-audio/src/appAudio";
import { SOUNDS } from "@packages/battle-audio/src/synthSounds.gen";
import { soundSettings } from "@packages/battle-audio/src/settings";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { standInPresentationRecordings } from "./support/soundStandIns";
import type { AudioMix } from "@packages/battle-audio/src/audioMix";

beforeAll(() => loadPresentation());

// Every score a battle opens, by the seed it shuffles with.
const scoreSeeds = vi.hoisted(() => [] as number[]);
vi.mock("@packages/battle-audio/src/music/battleMusic", async (importOriginal) => {
  const real =
    await importOriginal<typeof import("@packages/battle-audio/src/music/battleMusic")>();
  class BattleMusic extends real.BattleMusic {
    constructor(...args: ConstructorParameters<typeof real.BattleMusic>) {
      super(...args);
      scoreSeeds.push(args[3]);
    }
  }
  return { ...real, BattleMusic };
});

const param = (value = 0) => ({
  value,
  setTargetAtTime(value: number) {
    this.value = value;
  },
  setValueAtTime(value: number) {
    this.value = value;
  },
  linearRampToValueAtTime(value: number) {
    this.value = value;
  },
  cancelScheduledValues() {},
});
class Node {
  gain = param();
  frequency = param();
  playbackRate = param();
  pan = param();
  positionX = param();
  positionY = param();
  positionZ = param();
  forwardX = param();
  forwardY = param();
  forwardZ = param();
  upX = param();
  upY = param();
  upZ = param();
  connections = new Set<Node>();
  constructor(_context?: Context, options: Record<string, unknown> = {}) {
    nodes.push(this);
    Object.assign(this, options);
    for (const key of [
      "gain",
      "frequency",
      "playbackRate",
      "pan",
      "positionX",
      "positionY",
      "positionZ",
    ])
      if (options[key] !== undefined) Object.assign(this, { [key]: param(Number(options[key])) });
  }
  connect(node: Node) {
    this.connections.add(node);
    return node;
  }
  disconnect() {
    this.connections.clear();
  }
}
class Source extends Node {
  declare buffer: AudioBuffer;
  declare loop: boolean;
  onended: (() => void) | null = null;
  stopped = false;
  offset = -1;
  start(_at: number, offset: number) {
    this.offset = offset;
    sources.push(this);
  }
  stop() {
    this.stopped = true;
    this.onended?.();
  }
}
class Gain extends Node {}
class MediaSource extends Node {
  constructor(
    context: Context,
    readonly options: { mediaElement: Media },
  ) {
    super(context);
  }
}
class Media {
  src = "";
  loop = false;
  preload = "";
  paused = true;
  play = vi.fn(async () => {
    this.paused = false;
  });
  pause() {
    this.paused = true;
  }
  load() {}
  removeAttribute() {
    this.src = "";
  }
  constructor() {
    media.push(this);
  }
}
/** The gain from `from` to the speakers through gain nodes; −1 when it never arrives. */
function reach(from: Node, destination: Node): number {
  if (from === destination) return 1;
  for (const next of from.connections) {
    const rest = reach(next, destination);
    if (rest >= 0) return (from instanceof Gain ? from.gain.value : 1) * rest;
  }
  return -1;
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
  } as AudioBuffer;
}
class Context {
  state = "suspended";
  sampleRate = 8000;
  currentTime = 0;
  destination = new Node();
  listener = new Node();
  createBuffer = buffer;
  close = vi.fn(async () => {
    this.state = "closed";
  });
  resume = vi.fn(async () => {
    this.state = "running";
  });
  suspend = vi.fn(async () => {
    this.state = "suspended";
  });
  constructor() {
    contexts.push(this);
  }
}
const nodes: Node[] = [];
const contexts: Context[] = [];
const sources: Source[] = [];
const media: Media[] = [];
const active: AppAudio[] = [];
afterEach(() => {
  active.splice(0).forEach((a) => a.dispose());
  nodes.length = 0;
  contexts.length = 0;
  sources.length = 0;
  media.length = 0;
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
const presentation = audioMix as unknown as AudioMix;
const cookOff = presentationSettings.effects.cook_off;
const empty = (tick: number) => ({
  effects: { tick, segments: [], blasts: [], smokes: [] },
  launches: [],
  audible: [],
});
const camera = {
  target: [0, 0, 0] as [number, number, number],
  distance: 20,
  pitch: 0.6,
  yaw: 0,
  aspect: 1,
  fovY: 0.7,
  near: 0.1,
  far: 1000,
};
function setup() {
  for (const name of [
    "DynamicsCompressorNode",
    "ConvolverNode",
    "BiquadFilterNode",
    "PannerNode",
    "StereoPannerNode",
    "WaveShaperNode",
  ])
    vi.stubGlobal(name, Node);
  vi.stubGlobal("GainNode", Gain);
  vi.stubGlobal("MediaElementAudioSourceNode", MediaSource);
  vi.stubGlobal("Audio", Media);
  vi.stubGlobal("AudioBufferSourceNode", Source);
  vi.stubGlobal("AudioContext", Context);
  soundSettings.set({ muted: false, volume: 1, music: 1 });
  const catalog: SoundCatalog = {
    sources: {},
    clips: {},
    sounds: Object.fromEntries(
      Object.entries(SOUNDS).map(([name, sound]) => [
        name,
        { label: name, clips: [], synth: name, synth_gain: 1, gain: 1, loop: sound.loop },
      ]),
    ),
    defaults: {},
    impacts: {},
    effects: {},
    music: {
      moods: { menu: ["bed"], tension: [], combat: [] },
      tracks: { bed: { source: "bed", sha256: "a".repeat(64), gain: 0.5 } },
    },
  };
  catalog.sounds["recorded-clack-14"] = { ...catalog.sounds.rifle };
  catalog.sounds["recorded-clack-17"] = { ...catalog.sounds.cannon };
  standInPresentationRecordings(catalog, presentation);
  const app = new AppAudio({ presentation, catalog });
  active.push(app);
  return app;
}

test("the menu bed streams through the music player under the master; only ambience is a decoded loop", async () => {
  const app = setup();
  app.setScreen("menu");
  await vi.waitFor(() => expect(app.stats().menuReady).toBe(true));
  expect(app.stats().menuPlaying).toBe(true);
  expect(media).toHaveLength(1);
  expect(media[0]).toMatchObject({
    src: "/audio/music/bed.aaaaaaaa.mp3",
    loop: true,
    paused: false,
  });
  // The one decoded loop is the countryside, at a staggered phase.
  expect(sources.filter((s) => s.loop)).toHaveLength(1);
  expect(sources[0].offset).toBeGreaterThan(0);
});

test("master volume and the music level both scale the menu bed, and mute silences it", async () => {
  const app = setup();
  app.setScreen("menu");
  await vi.waitFor(() => expect(app.stats().menuPlaying).toBe(true));
  const bed = nodes.find((n) => n instanceof MediaSource)!;
  const speakers = contexts[0].destination;
  const { master, music } = presentation.buses;
  // Params hold their last target: the bed's fade-in has reached its catalog gain.
  expect(reach(bed, speakers)).toBeCloseTo(master * music * 0.5);
  soundSettings.set({ volume: 0.5 });
  expect(reach(bed, speakers)).toBeCloseTo(master * 0.5 * music * 0.5);
  soundSettings.set({ music: 0.4 });
  expect(reach(bed, speakers)).toBeCloseTo(master * 0.5 * music * 0.4 * 0.5);
  soundSettings.set({ muted: true });
  expect(media[0].paused).toBe(true);
  expect(contexts[0].suspend).toHaveBeenCalled();
  soundSettings.set({ muted: false });
  await vi.waitFor(() => expect(app.stats().menuPlaying).toBe(true));
  expect(media.at(-1)!.paused).toBe(false);
});

test("battle exit keeps the page context and prepared buffers but disconnects its graph and forgets its evidence", async () => {
  const app = setup();
  app.setScreen("menu");
  app.start();
  await vi.waitFor(() => expect(app.stats().menuReady).toBe(true));
  app.setScreen("loading");
  const first = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff }, 1);
  await vi.waitFor(() => expect(first.stats()?.loading).toBe(false));
  first.note(empty(8));
  first.update({
    clock: 8 / 30,
    motion: { vehicles: [], soldiers: [] },
    listener: cameraListener(camera, 0.25),
  });
  expect(first.stats()?.tick).toBe(8);
  const oldSources = sources.filter((s) => !s.stopped);
  const ambient = oldSources.at(-1)!.buffer;
  first.dispose();
  expect(first.stats()).toBeNull();
  expect(oldSources.every((s) => s.stopped && s.connections.size === 0)).toBe(true);
  expect(contexts[0].close).not.toHaveBeenCalled();
  const second = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff }, 1);
  await vi.waitFor(() => expect(second.stats()?.loading).toBe(false));
  expect(second.stats()).toMatchObject({ tick: -1, pending: 0 });
  second.note(empty(1));
  second.update({
    clock: 1 / 30,
    motion: { vehicles: [], soldiers: [] },
    listener: cameraListener(camera, 0.25),
  });
  expect(sources.at(-1)!.buffer).toBe(ambient);
  expect(contexts).toHaveLength(1);
  app.dispose();
  expect(contexts[0].close).toHaveBeenCalledTimes(1);
});

test("changing master volume preserves battle evidence, while mute and unmute discard it", async () => {
  const app = setup();
  const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff }, 1);
  await vi.waitFor(() => expect(battle.stats()?.loading).toBe(false));
  battle.note(empty(4));
  expect(battle.stats()?.tick).toBe(4);
  soundSettings.set({ volume: 0.3 });
  expect(battle.stats()?.tick).toBe(4);
  soundSettings.set({ muted: true });
  expect(battle.stats()).toMatchObject({ tick: -1, running: false });
  battle.note(empty(5));
  expect(battle.stats()?.tick).toBe(-1);
  soundSettings.set({ muted: false });
  await vi.waitFor(() => expect(battle.stats()?.running).toBe(true));
  expect(battle.stats()?.tick).toBe(-1);
});

test("a blocked early attempt resumes on menu input and the bed lasts until battle audio actually updates", async () => {
  const app = setup();
  let permitted = false;
  app.setScreen("menu");
  const context = contexts[0];
  context.state = "suspended";
  context.resume.mockImplementation(async () => {
    if (permitted) context.state = "running";
  });
  await vi.waitFor(() => expect(app.stats().menuReady).toBe(true));
  expect(app.stats().running).toBe(false);
  app.setScreen("loading");
  const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff }, 1);
  await vi.waitFor(() => expect(battle.stats()?.loading).toBe(false));
  battle.note(empty(4));
  battle.update({
    clock: 4 / 30,
    motion: { vehicles: [], soldiers: [] },
    listener: cameraListener(camera, 0.25),
  });
  expect(battle.stats()?.tick).toBe(-1);
  expect(sources.filter((s) => !s.stopped)).toHaveLength(1);
  permitted = true;
  window.dispatchEvent(new Event("pointerdown"));
  await vi.waitFor(() => expect(app.stats().running).toBe(true));
  expect(sources.filter((s) => !s.stopped)).toHaveLength(1);
  battle.note(empty(5));
  battle.update({
    clock: 5 / 30,
    motion: { vehicles: [], soldiers: [] },
    listener: cameraListener(camera, 0.25),
  });
  expect(app.stats().menuPlaying).toBe(false);
  expect(sources.filter((s) => !s.stopped)).toHaveLength(1);
  battle.dispose();
  app.setScreen("menu");
  expect(sources.filter((s) => !s.stopped)).toHaveLength(1);
  app.setScreen("other");
  expect(sources.filter((s) => !s.stopped)).toHaveLength(0);
});

test("repeated battle visits return every audio connection to the menu baseline", async () => {
  const app = setup();
  app.setScreen("menu");
  await vi.waitFor(() => expect(app.stats().menuReady).toBe(true));
  const baseline = nodes.reduce((n, node) => n + node.connections.size, 0);
  for (let visit = 0; visit < 5; visit++) {
    app.setScreen("loading");
    const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff }, 1);
    await vi.waitFor(() => expect(battle.stats()?.loading).toBe(false));
    battle.note(empty(1));
    battle.update({
      clock: 1 / 30,
      motion: { vehicles: [], soldiers: [] },
      listener: cameraListener(camera, 0.25),
    });
    battle.dispose();
    app.setScreen("menu");
    expect(nodes.reduce((n, node) => n + node.connections.size, 0)).toBe(baseline);
    expect(sources.filter((s) => !s.stopped)).toHaveLength(1);
  }
});

test("context admission failure stays explicit until manual recovery", () => {
  const app = setup();
  const admit = vi.fn(function () {
    throw new Error("Audio unavailable");
  });
  vi.stubGlobal("AudioContext", admit);
  app.setScreen("menu");
  window.dispatchEvent(new Event("pointerdown"));
  app.start();
  expect(app.stats()).toMatchObject({ error: "Audio unavailable", running: false });
  expect(admit).toHaveBeenCalledTimes(1);
});

test("quick mute then unmute resumes even while the earlier suspension is pending", async () => {
  const app = setup();
  app.setScreen("menu");
  await vi.waitFor(() => expect(app.stats().running).toBe(true));
  const context = contexts[0];
  let finish!: () => void;
  const suspended = new Promise<void>((resolve) => {
    finish = () => {
      context.state = "suspended";
      resolve();
    };
  });
  context.suspend.mockImplementation(() => suspended);
  context.resume.mockImplementation(async () => {
    await suspended;
    context.state = "running";
  });
  soundSettings.set({ muted: true });
  soundSettings.set({ muted: false });
  finish();
  await vi.waitFor(() => expect(app.stats().running).toBe(true));
});

test("resuming a browser-suspended context starts battle sound from fresh evidence", async () => {
  const app = setup();
  const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff }, 1);
  await vi.waitFor(() => expect(battle.stats()?.loading).toBe(false));
  battle.note(empty(4));
  expect(battle.stats()?.tick).toBe(4);
  contexts[0].state = "suspended";
  battle.note(empty(5));
  window.dispatchEvent(new Event("pointerdown"));
  await vi.waitFor(() => expect(battle.stats()?.running).toBe(true));
  expect(battle.stats()).toMatchObject({ tick: -1, pending: 0 });
});

test("menu controls play one quiet hover and a stronger click, respecting mute and disabled controls", async () => {
  const app = setup();
  app.setScreen("menu");
  await vi.waitFor(() => expect(app.stats().menuReady).toBe(true));
  const plate = document.createElement("main");
  plate.className = "menu";
  plate.innerHTML = "<button><span>Skirmish</span></button><button disabled>Unavailable</button>";
  document.body.append(plate);
  try {
    const button = plate.querySelector("button")!;
    const child = button.querySelector("span")!;
    const initial = sources.length;
    button.dispatchEvent(new MouseEvent("pointerover", { bubbles: true }));
    expect(sources.length).toBe(initial + 1);
    const hoverGain = [...sources.at(-1)!.connections][0].gain.value;
    child.dispatchEvent(new MouseEvent("pointerover", { bubbles: true, relatedTarget: button }));
    expect(sources.length).toBe(initial + 1);
    child.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(sources.length).toBe(initial + 2);
    expect([...sources.at(-1)!.connections][0].gain.value).toBeGreaterThan(hoverGain);
    expect(sources.at(-1)!.buffer.duration).toBeGreaterThan(sources.at(-2)!.buffer.duration);
    plate
      .querySelector("button:disabled")!
      .dispatchEvent(new MouseEvent("pointerover", { bubbles: true }));
    soundSettings.set({ muted: true });
    button.click();
    expect(sources.length).toBe(initial + 2);
    app.dispose();
    soundSettings.set({ muted: false });
    button.click();
    expect(sources.length).toBe(initial + 2);
  } finally {
    plate.remove();
  }
});

test("a battle's score shuffles by the battle's own seed", async () => {
  const app = setup();
  scoreSeeds.length = 0;
  for (const seed of [7, 7, 11]) {
    const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff }, seed);
    await vi.waitFor(() => expect(battle.stats()?.loading).toBe(false));
    battle.note(empty(1));
    battle.update({
      clock: 1 / 30,
      motion: { vehicles: [], soldiers: [] },
      listener: cameraListener(camera, 0.25),
    });
    battle.dispose();
  }
  expect(scoreSeeds).toEqual([7, 7, 11]);
});

// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import game from "@fixtures/game.json";
import { AppAudio } from "@packages/battle-audio/src/appAudio";
import { SOUNDS } from "@packages/battle-audio/src/synth";
import { soundSettings } from "@packages/battle-audio/src/settings";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import type { EffectPresentation } from "@packages/battle-renderer/src/effects/effectFrame";

const param = (value = 0) => ({
  value,
  setTargetAtTime(value: number) {
    this.value = value;
  },
  setValueAtTime(value: number) {
    this.value = value;
  },
  linearRampToValueAtTime() {},
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
const active: AppAudio[] = [];
afterEach(() => {
  active.splice(0).forEach((a) => a.dispose());
  nodes.length = 0;
  contexts.length = 0;
  sources.length = 0;
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
const presentation = game.presentation.audio as unknown as AudioPresentation;
const cookOff = (game.presentation.effects as unknown as EffectPresentation).cook_off;
const empty = (tick: number) => ({
  effects: { tick, shooters: [], segments: [], blasts: [], smokes: [] },
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
    "GainNode",
    "ConvolverNode",
    "BiquadFilterNode",
    "PannerNode",
    "StereoPannerNode",
  ])
    vi.stubGlobal(name, Node);
  vi.stubGlobal("AudioBufferSourceNode", Source);
  vi.stubGlobal("AudioContext", Context);
  soundSettings.set({ muted: false, volume: 1 });
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
    units: {},
    impacts: {},
    effects: {},
  };
  // A short fixture bed keeps lifecycle checks independent of musical content.
  catalog.sounds.menu_music = { ...catalog.sounds.countryside };
  catalog.sounds["recorded-clack-14"] = { ...catalog.sounds.rifle };
  catalog.sounds["recorded-clack-17"] = { ...catalog.sounds.cannon };
  const app = new AppAudio({ presentation, catalog });
  active.push(app);
  return app;
}

test("menu music starts at its introduction while ambience keeps a staggered loop phase", async () => {
  const app = setup();
  app.setScreen("menu");
  await vi.waitFor(() => expect(app.stats().menuReady).toBe(true));
  expect(sources[0].offset).toBe(0);
  expect(sources[1].offset).toBeGreaterThan(0);
});

test("battle exit keeps the page context and prepared buffers but disconnects its graph and forgets its evidence", async () => {
  const app = setup();
  app.setScreen("menu");
  app.start();
  await vi.waitFor(() => expect(app.stats().menuReady).toBe(true));
  app.setScreen("loading");
  const first = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff });
  await vi.waitFor(() => expect(first.stats()?.loading).toBe(false));
  first.note(empty(8));
  first.update(8 / 30, { vehicles: [], soldiers: [] }, camera);
  expect(first.stats()?.tick).toBe(8);
  const oldSources = sources.filter((s) => !s.stopped);
  const ambient = oldSources.at(-1)!.buffer;
  first.dispose();
  expect(first.stats()).toBeNull();
  expect(oldSources.every((s) => s.stopped && s.connections.size === 0)).toBe(true);
  expect(contexts[0].close).not.toHaveBeenCalled();
  const second = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff });
  await vi.waitFor(() => expect(second.stats()?.loading).toBe(false));
  expect(second.stats()).toMatchObject({ tick: -1, pending: 0 });
  second.note(empty(1));
  second.update(1 / 30, { vehicles: [], soldiers: [] }, camera);
  expect(sources.at(-1)!.buffer).toBe(ambient);
  expect(contexts).toHaveLength(1);
  app.dispose();
  expect(contexts[0].close).toHaveBeenCalledTimes(1);
});

test("changing master volume preserves battle evidence, while mute and unmute discard it", async () => {
  const app = setup();
  const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff });
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
  const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff });
  await vi.waitFor(() => expect(battle.stats()?.loading).toBe(false));
  battle.note(empty(4));
  battle.update(4 / 30, { vehicles: [], soldiers: [] }, camera);
  expect(battle.stats()?.tick).toBe(-1);
  expect(sources.filter((s) => !s.stopped)).toHaveLength(2);
  permitted = true;
  window.dispatchEvent(new Event("pointerdown"));
  await vi.waitFor(() => expect(app.stats().running).toBe(true));
  expect(sources.filter((s) => !s.stopped)).toHaveLength(2);
  battle.note(empty(5));
  battle.update(5 / 30, { vehicles: [], soldiers: [] }, camera);
  expect(app.stats().menuPlaying).toBe(false);
  expect(sources.filter((s) => !s.stopped)).toHaveLength(1);
  battle.dispose();
  app.setScreen("menu");
  expect(sources.filter((s) => !s.stopped)).toHaveLength(2);
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
    const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff });
    await vi.waitFor(() => expect(battle.stats()?.loading).toBe(false));
    battle.note(empty(1));
    battle.update(1 / 30, { vehicles: [], soldiers: [] }, camera);
    battle.dispose();
    app.setScreen("menu");
    expect(nodes.reduce((n, node) => n + node.connections.size, 0)).toBe(baseline);
    expect(sources.filter((s) => !s.stopped)).toHaveLength(2);
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
  const battle = app.createBattle({ tickHz: 30, presentation, smokeTimes: {}, cookOff });
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

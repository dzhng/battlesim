import { MemoryRouter } from "react-router";
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { SOUNDS } from "@packages/battle-audio/src/synthSounds.gen";
import { SoundWorkbench } from "../../apps/sound-workbench/src/SoundWorkbench";
import type { WorkbenchAPI, Snapshot } from "../../apps/sound-workbench/src/protocol";
import type { Auditioner, LiveMix } from "../../apps/sound-workbench/src/audition";

afterEach(cleanup);
const VEHICLE: Snapshot["vehicles"][string] = {
  engine: "engine_small",
  idle_rate: 0.8,
  load_rate: 1.4,
  idle_gain: 0.2,
  load_gain: 0.4,
  running: "wheels",
  running_gain: 0.3,
  full_speed_mps: 12,
  turret: null,
  turret_gain: 0,
  full_traverse_rps: 0.5,
  reverse: "reverse_beep",
  reverse_gain: 0.1,
};
// jsdom has no layout; scrolling is checked on the real browser route.
HTMLElement.prototype.scrollIntoView = () => {};
function fixture() {
  const catalog: Snapshot["catalog"] = {
    sources: {},
    clips: {},
    sounds: Object.fromEntries(
      Object.entries(SOUNDS).map(([id, sound]) => [
        id,
        {
          label: `Synth · ${id.replaceAll("_", " ")}`,
          clips: [],
          synth: id,
          synth_gain: 1,
          gain: 1,
          loop: sound.loop,
        },
      ]),
    ),
    music: { moods: { menu: [], tension: [], combat: [] }, tracks: {} },
  };
  catalog.sources.test = {
    label: "Test master",
    author: "Test",
    license: "CC0-1.0",
    url: "https://example.com/master",
    path: "assets/third-party/audio/test.wav",
    sha256: "a".repeat(64),
    notes: "",
  };
  catalog.clips.reload = {
    label: "Unused reload",
    category: "mechanical",
    role: "reload",
    source: "test",
    source_rate: 48000,
    source_frames: [1, 24001],
    processing: "clean",
    url: "/audio/clips/reload.wav",
    sha256: "b".repeat(64),
    sample_rate: 48000,
    frames: 24000,
    loop: false,
    notes: "Kept for future events",
  };
  const snapshot: Snapshot = {
    revision: "saved",
    catalog,
    looks: {
      rifle: { sound: { shot: { near: "rifle", far: "rifle_far", gain: 0.35 }, impacts: {} } },
    },
    effects: {},
    vehicles: {
      default: VEHICLE,
      tracked_heavy: { ...VEHICLE, engine: "engine_diesel", turret: "turret" },
    },
    footsteps: { sound: "footstep", gain: 0.07, stride_m: 0.8, max_step_m: 2 },
    runMps: 2,
    music: {
      duck: { depth: 0.35, attack_s: 0.05, release_s: 0.8 },
      combat_window_s: 20,
      calm_s: 45,
      crossfade_s: 4,
      combat_triggers: ["under_fire"],
    },
  };
  let saved = snapshot;
  const api: WorkbenchAPI = {
    snapshot: async () => saved,
    preview: vi.fn(async (draft) => ({
      candidateId: "reviewed",
      revision: draft.revision,
      files: [
        {
          path: "fixtures/sounds.json",
          before: JSON.stringify(saved.catalog),
          after: JSON.stringify(draft.catalog),
        },
      ],
    })),
    save: vi.fn(async () => saved),
  };
  const mix: LiveMix = { loops: vi.fn(), hit: vi.fn() };
  const audition: Auditioner = {
    play: vi.fn(async () => {}),
    live: vi.fn(async () => mix),
    voice: vi.fn(async () => {}),
    music: vi.fn(),
    lines: vi.fn(async () => null),
    stop: vi.fn(),
  };
  return {
    snapshot,
    api,
    audition,
    mix,
    publish(catalog: Snapshot["catalog"]) {
      saved = { ...snapshot, revision: "new", catalog };
    },
  };
}

test("the complete library auditions an unused reload and clones synthesis without changing its baseline", async () => {
  const { api, audition } = fixture();
  render(<SoundWorkbench api={api} audition={audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Unused reload" }));
  fireEvent.click(screen.getByRole("button", { name: "Play clip" }));
  await waitFor(() =>
    expect(vi.mocked(audition.play).mock.calls[0]?.slice(1, 3)).toEqual(["clip", "reload"]),
  );
  expect(screen.getByText("Kept for future events")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Synth · rifle" }));
  expect(screen.queryByLabelText("Recipe label")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Clone recipe" }));
  fireEvent.change(screen.getByLabelText("Recipe label"), { target: { value: "Custom rifle" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  const draft = vi.mocked(api.preview).mock.calls[0][0];
  expect(draft.catalog.sounds.rifle.label).toBe("Synth · rifle");
  expect(
    Object.values(draft.catalog.sounds).some((recipe) => recipe.label === "Custom rifle"),
  ).toBe(true);
});

test("library selection, filtering and view changes stay usable when scrolling completes asynchronously", async () => {
  const scrolling = vi
    .spyOn(HTMLElement.prototype, "scrollIntoView")
    .mockImplementation(async () => {});
  try {
    const { api, audition } = fixture();
    render(<SoundWorkbench api={api} audition={audition} />, { wrapper: MemoryRouter });
    await screen.findByRole("button", { name: "Unused reload" });
    fireEvent.click(screen.getByRole("button", { name: "Synth · rifle" }));
    expect(screen.getByRole("heading", { name: "Synth · rifle" })).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Library filter"), { target: { value: "baselines" } });
    fireEvent.click(screen.getByRole("button", { name: "Synth · hmg" }));
    expect(screen.getByRole("heading", { name: "Synth · hmg" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Movement" }));
    expect(screen.getByRole("heading", { name: "tracked_heavy" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Library & recipes" }));
    expect(screen.getByRole("heading", { name: "Synth · hmg" })).toBeTruthy();
  } finally {
    scrolling.mockRestore();
  }
});

test("unused clips include stored recipes that no battle assignment selects", async () => {
  const f = fixture();
  f.snapshot.catalog.sounds.handling = {
    label: "Stored handling recipe",
    clips: ["reload"],
    synth: null,
    synth_gain: 0,
    gain: 1,
    loop: false,
  };
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  await screen.findByRole("button", { name: "Unused reload" });
  fireEvent.change(screen.getByLabelText("Library filter"), { target: { value: "unused" } });
  expect(screen.getByRole("button", { name: "Unused reload" })).toBeTruthy();
});

test("preview shows the changed sound setting without searching full source JSON", async () => {
  const f = fixture();
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Synth · rifle" }));
  fireEvent.click(screen.getByRole("button", { name: "Clone recipe" }));
  fireEvent.change(screen.getByLabelText("Recipe label"), { target: { value: "Edited recipe" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  const changes = screen.getByRole("region", { name: "Changed sound settings" });
  expect(changes.textContent).toContain("sounds.custom_rifle.label");
  expect(changes.textContent).toContain("Edited recipe");
});

test("cloning a filtered baseline shows the new selected recipe in the library", async () => {
  const f = fixture();
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  await screen.findByRole("button", { name: "Synth · rifle" });
  fireEvent.change(screen.getByLabelText("Library filter"), { target: { value: "baselines" } });
  fireEvent.click(screen.getByRole("button", { name: "Synth · rifle" }));
  fireEvent.click(screen.getByRole("button", { name: "Clone recipe" }));
  expect(
    screen.getByRole("button", { name: "Synth · rifle · custom" }).getAttribute("aria-pressed"),
  ).toBe("true");
});

test("a vehicle class auditions the loops the battle plays for it, replacements included", async () => {
  const f = fixture();
  f.snapshot.effects.engine_diesel = "engine_small";
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Movement" }));
  fireEvent.click(screen.getByRole("button", { name: "Play tracked_heavy engine" }));
  await waitFor(() => expect(f.audition.play).toHaveBeenCalled());
  const [, kind, id] = vi.mocked(f.audition.play).mock.calls[0];
  expect([kind, id]).toEqual(["sound", "engine_small"]);
  // A class without a turret has nothing to play for it.
  expect(screen.queryByRole("button", { name: "Play default turret" })).toBeNull();
});

test("driving a vehicle class mixes its loops as the battle does at that speed", async () => {
  const f = fixture();
  f.snapshot.effects.engine_diesel = "engine_small";
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Movement" }));
  fireEvent.change(screen.getByLabelText("tracked_heavy speed"), { target: { value: "12" } });
  fireEvent.click(screen.getByRole("button", { name: "Drive tracked_heavy" }));
  await waitFor(() => expect(f.mix.loops).toHaveBeenCalled());
  // Full speed: the engine at its load rate and level, running gear at full.
  const layers = vi.mocked(f.mix.loops).mock.lastCall![0];
  expect(layers.map(({ sound, gain, rate }) => ({ sound, gain, rate }))).toEqual([
    { sound: "engine_small", gain: 0.4, rate: 1.4 },
    { sound: "wheels", gain: 0.3, rate: 1.5 },
  ]);
  // Stopped, the engine idles and the running gear falls silent.
  fireEvent.change(screen.getByLabelText("tracked_heavy speed"), { target: { value: "0" } });
  expect(vi.mocked(f.mix.loops).mock.lastCall![0].map((l) => [l.sound, l.rate])).toEqual([
    ["engine_small", 0.8],
  ]);
});

test("footsteps fall once a stride at the chosen pace", async () => {
  const f = fixture();
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Movement" }));
  fireEvent.click(screen.getByRole("button", { name: "Play footsteps" }));
  await waitFor(() => expect(vi.mocked(f.mix.hit).mock.calls.length).toBeGreaterThan(1));
  const delays = vi.mocked(f.mix.hit).mock.calls.map(([sound, , delay]) => {
    expect(sound).toBe("footstep");
    return delay;
  });
  // One soldier at 2 m/s with a 0.8 m stride: a step each 0.4 s.
  expect(delays[1] - delays[0]).toBeCloseTo(0.4);
});

test("an edit after preview invalidates review, and a stale-save error retains the draft", async () => {
  const { api, audition } = fixture();
  api.save = vi.fn(async () => {
    throw new Error("Outside edit; reload sources");
  });
  render(<SoundWorkbench api={api} audition={audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Synth · rifle" }));
  fireEvent.click(screen.getByRole("button", { name: "Clone recipe" }));
  fireEvent.change(screen.getByLabelText("Recipe label"), { target: { value: "Edited recipe" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  fireEvent.click(await screen.findByRole("button", { name: "Save reviewed JSON" }));
  await screen.findByText("Outside edit; reload sources");
  expect((screen.getByLabelText("Recipe label") as HTMLInputElement).value).toBe("Edited recipe");
  fireEvent.change(screen.getByLabelText("Recipe label"), { target: { value: "Next recipe" } });
  expect(screen.queryByRole("button", { name: "Save reviewed JSON" })).toBeNull();
});

test.each(["Stop", "selection", "unmount"])(
  "%s cancels preparation and never revives the older audition",
  async (action) => {
    const { api, audition } = fixture();
    let complete!: () => void;
    audition.play = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          complete = resolve;
        }),
    );
    const view = render(<SoundWorkbench api={api} audition={audition} />, {
      wrapper: MemoryRouter,
    });
    fireEvent.click(await screen.findByRole("button", { name: "Synth · rifle" }));
    fireEvent.click(screen.getByRole("button", { name: "Play recipe" }));
    await screen.findByText("Preparing audition…");
    vi.mocked(audition.stop).mockClear();
    if (action === "Stop") fireEvent.click(screen.getByRole("button", { name: "Stop audition" }));
    else if (action === "selection")
      fireEvent.click(screen.getByRole("button", { name: "Synth · hmg" }));
    else view.unmount();
    expect(audition.stop).toHaveBeenCalledOnce();
    complete();
    await waitFor(() => expect(screen.queryByText("Preparing audition…")).toBeNull());
    expect(screen.queryByText("Audition · Synth · rifle")).toBeNull();
  },
);

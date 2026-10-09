import { MemoryRouter } from "react-router";
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { SOUNDS } from "@packages/battle-audio/src/synth";
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
  reverse: "reverse_whine",
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
    defaults: {},
    impacts: {},
    effects: {},
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
    // A carbine derives from the rifle; a cannon has no firing row at all.
    weapons: { rifle: {}, carbine: { extends: "rifle" }, cannon: {} },
    firing: {
      default: { near: "hmg", far: "hmg_far", gain: 0.5, far_m: 300 },
      rifle: { near: "rifle", far: "rifle_far", gain: 0.35, far_m: 300 },
    },
    vehicles: {
      default: VEHICLE,
      tracked_heavy: { ...VEHICLE, engine: "engine_diesel", turret: "turret" },
    },
    footsteps: { sound: "footstep", gain: 0.07, stride_m: 0.8, max_step_m: 2 },
    runMps: 2,
    materials: ["ground", "hull"],
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

test("the global editor displays the effective shared fallback for an unassigned firing kind", async () => {
  const f = fixture();
  f.snapshot.catalog.defaults.default = { near: "hmg", far: "hmg", gain: 0.8 };
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Defaults & effects" }));
  expect((screen.getByLabelText("rifle near") as HTMLSelectElement).value).toBe("hmg");
  expect((screen.getByLabelText("rifle gain") as HTMLInputElement).value).toBe("0.8");
});

test("implicit firing fallbacks display effect replacements and retain them when editing gain", async () => {
  const f = fixture();
  f.snapshot.catalog.effects.rifle = "hmg";
  f.snapshot.catalog.effects.rifle_far = "hmg_far";
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Defaults & effects" }));
  expect((screen.getByLabelText("rifle near") as HTMLSelectElement).value).toBe("hmg");
  expect((screen.getByLabelText("rifle far") as HTMLSelectElement).value).toBe("hmg_far");
  fireEvent.change(screen.getByLabelText("rifle gain"), { target: { value: "0.8" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  expect(vi.mocked(f.api.preview).mock.calls[0][0].catalog.defaults.rifle).toEqual({
    near: "hmg",
    far: "hmg_far",
    gain: 0.8,
  });
});

test("preview shows the changed sound setting without searching full source JSON", async () => {
  const f = fixture();
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Defaults & effects" }));
  fireEvent.change(screen.getByLabelText("rifle near"), { target: { value: "hmg" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  const changes = screen.getByRole("region", { name: "Changed sound settings" });
  expect(changes.textContent).toContain("defaults.rifle.near");
  expect(changes.textContent).toContain("hmg");
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

test("a derived weapon row fires its ancestor's choice until given its own, then restores to it", async () => {
  const f = fixture();
  f.snapshot.catalog.defaults.rifle = { near: "rifle", far: "rifle_far", gain: 1 };
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Defaults & effects" }));
  expect(screen.getByText("inherited from rifle")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("carbine near"), { target: { value: "hmg" } });
  // The ancestor keeps its own choice.
  expect((screen.getByLabelText("rifle near") as HTMLSelectElement).value).toBe("rifle");
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  const draft = vi.mocked(f.api.preview).mock.calls[0][0];
  expect(draft.catalog.defaults.carbine.near).toBe("hmg");
  expect(draft.catalog.defaults.rifle.near).toBe("rifle");
  f.publish(draft.catalog);
  fireEvent.click(screen.getByRole("button", { name: "Save reviewed JSON" }));
  await screen.findByText("Saved. Reload or open a battle page to use this generation.");
  expect(f.api.save).toHaveBeenCalledWith({ candidateId: "reviewed", revision: "saved" });
  fireEvent.click(screen.getByRole("button", { name: "Restore carbine firing fallback" }));
  expect((screen.getByLabelText("carbine near") as HTMLSelectElement).value).toBe("rifle");
});

test("a weapon row with no styled ancestor fires the default row's baseline", async () => {
  const f = fixture();
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Defaults & effects" }));
  expect((screen.getByLabelText("cannon near") as HTMLSelectElement).value).toBe("hmg");
});

test("a vehicle class auditions the loops the battle plays for it, replacements included", async () => {
  const f = fixture();
  f.snapshot.catalog.effects.engine_diesel = "engine_small";
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
  f.snapshot.catalog.effects.engine_diesel = "engine_small";
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

test("the footstep is chosen among footstep recordings and walks with the choice", async () => {
  const f = fixture();
  f.snapshot.catalog.clips.step = {
    ...f.snapshot.catalog.clips.reload,
    label: "Boot on dirt",
    category: "footstep",
    role: "other",
  };
  f.snapshot.catalog.sounds.boots = {
    label: "Boots on dirt",
    clips: ["step"],
    synth: null,
    synth_gain: 0,
    gain: 1,
    loop: false,
  };
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Movement" }));
  const choice = screen.getByLabelText("Footstep sound") as HTMLSelectElement;
  // Only footstep recordings are offered, beside the original synthesis.
  expect([...choice.options].map((o) => o.text)).toEqual(["Original synthesis", "Boots on dirt"]);
  fireEvent.change(choice, { target: { value: "boots" } });
  fireEvent.click(screen.getByRole("button", { name: "Play footsteps" }));
  await waitFor(() => expect(f.mix.hit).toHaveBeenCalled());
  expect(vi.mocked(f.audition.live).mock.lastCall![1]).toEqual(["boots"]);
  expect(vi.mocked(f.mix.hit).mock.calls[0][0]).toBe("boots");
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
  fireEvent.click(await screen.findByRole("button", { name: "Defaults & effects" }));
  fireEvent.change(screen.getByLabelText("rifle near"), { target: { value: "hmg" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  fireEvent.click(await screen.findByRole("button", { name: "Save reviewed JSON" }));
  await screen.findByText("Outside edit; reload sources");
  expect((screen.getByLabelText("rifle near") as HTMLSelectElement).value).toBe("hmg");
  fireEvent.change(screen.getByLabelText("rifle near"), { target: { value: "cannon" } });
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

test("global firing, material impacts and matching loop slots publish the chosen recipes", async () => {
  const { api, audition } = fixture();
  render(<SoundWorkbench api={api} audition={audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Defaults & effects" }));
  fireEvent.change(screen.getByLabelText("rifle near"), { target: { value: "hmg" } });
  fireEvent.change(screen.getByLabelText("ground rifle impact"), {
    target: { value: "impact_hull" },
  });
  fireEvent.change(screen.getByLabelText("motor effect"), { target: { value: "engine_diesel" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  const catalog = vi.mocked(api.preview).mock.calls[0][0].catalog;
  expect(catalog.defaults.rifle.near).toBe("hmg");
  expect(catalog.impacts.ground.rifle).toBe("impact_hull");
  expect(catalog.effects.motor).toBe("engine_diesel");
});

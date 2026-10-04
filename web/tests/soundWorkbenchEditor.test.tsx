import { MemoryRouter } from "react-router";
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { SOUNDS } from "@packages/battle-audio/src/synth";
import { SoundWorkbench } from "../../apps/sound-workbench/src/SoundWorkbench";
import type { WorkbenchAPI, Snapshot } from "../../apps/sound-workbench/src/protocol";
import type { Auditioner } from "../../apps/sound-workbench/src/audition";

afterEach(cleanup);
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
    units: {},
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
    units: [
      { id: "alpha", name: "Alpha", mounts: [{ name: "rifle", weapons: ["rifle"], count: 2 }] },
      { id: "bravo", name: "Bravo", mounts: [{ name: "rifle", weapons: ["rifle"], count: 1 }] },
    ],
    firing: { rifle: { near: "rifle", far: "rifle_far", gain: 0.35, far_m: 300 } },
    materials: ["ground", "hull"],
    rounds: ["rifle"],
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
  const audition: Auditioner = { play: vi.fn(async () => {}), stop: vi.fn() };
  return {
    snapshot,
    api,
    audition,
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
    fireEvent.click(screen.getByRole("button", { name: "Unit assignments" }));
    expect(screen.getByRole("button", { name: "Alpha" })).toBeTruthy();
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
  fireEvent.click(screen.getByRole("button", { name: "Unit assignments" }));
  fireEvent.click(screen.getByRole("button", { name: "Alpha" }));
  expect((screen.getByLabelText("rifle near") as HTMLSelectElement).value).toBe("hmg");
  fireEvent.change(screen.getByLabelText("rifle gain"), { target: { value: "0.8" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  expect(vi.mocked(f.api.preview).mock.calls[0][0].catalog.units.alpha.rifle).toEqual({
    near: "hmg",
    far: "hmg_far",
    gain: 0.8,
  });
});

test("preview shows the changed sound setting without searching full source JSON", async () => {
  const f = fixture();
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Unit assignments" }));
  fireEvent.click(screen.getByRole("button", { name: "Alpha" }));
  fireEvent.change(screen.getByLabelText("rifle near"), { target: { value: "hmg" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  const changes = screen.getByRole("region", { name: "Changed sound settings" });
  expect(changes.textContent).toContain("units.alpha.rifle.near");
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

test("exact type assignments can be edited independently, restored, reviewed and saved", async () => {
  const f = fixture();
  render(<SoundWorkbench api={f.api} audition={f.audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Unit assignments" }));
  fireEvent.click(screen.getByRole("button", { name: "Alpha" }));
  expect(screen.getByText("2 physical mounts share this choice")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("rifle near"), { target: { value: "hmg" } });
  fireEvent.click(screen.getByRole("button", { name: "Bravo" }));
  expect((screen.getByLabelText("rifle near") as HTMLSelectElement).value).toBe("rifle");
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("button", { name: "Save reviewed JSON" });
  const draft = vi.mocked(f.api.preview).mock.calls[0][0];
  expect(draft.catalog.units.alpha.rifle.near).toBe("hmg");
  expect(draft.catalog.units.bravo).toBeUndefined();
  f.publish(draft.catalog);
  fireEvent.click(screen.getByRole("button", { name: "Save reviewed JSON" }));
  await screen.findByText("Saved. Reload or open a battle page to use this generation.");
  expect(f.api.save).toHaveBeenCalledWith({ candidateId: "reviewed", revision: "saved" });
  fireEvent.click(screen.getByRole("button", { name: "Alpha" }));
  fireEvent.click(screen.getByRole("button", { name: "Restore rifle fallback" }));
  expect((screen.getByLabelText("rifle near") as HTMLSelectElement).value).toBe("rifle");
});

test("an edit after preview invalidates review, and a stale-save error retains the draft", async () => {
  const { api, audition } = fixture();
  api.save = vi.fn(async () => {
    throw new Error("Outside edit; reload sources");
  });
  render(<SoundWorkbench api={api} audition={audition} />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Unit assignments" }));
  fireEvent.click(screen.getByRole("button", { name: "Alpha" }));
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

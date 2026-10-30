import { MemoryRouter } from "react-router";
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { SoundWorkbench } from "../../apps/sound-workbench/src/SoundWorkbench";
import type { MusicAPI, Snapshot, WorkbenchAPI } from "../../apps/sound-workbench/src/protocol";
import type { Auditioner } from "../../apps/sound-workbench/src/audition";
import type { VoiceAPI } from "../../apps/sound-workbench/src/voices/protocol";

afterEach(cleanup);

const generated = (id: string) => ({
  kind: "Generated" as const,
  generator: "ElevenLabs music",
  model: "music_v1",
  prompt: `Background underscore · ${id}`,
  length_s: 150,
  date: "2026-10-12",
  plan: "Duet ElevenLabs" as const,
  path: `assets/source/audio/music/${id}.00000000.mp3`,
  sha256: "a".repeat(64),
});
function fixture() {
  const combat = ["combat_1", "combat_2", "combat_3"];
  const tension = ["tension_sparse_1", "tension_sparse_2", "tension_dark_1"];
  const catalog: SoundCatalog = {
    sources: Object.fromEntries([...combat, ...tension].map((id) => [id, generated(id)])),
    clips: {},
    sounds: {},
    music: {
      moods: { menu: [], tension, combat },
      tracks: Object.fromEntries(
        [...combat, ...tension].map((id) => [
          id,
          { source: id, sha256: "b".repeat(64), gain: 0.3 },
        ]),
      ),
    },
  };
  const snapshot = {
    revision: "sounds-1",
    catalog,
    looks: {},
    weaponDefaults: { blast: {}, impacts: {} },
    effects: {},
    vehicles: {},
    footsteps: {},
    runMps: 4,
    music: {
      duck: { depth: 0.35, attack_s: 0.05, release_s: 0.8 },
      combat_window_s: 20,
      calm_s: 45,
      crossfade_s: 4,
      combat_triggers: ["under_fire", "suppressed", "pinned", "hit", "man_down"],
    },
  } as unknown as Snapshot;
  const sounds: WorkbenchAPI = {
    snapshot: vi.fn(async () => snapshot),
    preview: vi.fn(),
    save: vi.fn(),
  };
  /** The real player's surface: what it was last told to crossfade to, and over how long. */
  const player = { crossfadeTo: vi.fn(), stop: vi.fn() };
  const audition: Auditioner = {
    play: vi.fn(async () => {}),
    live: vi.fn(),
    voice: vi.fn(async () => {}),
    music: vi.fn(async () => player),
    lines: vi.fn(async () => null),
    stop: vi.fn(),
  };
  const music: MusicAPI = {
    regenerate: vi.fn(async (track: string) => ({
      job: { id: "1", args: ["music", "--track", track], status: "queued" as const, lines: [] },
    })),
    jobs: vi.fn(async () => ({ jobs: [] })),
  };
  return { sounds, audition, music, player, combat, tension };
}
async function open() {
  const f = fixture();
  render(
    <SoundWorkbench api={f.sounds} voices={{} as VoiceAPI} music={f.music} audition={f.audition} />,
    { wrapper: MemoryRouter },
  );
  fireEvent.click(await screen.findByRole("button", { name: "Music" }));
  return f;
}
const section = (name: string) => screen.getByRole("region", { name });

test("opening the Music tab is silent and lists each mood's tracks", async () => {
  const f = await open();
  for (const [mood, ids] of [
    ["Combat", f.combat],
    ["Tension", f.tension],
  ] as const) {
    const rows = within(section(mood));
    for (const id of ids) expect(rows.getByText(id)).toBeTruthy();
  }
  expect(f.audition.music).not.toHaveBeenCalled();
});

test("the mood switch crossfades the real player into a track of that mood over 4 s", async () => {
  const f = await open();
  fireEvent.click(screen.getByRole("button", { name: "Combat mood" }));
  await vi.waitFor(() => expect(f.player.crossfadeTo).toHaveBeenCalledTimes(1));
  const [combat, seconds] = f.player.crossfadeTo.mock.calls[0];
  expect(f.combat).toContain(combat);
  expect(seconds).toBe(4);

  fireEvent.click(screen.getByRole("button", { name: "Tension mood" }));
  await vi.waitFor(() => expect(f.player.crossfadeTo).toHaveBeenCalledTimes(2));
  const [tension, again] = f.player.crossfadeTo.mock.calls[1];
  expect(f.tension).toContain(tension);
  expect(again).toBe(4);
});

test("Regenerate asks the generation tool for exactly that track", async () => {
  const f = await open();
  const row = screen.getByText("combat_2").closest("li")!;
  fireEvent.click(within(row).getByRole("button", { name: "Regenerate" }));
  await vi.waitFor(() => expect(f.music.regenerate).toHaveBeenCalledWith("combat_2"));
  expect(await screen.findByText(/music --track combat_2/)).toBeTruthy();
});

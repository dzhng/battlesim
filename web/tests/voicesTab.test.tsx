import { MemoryRouter } from "react-router";
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import {
  ALL_TRIGGERS,
  voiceUrl,
  type Trigger,
  type VoiceCatalog,
  type VoiceTake,
} from "@packages/battle-audio/src/voices/voiceCatalog";
import { SoundWorkbench } from "../../apps/sound-workbench/src/SoundWorkbench";
import type { Snapshot, WorkbenchAPI } from "../../apps/sound-workbench/src/protocol";
import {
  candidateUrl,
  type TakeView,
  type VerdictRequest,
  type VoiceAPI,
  type VoiceSnapshot,
} from "../../apps/sound-workbench/src/voices/protocol";
import type { Auditioner } from "../../apps/sound-workbench/src/audition";

afterEach(cleanup);
HTMLElement.prototype.scrollIntoView = () => {};

const FLAGGED = "tank/lead/move/stressed/0";
const MOVE = "tank/lead/move/steady/0";
const STOP = "tank/lead/stop/steady/1";
const ACCEPTED = "tank/lead/attack/steady/0";
const take = (id: string): VoiceTake => {
  const [set, voice, trigger, intensity, index] = id.split("/");
  return {
    set,
    voice,
    trigger: trigger as Trigger,
    intensity: intensity as VoiceTake["intensity"],
    take: Number(index),
  };
};
const view = (text: string, sha: string, extra: Partial<TakeView> = {}): TakeView => ({
  text,
  sha256: sha.repeat(64),
  duration_s: 1.3,
  transcript: text,
  match: { score: 1, missing: [], overlong: false, flagged: false },
  verdict: null,
  ...extra,
});
function fixture() {
  const line = (text: string) => ({
    steady: [`${text}.`, `${text}, steady.`] as [string, string],
    stressed: [`${text}!`, `${text}, now!`] as [string, string],
  });
  const catalog: VoiceCatalog = {
    roles: {
      crew: {
        lines: Object.fromEntries(ALL_TRIGGERS.map((t) => [t, line(t)])) as Record<
          Trigger,
          ReturnType<typeof line>
        >,
      },
    },
    sets: {
      tank: {
        role: "crew",
        country: "us",
        callsign: "Oscar 55",
        processing: "voice-radio-light",
        voices: [
          { id: "lead", speaks: "responses", gender: "male", description: "Lead", voice_id: "v1" },
          { id: "soldier", speaks: "barks", gender: "male", description: "Crew", voice_id: null },
        ],
      },
    },
    assignments: {},
    retakes: {},
  };
  const snapshot: VoiceSnapshot = {
    revision: "voices-1",
    catalog,
    takes: {
      [FLAGGED]: view("move!", "b", {
        transcript: "groove",
        match: { score: 0.4, missing: [], overlong: false, flagged: true },
      }),
      [MOVE]: view("move.", "a"),
      [STOP]: view("stop, steady.", "c"),
      [ACCEPTED]: view("attack.", "d", { verdict: "ok" }),
    },
    counts: {
      tank: { expected: 164, takes: 4, unreviewed: 3, flagged: 1, ok: 1, bad: 0 },
    },
    designs: {
      "tank/soldier": {
        text: "Moving out. Taking fire! Get out, get out!",
        candidates: [
          { transcript: "Moving out. Taking fire! Get out, get out!", score: 1, duration_s: 9.1 },
          { transcript: "Moving out. Taking fire! Get out!", score: 0.88, duration_s: 8.4 },
          { transcript: "Moving. Taking fire! Get out, get out!", score: 0.88, duration_s: 10.2 },
        ],
        picked: null,
      },
      "tank/lead": {
        text: "Moving out. Taking fire! Get out, get out!",
        candidates: [
          { transcript: "Moving out.", score: 0.9, duration_s: 9.0 },
          { transcript: "Moving out. Taking fire!", score: 0.95, duration_s: 9.4 },
          { transcript: "Moving out, taking fire.", score: 0.92, duration_s: 9.2 },
        ],
        picked: 2,
      },
    },
  };
  const voices: VoiceAPI = {
    snapshot: vi.fn(async () => snapshot),
    preview: vi.fn(async (draft) => ({
      candidateId: "reviewed",
      revision: draft.revision,
      files: [{ path: "fixtures/voices.json", before: "{}", after: JSON.stringify(draft.catalog) }],
    })),
    save: vi.fn(async () => snapshot),
    verdict: vi.fn(async ({ verdict, takes }: VerdictRequest) => ({
      takes: Object.fromEntries(
        takes.map(({ take: id }) => [id, { ...snapshot.takes[id], verdict }]),
      ),
      counts: snapshot.counts,
      changed: [],
    })),
    generate: vi.fn(async () => ({
      job: { id: "1", args: [], status: "queued" as const, lines: [] },
    })),
    jobs: vi.fn(async () => ({ jobs: [] })),
    design: vi.fn(async () => ({
      job: { id: "2", args: [], status: "queued" as const, lines: [] },
    })),
    pick: vi.fn(async () => snapshot),
  };
  const sounds: WorkbenchAPI = {
    snapshot: () => new Promise<Snapshot>(() => {}),
    preview: vi.fn(),
    save: vi.fn(),
  };
  const audition: Auditioner = {
    play: vi.fn(async () => {}),
    live: vi.fn(),
    voice: vi.fn(async () => {}),
    music: vi.fn(),
    lines: vi.fn(async () => null),
    stop: vi.fn(),
  };
  return { voices, sounds, audition };
}
const press = (key: string) => fireEvent.keyDown(document.body, { key });
const heard = (audition: Auditioner) =>
  vi.mocked(audition.voice).mock.calls.at(-1)?.[0] as string[] | undefined;
const url = (id: string, sha: string) => voiceUrl(take(id), sha.repeat(64));

async function open() {
  const f = fixture();
  render(<SoundWorkbench api={f.sounds} voices={f.voices} audition={f.audition} />, {
    wrapper: MemoryRouter,
  });
  fireEvent.click(screen.getByRole("button", { name: "Voices" }));
  await screen.findByRole("heading", { name: "tank" });
  return f;
}

test("the review queue plays flagged takes first, then one take per voice and trigger", async () => {
  const { audition } = await open();
  // Opening the tab stays silent.
  expect(audition.voice).not.toHaveBeenCalled();
  press(" ");
  expect(heard(audition)).toEqual([url(FLAGGED, "b")]);
  press("j");
  expect(heard(audition)).toEqual([url(MOVE, "a")]);
  press("j");
  expect(heard(audition)).toEqual([url(STOP, "c")]);
  // An accepted take has been reviewed; the queue ends here.
  press("j");
  expect(heard(audition)).toEqual([url(STOP, "c")]);
  press("k");
  expect(heard(audition)).toEqual([url(MOVE, "a")]);
});

test("A and X judge the take that was heard and move on", async () => {
  const { voices, audition } = await open();
  press("a");
  await waitFor(() =>
    expect(voices.verdict).toHaveBeenCalledWith({
      verdict: "ok",
      takes: [{ take: FLAGGED, sha256: "b".repeat(64) }],
    }),
  );
  await waitFor(() => expect(heard(audition)).toEqual([url(MOVE, "a")]));
  press("x");
  await waitFor(() =>
    expect(voices.verdict).toHaveBeenLastCalledWith({
      verdict: "bad",
      takes: [{ take: MOVE, sha256: "a".repeat(64) }],
    }),
  );
  const chip = screen.getByRole("button", { name: /^lead move steady 1/ });
  await waitFor(() => expect(chip.dataset.verdict).toBe("bad"));
});

test("R marks bad, bumps the take's retakes and regenerates exactly that take", async () => {
  const { voices } = await open();
  press("r");
  await waitFor(() => expect(voices.generate).toHaveBeenCalled());
  expect(voices.verdict).toHaveBeenCalledWith({
    verdict: "bad",
    takes: [{ take: FLAGGED, sha256: "b".repeat(64) }],
  });
  expect(vi.mocked(voices.preview).mock.calls[0][0].catalog.retakes).toEqual({ [FLAGGED]: 1 });
  expect(voices.save).toHaveBeenCalledWith({ candidateId: "reviewed", revision: "voices-1" });
  expect(voices.generate).toHaveBeenCalledWith({
    sets: ["tank"],
    voices: ["lead"],
    triggers: ["move"],
    intensities: ["stressed"],
    take: 0,
  });
});

test("E edits the current line's text for its role", async () => {
  const { voices } = await open();
  press("e");
  const editor = screen.getByRole("textbox", { name: "Line text" });
  // Typing in the editor is text, not review keys.
  fireEvent.keyDown(editor, { key: "a" });
  expect(voices.verdict).not.toHaveBeenCalled();
  fireEvent.change(editor, { target: { value: "[shouting] Go, go!" } });
  fireEvent.click(screen.getByRole("button", { name: "Save text" }));
  await waitFor(() => expect(voices.save).toHaveBeenCalled());
  expect(vi.mocked(voices.preview).mock.calls[0][0].catalog.roles.crew.lines.move.stressed).toEqual(
    ["[shouting] Go, go!", "move, now!"],
  );
});

test("the set list counts what needs review, and the grid marks flagged and judged takes", async () => {
  await open();
  const set = screen.getByRole("button", { name: /^tank/ });
  expect(within(set).getByText("3 unreviewed")).toBeTruthy();
  expect(within(set).getByText("1 flagged")).toBeTruthy();
  expect(screen.getByRole("button", { name: /^lead move stressed 1/ }).dataset.flagged).toBe(
    "true",
  );
  expect(screen.getByRole("button", { name: /^lead attack steady 1/ }).dataset.verdict).toBe("ok");
  // A voice still waiting for its designed voice has no takes to play.
  expect(
    screen.getByRole("button", { name: /^soldier contact_infantry steady 1/ }).dataset.verdict,
  ).toBe("missing");
});

test("Play row plays a voice's lines back to back, and bulk accept judges only unflagged takes", async () => {
  const { voices, audition } = await open();
  fireEvent.click(screen.getByRole("button", { name: "Play row lead" }));
  expect(heard(audition)).toEqual([url(MOVE, "a"), url(ACCEPTED, "d"), url(STOP, "c")]);
  fireEvent.click(screen.getByRole("button", { name: "Accept unflagged in set" }));
  await waitFor(() =>
    expect(voices.verdict).toHaveBeenCalledWith({
      verdict: "ok",
      takes: [
        { take: MOVE, sha256: "a".repeat(64) },
        { take: STOP, sha256: "c".repeat(64) },
      ],
    }),
  );
});

test("a voice already picked keeps its candidates: the one in use is marked, another replaces it", async () => {
  const { voices, audition } = await open();
  const panel = screen.getByRole("region", { name: "Candidates for lead" });
  expect(within(panel).getByText("In use")).toBeTruthy();
  expect(within(panel).queryByRole("button", { name: "Use candidate 2" })).toBeNull();
  expect(within(panel).queryByRole("button", { name: "None of these" })).toBeNull();
  fireEvent.click(within(panel).getByRole("button", { name: "Play candidate 3" }));
  expect(heard(audition)).toEqual([candidateUrl("tank/lead", 3)]);
  fireEvent.click(within(panel).getByRole("button", { name: "Use candidate 3" }));
  await waitFor(() => expect(voices.pick).toHaveBeenCalledWith("tank/lead", 3, true));
});

test("a voice waiting for its designed voice offers its candidates through the set's profile", async () => {
  const { voices, audition } = await open();
  const panel = screen.getByRole("region", { name: "Candidates for soldier" });
  expect(within(panel).getByText(/match 0\.88 · 8\.4 s/)).toBeTruthy();
  fireEvent.click(within(panel).getByRole("button", { name: "Play candidate 2" }));
  expect(heard(audition)).toEqual([candidateUrl("tank/soldier", 2)]);
  fireEvent.click(within(panel).getByRole("button", { name: "Pick candidate 2" }));
  await waitFor(() => expect(voices.pick).toHaveBeenCalledWith("tank/soldier", 2, false));
  // None of these: a design retake, then new candidates.
  fireEvent.click(within(panel).getByRole("button", { name: "None of these" }));
  await waitFor(() => expect(voices.design).toHaveBeenCalledWith("tank/soldier"));
  expect(vi.mocked(voices.preview).mock.calls.at(-1)![0].catalog.retakes).toEqual({
    "tank/soldier/design": 1,
  });
});

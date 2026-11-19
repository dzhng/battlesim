import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";
import { CAMERA_KEYS } from "@packages/renderer-core/src/cameraController";
import { CommandBindings } from "@web/battle/input/commandBindings";
import {
  cardParts,
  coachView,
  linesBetween,
  validateCoach,
  type CoachDocument,
  type ScenarioOutline,
} from "@web/battle/coach/coach";
import type { UiSignal } from "@web/battle/input/uiSignals";
import voices from "@fixtures/voices.json";

const NARRATION = (
  voices as unknown as { narration: { lines: Record<string, { speaker: string }> } }
).narration;

/** A two-chapter coach over an invented director. */
const DOC: CoachDocument = {
  speaker: { id: "hq", name: "HQ", post: "Base", portrait: "/hq.jpg" },
  chapters: [
    {
      id: "one",
      title: "One",
      steps: [
        { id: "hello", card: "Hello", until: "continue" },
        { id: "look", card: "Look", until: { ui: ["camera_rotate", "camera_tilt"] } },
        { id: "bought", card: "Buy", until: { beat: "bought" } },
      ],
    },
    { id: "two", title: "Two", steps: [{ id: "go", card: "Go", until: { beat: "arrived" } }] },
  ],
};
const OUTLINE: ScenarioOutline = {
  chapters: [
    {
      id: "one",
      beats: [
        { id: "bought", audience: ["blue"] },
        { id: "secret", audience: ["red"] },
      ],
    },
    { id: "two", beats: [{ id: "arrived", audience: ["blue"] }] },
  ],
  zones: {},
};

const view = (
  chapter: string | null,
  beats: string[] = [],
  signals: UiSignal[] = [],
  dismissed: string[] = [],
) =>
  coachView(DOC, {
    scenario:
      chapter === null ? null : { chapter, beats: beats.map((beat) => ({ beat, tick: 1 })) },
    signals: new Set(signals),
    dismissed: new Set(dismissed),
  })?.step.id ?? null;

test("the coach shows the open chapter's first step not yet done", () => {
  expect(view("one")).toBe("hello");
  // Continue is the player's; a UI step needs every signal it names.
  expect(view("one", [], [], ["hello"])).toBe("look");
  expect(view("one", [], ["camera_rotate"], ["hello"])).toBe("look");
  expect(view("one", [], ["camera_rotate", "camera_tilt"], ["hello"])).toBe("bought");
  // A beat step is done by the publication alone.
  expect(view("one", ["bought"], ["camera_rotate", "camera_tilt"], ["hello"])).toBeNull();
});

test("a chapter the director closed takes its unfinished steps with it", () => {
  expect(view("two")).toBe("go");
  expect(view("two", ["arrived"])).toBeNull();
  // No scenario, or one past its last chapter: no coach.
  expect(view(null)).toBeNull();
  expect(view("after")).toBeNull();
});

test("a coach that disagrees with its director is refused by step", () => {
  expect(validateCoach(DOC, OUTLINE, "blue")).toEqual([]);
  const waitingOnRed: CoachDocument = {
    ...DOC,
    chapters: [
      {
        ...DOC.chapters[0],
        steps: [{ id: "x", card: "{key:nowhere}", until: { beat: "secret" } }],
      },
      DOC.chapters[1],
    ],
  };
  const problems = validateCoach(waitingOnRed, OUTLINE, "blue");
  expect(problems.map((p) => p.split(":")[0])).toEqual(["one.x", "one.x"]);
  expect(validateCoach({ ...DOC, chapters: [DOC.chapters[1]] }, OUTLINE, "blue")).toHaveLength(1);
});

test("a step's marks, rings and tour must name what the battle has", () => {
  const at = { target: [0, 0], distance: 100, yaw: 0, pitch: 1 } as const;
  const directed = (step: Partial<CoachDocument["chapters"][0]["steps"][0]>) =>
    validateCoach(
      {
        ...DOC,
        chapters: [
          { ...DOC.chapters[0], steps: [{ id: "x", card: "", until: "continue", ...step }] },
          DOC.chapters[1],
        ],
      },
      { ...OUTLINE, zones: { ford: { center: [0, 0], radius_m: 10 } } },
      "blue",
    );
  const tour = (shot: object, fade_s = 0) => ({
    fade_s,
    shots: [{ seconds: 2, from: at, to: at, ...shot }],
  });
  expect(
    directed({ marks: ["ford"], highlight: ["shop", "command:unload"], camera: tour({}) }),
  ).toEqual([]);
  expect(directed({ marks: ["nowhere"] })).toHaveLength(1);
  expect(directed({ highlight: ["command:nothing" as "shop"] })).toHaveLength(1);
  // A tour never dips to black, follows no unit, and is a valid reel.
  expect(directed({ camera: tour({}, 0.5) })).toHaveLength(1);
  expect(directed({ camera: tour({ follow: 1 }) })).toHaveLength(1);
  expect(directed({ camera: tour({ seconds: -1 }) })).toHaveLength(1);
});

test("a card names keys from their owners", () => {
  expect(cardParts("press {key:unknown}")).toEqual(["press {key:unknown}"]);
  const [words, policy] = cardParts("press {key:toggle_fire_policy}");
  expect(words).toBe("press ");
  expect(policy).toEqual({
    name: "key:toggle_fire_policy",
    keys: [CommandBindings.toggle_fire_policy.label],
  });
  // A camera axis shows each of its letter keys, as many as bind it.
  const turn = Object.keys(CAMERA_KEYS).filter(
    (code) => code.startsWith("Key") && CAMERA_KEYS[code][0] === "turn",
  );
  expect(cardParts("turn {camera:turn}")).toEqual([
    "turn ",
    { name: "camera:turn", keys: expect.arrayContaining(turn.map((c) => c.slice(3))) },
  ]);
});

/** A scenario document's outline, as the page may know it. */
const outlineOf = (scenario: {
  director?: { chapters: { id: string; beats: { id: string; audience?: string[] }[] }[] };
  zones?: ScenarioOutline["zones"];
}): ScenarioOutline => ({
  chapters: (scenario.director?.chapters ?? []).map((c) => ({
    id: c.id,
    beats: c.beats.map((b) => ({ id: b.id, audience: b.audience ?? [] })),
  })),
  zones: scenario.zones ?? {},
});

test("every shipped coach fits its scenario's director", () => {
  const maps = join(import.meta.dirname, "../../fixtures/maps");
  let checked = 0;
  for (const map of readdirSync(maps)) {
    const dir = join(maps, map, "scenarios");
    if (!existsSync(dir)) continue;
    for (const file of readdirSync(dir).filter((f) => f.endsWith(".coach.json"))) {
      const name = file.slice(0, -".coach.json".length);
      const doc = JSON.parse(readFileSync(join(dir, file), "utf8")) as CoachDocument;
      const scenario = JSON.parse(readFileSync(join(dir, `${name}.json`), "utf8"));
      const human = scenario.players.find((p: { controller: string }) => p.controller === "human");
      expect(validateCoach(doc, outlineOf(scenario), human.side), `${map}/${name}`).toEqual([]);
      // Every line a step speaks is its speaker's in the narration catalog.
      const spoken = doc.chapters.flatMap((c) => c.steps.flatMap((s) => (s.line ? [s.line] : [])));
      for (const line of spoken)
        expect(NARRATION.lines[line]?.speaker, `${map}/${name} line ${line}`).toBe(doc.speaker.id);
      checked++;
    }
  }
  expect(checked).toBeGreaterThan(0);
});

test("a step's line is spoken as it appears, or, if it reacts, as its gate is met", () => {
  const ask = { id: "ask", card: "", line: "go", until: { beat: "went" } } as const;
  const react = {
    id: "react",
    card: "",
    line: "gone",
    reacts: true,
    until: { beat: "went" },
  } as const;
  const after = { id: "after", card: "", line: "next", until: "continue" } as const;
  expect(linesBetween(null, ask, new Set())).toEqual(["go"]);
  // A reacting step says nothing as it appears...
  expect(linesBetween(ask, react, new Set())).toEqual([]);
  // ...its line comes as its gate is met, before the next step's own.
  expect(linesBetween(react, after, new Set(["went"]))).toEqual(["gone", "next"]);
  // A step a closing chapter dropped never had its gate met: no reaction.
  expect(linesBetween(react, after, new Set())).toEqual(["next"]);
});

test("once every chapter closed, the last chapter's remaining steps stay", () => {
  // The last chapter ("two") waits on "arrived"; it fired, and the director
  // closed every chapter, so a step the player has yet to dismiss stays.
  const closing = {
    ...DOC,
    chapters: [
      DOC.chapters[0],
      {
        ...DOC.chapters[1],
        steps: [...DOC.chapters[1].steps, { id: "bye", card: "Bye", until: "continue" as const }],
      },
    ],
  };
  const shown = coachView(closing, {
    scenario: { chapter: null, beats: [{ beat: "arrived", tick: 9 }] },
    signals: new Set(),
    dismissed: new Set(),
  });
  expect(shown?.step.id).toBe("bye");
});

test("a card the story has already moved past is done, but an act still waits", () => {
  // Chapter one's "bought" beat is told: its "hello" card (continue) comes
  // before it, so a run that starts there skips it; "look" asks for an act.
  const shown = coachView(DOC, {
    scenario: { chapter: "one", beats: [{ beat: "bought", tick: 3 }] },
    signals: new Set(),
    dismissed: new Set(),
  });
  expect(shown?.step.id).toBe("look");
});

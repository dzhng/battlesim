# 09 Coach core

**Unlocks:** chapters 1–2 coached in the real battle, with a plain text panel.
The look and the voice come in 10, and the camera tour, marks and highlights in
11.

**Contract (C9):**

```ts
// web/src/battle/coach/coach.ts (owner: browser presentation)
export interface CoachDocument { // fixtures/maps/<id>/scenarios/<name>.coach.json
  title: string;
  chapters: { id: string; title: string; steps: CoachStep[] }[]; // one per director chapter
}
export interface CoachStep {
  id: string;
  card: string;                 // {key:unload}, {key:show_orders} … from commandBindings
  line?: string;                // narration line id (10)
  until: { beat: string } | { ui: UiSignalKind; count?: number } | "continue";
  marks?: CoachMark[];          // 11
  camera?: Reel;                // 11
  pause?: boolean;              // hold the offline battle while this step shows
}
export function coachView(doc: CoachDocument, progress: ScenarioView, ui: UiSignalLog): CoachView;
```

- **A pure function of progress.** The current step is the first unfinished step
  of the director's open chapter. A `beat` step is done when the publication
  says so; a `ui` step when its signal arrived after the chapter opened; a
  `continue` step when the player dismisses it. When the director opens the
  next chapter, the previous chapter's unfinished steps are dropped (R12).
  Nothing accumulates beyond the UI signal log, so a restart re-derives the
  coach's state.
- **Inputs, by type:** the side's `ScenarioView`, its own observation (for its
  own units), `ScenarioPublic` (zones) and UI signals. It is never handed the
  forces.
- **The validator** (`validateCoach`, beside `validateReel`) refuses:
  - an unknown beat;
  - a waited-on beat whose audience lacks the human side;
  - an unknown zone;
  - an unknown `{key:…}` placeholder;
  - a chapter count that differs from the director's.

  Tests never pin card text.
- **UI signals** come from one typed bus, `web/src/battle/input/uiSignals.ts`.
  The input owners emit one line each:
  - `useUnitControl`: selected via click, card or box; control group set or
    recalled; show-orders held;
  - the camera controller: pan, rotate, tilt, zoom;
  - contact focus;
  - the purchase picker: opened, placed with facing.

  The sim never sees these signals.
- **Pausing** while a `pause` step shows goes through the session's
  `SimClient.pause()`, the way the pause menu does it. Ticks don't pass, so
  determinism is untouched. The coach and the pause menu share one ownership
  rule for who may resume.
- **`CoachPanel`** is mounted by `BattleView` when the session's report carries a
  scenario. In this slice it is a plain readable card (title, card text,
  Continue). Load game-ui before drawing it: it is a transient instruction
  card, not a dashboard panel.

**Content:** `operation-ferryman.coach.json` for chapters 1–2 (every step of
[story.md](../story.md), with lines named but not yet played).

**Playable:** the tutorial battle coaches chapters 1–2 step by step from the
address.

**Gates:**

- `web/tests/coach.test.ts`, the one unit test, justified because step
  derivation is complex business logic:
  - step order;
  - a beat step completes from the publication alone;
  - a UI step completes from its signal;
  - steps re-derive after a chapter change, and dropped steps don't return;
  - validator refusals by code, never by message.
- Scene `web/scenes/tutorial.mjs` drives the real page with the cursor and keys
  through chapter 1. It asserts a step change for each gate kind and shoots each
  step.
- Screenshot-critique last on the panel shots. Show them inline (non-blocking).

**Delegated:** the module split inside `coach/`, signal names, the panel's
position within the game-ui rules (10 restyles it).

**Must stay green:** `useUnitControl` and camera tests (the signal emits are one
line each and change no behaviour); pause menu tests.

/** The tutorial's coach: the steps an authored scenario teaches by, and
 *  which one is showing. It is a pure function of the scenario's published
 *  progress (the open chapter and the beats the player's side was told), the
 *  UI signals since the chapter opened and the steps the player dismissed.
 *  It never sees a force: what it knows of the battle, the player's side was
 *  told. Its text is data and never enters the simulation. */
import { CAMERA_KEYS } from "@packages/renderer-core/src/cameraController";
import {
  CommandBindings,
  ContactFocusBinding,
  ShowOrdersBinding,
  type KeyCommand,
} from "../input/commandBindings";
import { UI_SIGNALS, type UiSignal } from "../input/uiSignals";
import type { ScenarioView } from "../sim/observationView";

/** A scenario's coach: `fixtures/maps/<map>/scenarios/<name>.coach.json`. */
export interface CoachDocument {
  /** What the loading screen and the pause menu call the scenario. */
  title: string;
  /** Who speaks the steps' lines: their narration speaker, the name and
   *  post the card shows, and their portrait. */
  speaker: { id: string; name: string; post: string; portrait: string };
  /** One per director chapter, in the same order and with the same ids. */
  chapters: CoachChapter[];
}

export interface CoachChapter {
  id: string;
  title: string;
  steps: CoachStep[];
}

export interface CoachStep {
  id: string;
  /** What the card says; `{key:<command>}` and `{camera:<axis>}` name the
   *  keys from their owners. */
  card: string;
  /** The narration line spoken with it, by id. */
  line?: string;
  /** What finishes it: a beat the player's side is told, every one of these
   *  UI signals since its chapter opened, or the player's own Continue. */
  until: { beat: string } | { ui: UiSignal[] } | "continue";
}

/** What the page may know of a scenario before and during its battle: its
 *  chapters and beats, each beat with the sides told it, and its zones. */
export interface ScenarioOutline {
  chapters: { id: string; beats: { id: string; audience: string[] }[] }[];
  zones: Record<string, { center: [number, number]; radius_m: number }>;
}

/** Every way `doc` disagrees with the scenario it coaches, for a player on
 *  `side`; empty when it fits. */
export function validateCoach(
  doc: CoachDocument,
  outline: ScenarioOutline,
  side: string,
): string[] {
  const problems: string[] = [];
  const ids = (list: { id: string }[]) => list.map((c) => c.id).join(",");
  if (ids(doc.chapters) !== ids(outline.chapters))
    problems.push(`chapters: ${ids(doc.chapters)} are not the director's ${ids(outline.chapters)}`);
  const told = new Set(
    outline.chapters.flatMap((c) =>
      c.beats.filter((b) => b.audience.includes(side)).map((b) => b.id),
    ),
  );
  const steps = new Set<string>();
  for (const chapter of doc.chapters)
    for (const step of chapter.steps) {
      const at = `${chapter.id}.${step.id}`;
      if (steps.has(step.id)) problems.push(`${at}: the step's id repeats`);
      steps.add(step.id);
      if (typeof step.until === "object" && "beat" in step.until && !told.has(step.until.beat))
        problems.push(`${at}: waits on ${step.until.beat}, which ${side} is never told`);
      if (typeof step.until === "object" && "ui" in step.until)
        for (const signal of step.until.ui)
          if (!UI_SIGNALS.includes(signal)) problems.push(`${at}: no UI signal ${signal}`);
      for (const [, kind, name] of step.card.matchAll(/\{(key|camera):([a-z_]+)\}/g))
        if (keyText(kind as "key" | "camera", name) === null)
          problems.push(`${at}: no ${kind} ${name}`);
    }
  return problems;
}

/** Letter keys as a keyboard reads them, so a pair shows as W/S and A/D. */
const KEYBOARD = "QWERTYUIOPASDFGHJKLZXCVBNM";

/** The keys a placeholder names, as the card shows them; null for none. */
function keyText(kind: "key" | "camera", name: string): string[] | null {
  if (kind === "key") {
    const held: Record<string, string> = {
      show_orders: ShowOrdersBinding.label,
      contact_focus: ContactFocusBinding.label,
    };
    const label = held[name] ?? CommandBindings[name as KeyCommand]?.label;
    return label ? [label] : null;
  }
  const keys = Object.entries(CAMERA_KEYS)
    .filter(([code, [axis]]) => axis === name && code.startsWith("Key"))
    .map(([code]) => code.slice(3))
    .sort((a, b) => KEYBOARD.indexOf(a) - KEYBOARD.indexOf(b));
  return keys.length ? keys : null;
}

/** A card's words, and the keys it names in their place. */
export type CardPart = string | { keys: string[] };

/** A card's text with its keys filled in, as parts the panel draws. */
export function cardParts(card: string): CardPart[] {
  const parts: CardPart[] = [];
  let at = 0;
  for (const found of card.matchAll(/\{(key|camera):([a-z_]+)\}/g)) {
    const keys = keyText(found[1] as "key" | "camera", found[2]);
    if (!keys) continue;
    parts.push(card.slice(at, found.index), { keys });
    at = found.index + found[0].length;
  }
  parts.push(card.slice(at));
  return parts.filter((part) => part !== "");
}

/** What the coach has heard since the open chapter opened, and what the
 *  player dismissed. */
export interface CoachProgress {
  scenario: ScenarioView | null;
  signals: ReadonlySet<UiSignal>;
  dismissed: ReadonlySet<string>;
}

export interface CoachView {
  chapter: CoachChapter;
  step: CoachStep;
  /** The step's place in its chapter, from 0. */
  index: number;
}

/** The step showing now: the first of the open chapter's steps not yet done,
 *  or none once the chapter's steps are all done or the scenario has ended.
 *  A chapter the director has closed is gone, done or not. */
export function coachView(doc: CoachDocument, progress: CoachProgress): CoachView | null {
  const chapter = doc.chapters.find((c) => c.id === progress.scenario?.chapter);
  if (!chapter) return null;
  const told = new Set(progress.scenario?.beats.map((b) => b.beat));
  const done = (step: CoachStep) =>
    progress.dismissed.has(step.id) ||
    (step.until !== "continue" &&
      ("beat" in step.until
        ? told.has(step.until.beat)
        : step.until.ui.every((signal) => progress.signals.has(signal))));
  const index = chapter.steps.findIndex((step) => !done(step));
  return index < 0 ? null : { chapter, step: chapter.steps[index], index };
}

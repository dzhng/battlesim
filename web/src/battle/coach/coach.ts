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
import { FirstGroupKeys } from "../input/controlGroups";
import { isHudAnchor, type HudAnchor } from "../present/hudAnchors";
import { validateReel, type Reel } from "@apps/battle-lab/src/capture/capture";
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
  /** The narration line spoken with it, by id: as the step appears, or,
   *  for a step that `reacts`, as its gate is met. */
  line?: string;
  /** Its line answers its gate ("They're pinned") rather than asking for
   *  it, so it's spoken when the gate is met, before the next step's. */
  reacts?: boolean;
  /** What finishes it: a beat the player's side is told, every one of these
   *  UI signals since its chapter opened, or the player's own Continue. */
  until: { beat: string } | { ui: UiSignal[] } | "continue";
  /** A camera tour flown when the step appears, without dips; the player's
   *  camera takes over where it ends. */
  camera?: Reel;
  /** Zones painted on the ground while the step shows, by scenario zone id. */
  marks?: string[];
  /** HUD controls ringed while the step shows. */
  highlight?: HudAnchor[];
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
      for (const zone of step.marks ?? [])
        if (!(zone in outline.zones)) problems.push(`${at}: no zone ${zone}`);
      for (const anchor of step.highlight ?? [])
        if (!isHudAnchor(anchor)) problems.push(`${at}: no HUD anchor ${anchor}`);
      if (step.camera) problems.push(...tourProblems(step.camera, at));
    }
  return problems;
}

/** A tour is a reel over the world: it never dips to black, and it follows
 *  no unit, since the coach knows no unit's id. */
function tourProblems(camera: Reel, at: string): string[] {
  try {
    const reel = validateReel(camera, `${at}.camera`);
    if (reel.fade_s !== 0) return [`${at}: a tour never dips (fade_s 0)`];
    if (reel.shots.some((shot) => shot.follow !== undefined))
      return [`${at}: a tour follows no unit`];
    return [];
  } catch (error) {
    return [`${at}: ${(error as Error).message}`];
  }
}

/** Letter keys as a keyboard reads them, so a pair shows as W/S and A/D. */
const KEYBOARD = "QWERTYUIOPASDFGHJKLZXCVBNM";

/** The keys a placeholder names, as the card shows them; null for none. */
function keyText(kind: "key" | "camera", name: string): string[] | null {
  if (kind === "key") {
    const held: Record<string, string> = {
      show_orders: ShowOrdersBinding.label,
      contact_focus: ContactFocusBinding.label,
      group_assign: FirstGroupKeys.assign,
      group_recall: FirstGroupKeys.recall,
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
 *  or none once the chapter's steps are all done. A chapter the director has
 *  closed is gone, done or not, except the last: once every chapter closed,
 *  its steps stay, so the story's closing words outlast the battle. */
export function coachView(doc: CoachDocument, progress: CoachProgress): CoachView | null {
  const open = progress.scenario?.chapter;
  const chapter = open === null ? doc.chapters.at(-1) : doc.chapters.find((c) => c.id === open);
  if (!chapter) return null;
  const told = new Set(progress.scenario?.beats.map((b) => b.beat));
  const beaten = (step: CoachStep) =>
    typeof step.until === "object" && "beat" in step.until && told.has(step.until.beat);
  // A Continue step before one whose beat the story already told is behind
  // it, as a restarted or resumed run finds a card it never showed; a step
  // that asks for an act still waits for the act, since beats fire in any
  // order.
  const behind = chapter.steps.reduce((last, step, at) => (beaten(step) ? at : last), -1);
  const done = (step: CoachStep, at: number) =>
    (at < behind && step.until === "continue") ||
    progress.dismissed.has(step.id) ||
    beaten(step) ||
    (typeof step.until === "object" &&
      "ui" in step.until &&
      step.until.ui.every((signal) => progress.signals.has(signal)));
  const index = chapter.steps.findIndex((step, at) => !done(step, at));
  return index < 0 ? null : { chapter, step: chapter.steps[index], index };
}

/** The lines to speak, in order, as the coach moves from step `left` to
 *  step `shown`: the left step's reaction, if its gate was met (a step a
 *  closing chapter dropped says nothing), then the shown step's own line,
 *  unless it waits to react. */
export function linesBetween(
  left: CoachStep | null,
  shown: CoachStep | null,
  told: ReadonlySet<string>,
): string[] {
  const met =
    left?.reacts &&
    left.line &&
    (typeof left.until !== "object" || !("beat" in left.until) || told.has(left.until.beat));
  return [
    ...(met && left?.line ? [left.line] : []),
    ...(shown?.line && !shown.reacts ? [shown.line] : []),
  ];
}

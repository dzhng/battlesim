// The Armory stage's readout for `web/scenes/armory.mjs`, on
// `window.__armory`: what stands on the stage, where it falls on screen and
// the files its art was read from, each unit switch's time, and how many
// show loops sound. It outlives the
// stage, so a scene can read the loops after leaving.
import type { RefObject } from "react";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import type { ShowcaseStage, StageArt } from "@web/battle/render/showcaseStage";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { ArmoryAppearances } from "./armoryAppearances";
import { drawnModels, grounded, screenBox, worldBox } from "./stageProbes";
import type { Spot } from "./yard";

/** What the stage last showed: the kind (null: none), its art, and the
 *  spot whose camera sees it. */
export interface Shown {
  kind: string | null;
  art: StageArt;
  spot: Spot;
}

/** One unit switch: the kind asked for, and the milliseconds from the pick
 *  to its standing on the stage. */
export interface Switch {
  kind: string;
  ms: number;
}

/** Publishes the readout for a live stage; the returned call leaves only
 *  what outlives it (switches and loops). */
export function publishProbe(live: {
  stage: ShowcaseStage;
  canvas: HTMLCanvasElement;
  /** The orbit the stage draws a spot from. */
  orbit: (spot: Spot) => Camera3DParams;
  audio: AppAudio;
  switches: readonly Switch[];
  shown: RefObject<Shown | null>;
  appearances: ArmoryAppearances;
}): () => void {
  const { stage, canvas, orbit, audio, switches, shown, appearances } = live;
  const lasting = { switches, voices: () => audio.stats().showVoices };
  /** The drawn models standing in the spot's clear disc: the unit. */
  const unitModels = (on: Shown) =>
    drawnModels(stage).filter(
      (m) => Math.hypot(m.at[0] - on.spot.at[0], m.at[1] - on.spot.at[1]) < on.spot.clear_m,
    );
  const probe = {
    ...lasting,
    /** The stage's cell, where the unit stands framed. */
    stage: () => {
      const r = canvas.parentElement!.getBoundingClientRect();
      return { x: r.left, y: r.top, width: r.width, height: r.height };
    },
    shown: () => {
      const on = shown.current;
      if (!on) return null;
      const models = unitModels(on);
      const box = worldBox(models, on.art.appearances);
      const r = canvas.getBoundingClientRect();
      const rect = box && screenBox(orbit(on.spot), r.width, r.height, box);
      return {
        kind: on.kind,
        spot: on.spot.id,
        held: appearances.held(),
        standing: stage.standing(),
        gaps: grounded(models, on.art.appearances),
        /** The unit's bounds on the page, in CSS pixels. */
        rect: rect && { ...rect, x: rect.x + r.left, y: rect.y + r.top },
      };
    },
  };
  const target = window as unknown as { __armory: unknown };
  target.__armory = probe;
  return () => {
    target.__armory = { ...lasting, stage: () => null, shown: () => null };
  };
}

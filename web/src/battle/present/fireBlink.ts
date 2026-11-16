/** When an army card blinks: yellow when an enemy round aimed at its unit
 *  passed near it (the publication's `shotAt`), red when its unit was hurt
 *  (its health fell since the last publication). A blink finishes once
 *  started, except that red cuts into yellow; a card never shows the same
 *  colour again within `presentation.hud.fire_blink.wait_s`. A unit lost
 *  (gone from the side without withdrawing, the one other way out) always
 *  blinks red, and its card stays for that blink. Read once per publication,
 *  so a pass published for one tick is never missed, and timed in ticks, so
 *  the rules follow the battle and not the frame rate. A change of the side
 *  watched starts over, as a new battle does. */
import { Feed } from "@web/feed";
import type { ObservationView, OwnUnitView } from "../sim/observationView";
import { gameHud, type HudTheme } from "./hudTheme";

export type BlinkColour = "yellow" | "red";
/** A card's latest blink; a new `serial` starts its animation again. */
export interface CardBlink {
  colour: BlinkColour;
  serial: number;
  /** Its unit was lost: the card stays until this blink has run. */
  lost: boolean;
}

export type BlinkTiming = Pick<HudTheme["fire_blink"], "on_s" | "off_s" | "wait_s">;

/** Plays a blink on its element as it mounts (a ref callback, so a stable
 *  function: it runs once per blink, never per render). Two pulses, each
 *  lit for `on_s` and dark for `off_s`: it rises as fast as it fades (half of
 *  `off_s`, kept within `on_s` so the keyframes stay in time order), so an
 *  alert lands at once, holds while easing off a little, and is dark for the
 *  rest. */
export function playBlink(el: HTMLElement | null, timing: BlinkTiming = gameHud.fire_blink) {
  if (!el?.animate || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const { on_s: on, off_s: off } = timing;
  const total = 2 * (on + off);
  const frame = (s: number, opacity: number) =>
    ({ offset: s / total, opacity, easing: "ease-in-out" }) as const;
  el.animate(
    [0, on + off]
      .flatMap((p) => [
        frame(p, 0),
        frame(p + Math.min(off, on) / 2, 1),
        frame(p + on, 0.85),
        frame(p + on + off / 2, 0),
      ])
      .concat(frame(total, 0)),
    { duration: total * 1000 },
  );
}

/** A unit's health as one number: a vehicle's, plus every living soldier's. */
const health = (u: OwnUnitView) => u.hp + u.memberHp.reduce((sum, hp) => sum + hp, 0);

export class FireBlinks {
  /** Each unit's running blink, by unit id; gone once it has run, so a card
   *  that moves or remounts never shows an old one again. */
  readonly feed = new Feed<ReadonlyMap<number, CardBlink>>(new Map());
  private readonly blinkTicks: number;
  private readonly waitTicks: number;
  /** Each unit's health and whether it was withdrawing, last publication. */
  private last = new Map<number, { health: number; withdrawing: boolean }>();
  private side: string | null = null;
  /** Each unit's current blink and the tick it started. */
  private started = new Map<number, { colour: BlinkColour; tick: number }>();
  /** The tick each unit last showed each colour. */
  private shown = new Map<string, number>();
  private tick = -1;
  private serial = 0;

  constructor(tickHz: number, timing: BlinkTiming = gameHud.fire_blink) {
    this.blinkTicks = Math.ceil(2 * (timing.on_s + timing.off_s) * tickHz);
    this.waitTicks = Math.round(timing.wait_s * tickHz);
  }

  observe(o: ObservationView) {
    if (o.tick < this.tick || o.groundPatch.side !== this.side) this.reset(o.groundPatch.side);
    this.tick = o.tick;
    const present = new Set(o.own.map((u) => u.id));
    let next: Map<number, CardBlink> | null = null;
    for (const [id, blink] of this.feed.current) {
      const running = this.started.get(id);
      const shown = present.has(id) || blink.lost;
      if (shown && running && o.tick - running.tick < this.blinkTicks) continue;
      next ??= new Map(this.feed.current);
      next.delete(id);
    }
    for (const [id, before] of this.last) {
      if (present.has(id) || before.withdrawing) continue;
      this.started.set(id, { colour: "red", tick: o.tick });
      next ??= new Map(this.feed.current);
      next.set(id, { colour: "red", serial: ++this.serial, lost: true });
    }
    const seen = new Map<number, { health: number; withdrawing: boolean }>();
    for (const unit of o.own) {
      const now = health(unit);
      seen.set(unit.id, { health: now, withdrawing: unit.withdrawing });
      const before = this.last.get(unit.id)?.health;
      const hurt = before !== undefined && now < before;
      // Red first; a hit inside red's wait still lets a pass show yellow.
      const colour = (["red", "yellow"] as const).find(
        (c) => (c === "red" ? hurt : unit.shotAt) && this.starts(unit.id, c, o.tick),
      );
      if (colour) {
        next ??= new Map(this.feed.current);
        next.set(unit.id, { colour, serial: ++this.serial, lost: false });
      }
    }
    this.last = seen;
    if (next) this.feed.set(next);
  }

  /** Whether a blink of `colour` starts on `unit` now, by the wait and the
   *  cut-in; records it if it does. */
  private starts(unit: number, colour: BlinkColour, tick: number): boolean {
    const last = this.shown.get(`${unit}:${colour}`);
    if (last !== undefined && tick - last < this.waitTicks) return false;
    const running = this.started.get(unit);
    const busy = running && tick - running.tick < this.blinkTicks;
    if (busy && !(colour === "red" && running.colour === "yellow")) return false;
    this.shown.set(`${unit}:${colour}`, tick);
    this.started.set(unit, { colour, tick });
    return true;
  }

  private reset(side: string) {
    this.side = side;
    this.last = new Map();
    this.started = new Map();
    this.shown = new Map();
    this.feed.set(new Map());
  }
}

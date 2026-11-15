/** When an army card blinks: yellow when an enemy round aimed at its unit
 *  passed near it (the publication's `shotAt`), red when its unit was hurt
 *  (its health fell since the last publication). A blink finishes once
 *  started, except that red cuts into yellow; a card never shows the same
 *  colour again within `presentation.hud.fire_blink.wait_s`. Read once per
 *  publication, so a pass published for one tick is never missed, and timed
 *  in ticks, so the rules follow the battle and not the frame rate. */
import { Feed } from "@web/feed";
import type { ObservationView, OwnUnitView } from "../sim/observationView";
import { gameHud } from "./hudTheme";

export type BlinkColour = "yellow" | "red";
/** A card's latest blink; a new `serial` starts its animation again. */
export interface CardBlink {
  colour: BlinkColour;
  serial: number;
}

export interface BlinkTiming {
  on_s: number;
  off_s: number;
  wait_s: number;
}

/** A unit's health as one number: a vehicle's, plus every living soldier's. */
const health = (u: OwnUnitView) => u.hp + u.memberHp.reduce((sum, hp) => sum + hp, 0);

export class FireBlinks {
  /** Each unit's latest blink, by unit id. */
  readonly feed = new Feed<ReadonlyMap<number, CardBlink>>(new Map());
  private readonly blinkTicks: number;
  private readonly waitTicks: number;
  private health = new Map<number, number>();
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
    if (o.tick < this.tick) this.reset();
    this.tick = o.tick;
    let next: Map<number, CardBlink> | null = null;
    const measured = new Map<number, number>();
    for (const unit of o.own) {
      const now = health(unit);
      measured.set(unit.id, now);
      const before = this.health.get(unit.id);
      const hurt = before !== undefined && now < before;
      // Red first; a hit inside red's wait still lets a pass show yellow.
      const colour = (["red", "yellow"] as const).find(
        (c) => (c === "red" ? hurt : unit.shotAt) && this.starts(unit.id, c, o.tick),
      );
      if (colour) {
        next ??= new Map(this.feed.current);
        next.set(unit.id, { colour, serial: ++this.serial });
      }
    }
    this.health = measured;
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

  private reset() {
    this.health = new Map();
    this.started = new Map();
    this.shown = new Map();
    this.feed.set(new Map());
  }
}

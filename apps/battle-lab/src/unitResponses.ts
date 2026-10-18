// The battle's side of its units' answers (R1–R12): what the player did
// (selections, acknowledged orders) and what the side observes (arrivals),
// turned into the lines `responseFor` asks the battle's sound for, and the
// voice sets it loads: those the side's units, and the purchases on their
// way, speak with for the player's faction (`voices.json` assignments).
import { assignments as voiceAssignments } from "@fixtures/voices.json";
import {
  answerer,
  Arrivals,
  responseFor,
  type ResponseEvent,
  type ResponseRequest,
  type ResponseUnit,
} from "@packages/battle-audio/src/voices/responses";
import { assignedSet, type VoiceCatalog } from "@packages/battle-audio/src/voices/voiceCatalog";
import { FACTIONS, type Faction, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { ObservationView, OwnUnitView, RefusalView } from "@web/battle/sim/observationView";
import type { CommandAck, Order } from "@web/battle/sim/protocol";

/** The battle's sound, as far as answers go. */
export interface Responder {
  respond(request: ResponseRequest): void;
  loadVoices(sets: Iterable<string>): void;
}

export class UnitResponses {
  private own: readonly OwnUnitView[] = [];
  /** The unit that answered the selection: it answers the group's orders too. */
  private lead: number | null = null;
  private readonly arrivals = new Arrivals();
  private readonly sets = new Map<string, string | null>();
  /** Refusals already answered from an acknowledgement, until the tick its
   *  order applies publishes them (`ObservationFrame.refusals`). */
  private answered: { tick: number; refused: RefusalView[] }[] = [];

  constructor(
    private readonly audio: Responder,
    private readonly units: UnitCatalog,
    /** The player's faction in a skirmish; elsewhere each unit row's own. */
    private readonly faction: Faction | null,
    private readonly tickHz: number,
    private readonly assignments: VoiceCatalog["assignments"] = voiceAssignments,
  ) {}

  /** A publication: the sets its units speak with load, and arrivals report. */
  observed(observation: ObservationView) {
    this.own = observation.own;
    const kinds = new Set(observation.own.map((unit) => unit.kind));
    for (const { kind } of observation.skirmish?.players.flatMap((player) => player.pending) ?? [])
      kinds.add(kind);
    this.audio.loadVoices([...kinds].flatMap((kind) => this.setOf(kind) ?? []));
    const moving = observation.own.map((unit) => ({
      id: unit.id,
      moving: unit.goal !== null && !unit.withdrawing,
    }));
    for (const { id, group } of this.arrivals.observe(moving, observation.tick / this.tickHz)) {
      const unit = this.answering([id])[0];
      if (unit) this.say({ kind: "arrived", unit, group });
    }
    this.observedRefusals(observation);
  }

  /** The selection gained `added`, by clicking `clicked` if one was. */
  selected(added: readonly number[], clicked: number | null) {
    const units = this.answering(added);
    this.lead = answerer(units, clicked)?.id ?? this.lead;
    this.say({ kind: "selected", units, lead: this.lead });
  }

  /** The authority's answer to `order`: refused whole, or accepted for the
   *  units it did not refuse one by one. */
  acknowledged(order: Order, ack: CommandAck, queued: boolean) {
    const ids = "units" in order ? order.units : [];
    const id = ack.seq;
    if (ack.error) {
      const units = this.answering(ids);
      this.say({
        kind: "refused",
        reason: ack.error.reason,
        units,
        lead: this.lead,
        group: `order/${id}`,
      });
      return;
    }
    const refused = ack.refused.map(({ unit, reason }) => ({ unit, reason: reason.reason }));
    this.refusedAt(refused, `order/${id}`);
    // The side observes these refusals too, the tick the order applies.
    if (refused.length) this.answered.push({ tick: ack.applied_tick, refused });
    const left = new Set(refused.map(({ unit }) => unit));
    const accepted = ids.filter((unit) => !left.has(unit));
    if (!accepted.length) return;
    this.arrivals.ordered({ ...order, units: accepted }, queued, `order/${id}`);
    this.say({ kind: "accepted", order, units: this.answering(accepted), lead: this.lead, id });
  }

  /** `refused` units answer with their refusal: one answer per reason. */
  private refusedAt(refused: readonly RefusalView[], group: string) {
    for (const reason of new Set(refused.map((refusal) => refusal.reason))) {
      const units = this.answering(refused.filter((r) => r.reason === reason).map((r) => r.unit));
      this.say({ kind: "refused", reason, units, lead: this.lead, group });
    }
  }

  /** The side's refusals this tick that no acknowledgement has answered:
   *  those that happen later, such as no room to unload on arrival. */
  private observedRefusals({ tick, refusals }: ObservationView) {
    const later = refusals.filter(
      (refusal) =>
        !this.answered.some(
          (order) =>
            order.tick === tick &&
            order.refused.some((r) => r.unit === refusal.unit && r.reason === refusal.reason),
        ),
    );
    this.answered = this.answered.filter((order) => order.tick > tick);
    this.refusedAt(later, `tick/${tick}`);
  }

  private say(event: ResponseEvent) {
    const request = responseFor(event);
    if (request) this.audio.respond(request);
  }

  /** `ids` among the side's units that have a voice. */
  private answering(ids: readonly number[]): ResponseUnit[] {
    return ids.flatMap((id) => {
      const view = this.own.find((unit) => unit.id === id);
      const set = view && this.setOf(view.kind);
      if (!view || !set) return [];
      return [{ id, set, value: this.units.type(view.kind).cost, stressed: this.stressed(view) }];
    });
  }

  private setOf(kind: string): string | null {
    let set = this.sets.get(kind);
    if (set === undefined) {
      const type = this.units.has(kind) ? this.units.type(kind) : null;
      const own = FACTIONS.find((faction) => faction === type?.faction);
      const faction = this.faction ?? own ?? type?.roster?.factions[0] ?? "";
      set = assignedSet(this.assignments, kind, faction);
      this.sets.set(kind, set);
    }
    return set;
  }

  /** Health at half or less (a squad's living soldiers of its slots), or suppressed. */
  private stressed(unit: OwnUnitView): boolean {
    if (unit.suppression !== "none") return true;
    const hull = this.units.hull(unit.kind);
    if (hull) return unit.hp * 2 <= hull.hp;
    return unit.memberIds.length * 2 <= this.units.slots(unit.kind).length;
  }
}

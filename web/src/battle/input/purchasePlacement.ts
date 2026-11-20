import type { CommandAck, Order, PurchasePlacement } from "../sim/protocol";
import type { SeatClient } from "../sim/client";
/** A placed ghost faces the opposing edge unless it is turned as it is placed. */
export const DEFAULT_PLACEMENT_FACING = -Math.PI / 2;
export interface PurchaseGhost {
  variant: string;
  /** The carrier `variant` is bought aboard; the ghost is drawn as it, since
   *  it is what arrives. Absent for a unit bought on its own. */
  carrier?: string;
  /** Where the unit is sent: the cursor, snapped by the authority to the
   *  nearest ground the unit stands on once a preview answers. */
  destination: [number, number];
  /** World bearing of the ghost; deployment defaults toward the opposing edge. */
  facing: number;
  valid: boolean | null;
  /** Where a squad's soldiers stand; empty for a vehicle, or until a preview answers. */
  spots: [number, number][];
  /** The radius of the area a squad will hold round `destination`; null for
   *  a vehicle, or until a preview answers. */
  areaRadius: number | null;
}
/** Free previews are coalesced and only draw the ghost; a click sends the
 *  purchase without waiting for one, and only its acknowledgement commits it. */
export class PurchasePlacementControl {
  variant: string | null = null;
  /** The carrier the chosen squad rides in, or null on foot. */
  carrier: string | null = null;
  private intent: {
    client: SeatClient;
    /** The authority, variant and spot asked about. */
    spot: string;
    /** The side's knowledge the question was asked of. */
    revision: string;
    cursor: [number, number];
    ghost: PurchaseGhost;
  } | null = null;
  /** The latest answer for a spot. It stands while the side's knowledge moves
   *  on (in a team battle it does every tick or two, faster than an answer
   *  comes back): a fresh preview is asked meanwhile, and the authority checks
   *  the purchase again when it is confirmed. A preview that did not arrive
   *  (dropped, timed out) is no answer about the ground: its placement is
   *  null, and it is not asked again until the pointer or the battle moves. */
  private resolved: {
    spot: string;
    revision: string;
    placement: PurchasePlacement | null;
  } | null = null;
  /** The chosen squad's soldiers about its destination, as the last preview
   *  spread them: the ghost keeps this shape while the next one is pending. */
  private shape: [number, number][] = [];
  /** The chosen squad's area radius, as the last preview named it. */
  private areaRadius: number | null = null;
  private previewRun: string | null = null;
  private generation = 0;
  /** Confirmations sent and not yet answered. */
  private confirming = 0;
  private client: SeatClient | null = null;

  /** The unit that enters the map: the carrier, else the variant. */
  get arriving(): string | null {
    return this.carrier ?? this.variant;
  }
  choose(variant: string, carrier: string | null = null) {
    this.cancel();
    this.variant = variant;
    this.carrier = carrier;
  }
  cancel() {
    this.generation++;
    this.variant = null;
    this.carrier = null;
    this.intent = null;
    this.resolved = null;
    this.shape = [];
    this.areaRadius = null;
  }
  at(
    destination: [number, number] | null,
    client: SeatClient | null,
    revision: string,
    facing = DEFAULT_PLACEMENT_FACING,
  ): PurchaseGhost | null {
    if (!this.variant || !destination || !client) {
      this.intent = null;
      return null;
    }
    if (this.client !== client) {
      this.client = client;
      this.generation++;
      this.resolved = null;
    }
    // Admission depends on where the ghost stands, never on how it is turned.
    const spot = JSON.stringify([this.generation, this.variant, this.carrier, destination]);
    const placement = this.resolved?.spot === spot ? this.resolved.placement : null;
    const at = placement && "Ok" in placement ? placement.Ok.destination : destination;
    const ghost: PurchaseGhost = {
      variant: this.variant,
      ...(this.carrier && { carrier: this.carrier }),
      destination: at,
      facing,
      valid: placement ? "Ok" in placement : null,
      spots: this.shape.map(([dx, dy]): [number, number] => [at[0] + dx, at[1] + dy]),
      areaRadius: this.areaRadius,
    };
    this.intent = { client, spot, revision, cursor: destination, ghost };
    this.resolve();
    return ghost;
  }
  private resolve() {
    const intent = this.intent;
    if (
      !intent ||
      this.previewRun ||
      (this.resolved?.spot === intent.spot && this.resolved.revision === intent.revision)
    )
      return;
    const run = (this.previewRun = intent.spot);
    void intent.client
      .previewPurchase(intent.ghost.variant, intent.cursor, intent.ghost.carrier)
      .then(
        (placement) => {
          if (this.intent?.spot !== run) return;
          this.resolved = { spot: intent.spot, revision: intent.revision, placement };
          if ("Ok" in placement) {
            const [x, y] = placement.Ok.destination;
            this.shape = placement.Ok.spots.map(([sx, sy]) => [sx - x, sy - y]);
            this.areaRadius = placement.Ok.area_radius_m;
          }
        },
        () => {
          if (this.intent?.spot === run)
            this.resolved = { spot: intent.spot, revision: intent.revision, placement: null };
        },
      )
      .finally(() => {
        if (this.previewRun === run) this.previewRun = null;
        this.resolve();
      });
  }
  /** The ghost the authority accepted, or null when nothing was placed.
   *  `another` keeps the choice after an accepted placement, for the next.
   *
   *  A click is for the spot under the pointer when it lands, and it never
   *  waits for a preview: the order goes at once, and the authority, which
   *  places every purchase itself, admits or refuses it. On a slow link a
   *  preview is a round trip behind the pointer, so a click that waited for
   *  one would be late or lost. Only a spot whose own preview has already
   *  refused it sends nothing. */
  async confirm(
    send: (order: Order) => Promise<CommandAck | null | undefined>,
    another = false,
  ): Promise<PurchaseGhost | null> {
    const intent = this.intent;
    // A plain click while one is still being bought is the same click twice.
    if (!intent || (this.confirming > 0 && !another)) return null;
    const generation = this.generation;
    const answered = this.resolved?.spot === intent.spot ? this.resolved.placement : null;
    if (answered && !("Ok" in answered)) return null;
    // Answered, the ghost already stands where the authority sends it;
    // unanswered, it stands under the pointer and the authority snaps it.
    const ghost: PurchaseGhost = { ...intent.ghost, valid: true };
    this.confirming++;
    try {
      const ack = await send({
        kind: "confirm_purchase",
        variant: ghost.variant,
        ...(ghost.carrier && { carrier: ghost.carrier }),
        destination: ghost.destination,
      });
      if (!ack || ack.error) return null;
      if (generation === this.generation && !another) this.cancel();
      return ghost;
    } finally {
      this.confirming--;
    }
  }
}

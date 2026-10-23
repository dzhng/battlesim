import type { CommandAck, Order, PurchasePlacement } from "../sim/protocol";
import type { SimClient } from "../sim/client";
/** A placed ghost faces the opposing edge unless it is turned as it is placed. */
export const DEFAULT_PLACEMENT_FACING = -Math.PI / 2;
export interface PurchaseGhost {
  variant: string;
  /** Where the unit is sent: the cursor, snapped by the authority to the
   *  nearest ground the unit stands on once a preview answers. */
  destination: [number, number];
  /** World bearing of the ghost; deployment defaults toward the opposing edge. */
  facing: number;
  valid: boolean | null;
  /** Where a squad's soldiers stand; empty for a vehicle, or until a preview answers. */
  spots: [number, number][];
}
/** Free previews are coalesced; only an acknowledgement commits a purchase. */
export class PurchasePlacementControl {
  variant: string | null = null;
  private ghost: PurchaseGhost | null = null;
  private intent: {
    client: SimClient;
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
   *  the purchase again when it is confirmed. */
  private resolved: { spot: string; revision: string; placement: PurchasePlacement } | null = null;
  /** The chosen squad's soldiers about its destination, as the last preview
   *  spread them: the ghost keeps this shape while the next one is pending. */
  private shape: [number, number][] = [];
  private inFlight = false;
  private generation = 0;
  private confirming = false;
  private client: SimClient | null = null;

  choose(variant: string) {
    this.cancel();
    this.variant = variant;
  }
  cancel() {
    this.generation++;
    this.variant = null;
    this.ghost = null;
    this.intent = null;
    this.resolved = null;
    this.shape = [];
  }
  at(
    destination: [number, number] | null,
    client: SimClient | null,
    revision: string,
    facing = DEFAULT_PLACEMENT_FACING,
  ): PurchaseGhost | null {
    if (!this.variant || !destination || !client) {
      this.ghost = null;
      this.intent = null;
      return null;
    }
    if (this.client !== client) {
      this.client = client;
      this.generation++;
      this.resolved = null;
    }
    // Admission depends on where the ghost stands, never on how it is turned.
    const spot = JSON.stringify([this.generation, this.variant, destination]);
    const placement = this.resolved?.spot === spot ? this.resolved.placement : null;
    const at = placement && "Ok" in placement ? placement.Ok.destination : destination;
    const ghost = {
      variant: this.variant,
      destination: at,
      facing,
      valid: placement ? "Ok" in placement : null,
      spots: this.shape.map(([dx, dy]): [number, number] => [at[0] + dx, at[1] + dy]),
    };
    this.intent = { client, spot, revision, cursor: destination, ghost };
    this.ghost = ghost;
    this.resolve();
    return ghost;
  }
  private resolve() {
    const intent = this.intent;
    if (
      !intent ||
      this.inFlight ||
      (this.resolved?.spot === intent.spot && this.resolved.revision === intent.revision)
    )
      return;
    this.inFlight = true;
    void intent.client
      .previewPurchase(intent.ghost.variant, intent.cursor)
      .then(
        (placement) => {
          if (this.intent?.spot !== intent.spot) return;
          this.resolved = { spot: intent.spot, revision: intent.revision, placement };
          if ("Ok" in placement) {
            const [x, y] = placement.Ok.destination;
            this.shape = placement.Ok.spots.map(([sx, sy]) => [sx - x, sy - y]);
          }
        },
        () => {
          if (this.intent?.spot === intent.spot)
            this.resolved = {
              spot: intent.spot,
              revision: intent.revision,
              placement: { Err: { reason: "preview_unavailable" } },
            };
        },
      )
      .finally(() => {
        this.inFlight = false;
        this.resolve();
      });
  }
  /** The ghost the authority accepted, or null when nothing was placed. */
  async confirm(
    send: (order: Order) => Promise<CommandAck | null | undefined>,
  ): Promise<PurchaseGhost | null> {
    const ghost = this.ghost;
    if (
      !ghost ||
      this.resolved?.spot !== this.intent?.spot ||
      !this.resolved ||
      !("Ok" in this.resolved.placement) ||
      this.confirming
    )
      return null;
    const generation = this.generation;
    this.confirming = true;
    try {
      const ack = await send({
        kind: "confirm_purchase",
        variant: ghost.variant,
        destination: ghost.destination,
      });
      if (!ack || ack.error) return null;
      if (generation === this.generation) this.cancel();
      return { ...ghost, valid: true };
    } finally {
      this.confirming = false;
    }
  }
}

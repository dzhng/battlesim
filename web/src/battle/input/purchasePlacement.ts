import type { CommandAck, Order, PurchasePlacement } from "../sim/protocol";
import type { SimClient } from "../sim/client";
export interface PurchaseGhost {
  variant: string;
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
  private intent: { client: SimClient; key: string; ghost: PurchaseGhost } | null = null;
  private resolved: { key: string; placement: PurchasePlacement } | null = null;
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
    facing = -Math.PI / 2,
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
    const key = JSON.stringify([this.generation, this.variant, destination, revision]);
    const ghost = {
      variant: this.variant,
      destination,
      facing,
      valid: this.resolved?.key === key ? "Ok" in this.resolved.placement : null,
      spots: this.shape.map(([dx, dy]): [number, number] => [
        destination[0] + dx,
        destination[1] + dy,
      ]),
    };
    this.intent = { client, key, ghost };
    this.ghost = ghost;
    this.resolve();
    return ghost;
  }
  private resolve() {
    const intent = this.intent;
    if (!intent || this.inFlight || this.resolved?.key === intent.key) return;
    this.inFlight = true;
    void intent.client
      .previewPurchase(intent.ghost.variant, intent.ghost.destination)
      .then(
        (placement) => {
          if (this.intent?.key !== intent.key) return;
          this.resolved = { key: intent.key, placement };
          if ("Ok" in placement) {
            const [x, y] = intent.ghost.destination;
            this.shape = placement.Ok.spots.map(([sx, sy]) => [sx - x, sy - y]);
          }
        },
        () => {
          if (this.intent?.key === intent.key)
            this.resolved = {
              key: intent.key,
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
      this.resolved?.key !== this.intent?.key ||
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

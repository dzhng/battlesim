import type { SimClient } from "@web/battle/sim/client";
import type { BuildingPlacement, MoveDestination } from "@web/battle/sim/protocol";
import type { PointerPreview } from "./pointerPaint";

type PreviewAvailability = "idle" | "pending" | "ready" | "blocked";

export interface OrderPreviewSnapshot {
  destinations: MoveDestination[];
  building: BuildingPlacement | null;
  state: PreviewAvailability;
  /** While the current gesture's query is in flight, what the previous
   *  gesture of the same kind settled on: the cursor keeps its badge
   *  through the gap instead of flickering to the plain arrow. Its marks
   *  never carry; a stale goal's destinations would sit in the wrong place. */
  carried?: PreviewAvailability;
}

type PreviewClient = Pick<SimClient, "previewMove" | "previewBuilding">;

interface Answer {
  key: string;
  destinations: MoveDestination[];
  building: BuildingPlacement | null;
  /** A rejected building query means the target cannot accept this order. */
  state?: PreviewAvailability;
}

const IDLE: OrderPreviewSnapshot = {
  destinations: [],
  building: null,
  state: "idle",
};

function availability(
  destinations: readonly MoveDestination[],
  building: BuildingPlacement | null,
): PreviewAvailability {
  return building?.entries.length || destinations.some((destination) => destination.placed)
    ? "ready"
    : building
      ? "blocked"
      : "pending";
}

/** Owns asynchronous move/building admission while pointer paint only draws it. */
export class OrderPreview {
  private client: PreviewClient | null = null;
  private gesture = "";
  private generation = 0;
  private active: number | null = null;
  private answer: Answer | null = null;
  private kind: PointerPreview["kind"] | null = null;
  private carried: PreviewAvailability | undefined;

  get snapshot(): OrderPreviewSnapshot {
    if (this.answer) {
      const { destinations, building, state } = this.answer;
      return { destinations, building, state: state ?? availability(destinations, building) };
    }
    return this.gesture
      ? { destinations: [], building: null, state: "pending", carried: this.carried }
      : IDLE;
  }

  /**
   * Advances the current preview on the same frame cadence as the cursor.
   * A new `revision` (the battle moved on) or facing asks again and keeps
   * the last answer until the replacement lands: the marks and badge never
   * blink while the pointer rests. A new request or `identity` (a new press)
   * drops it. The selected-unit
   * input stays at this seam with the existing controller call; PointerPaint
   * still projects the current unit inputs into markers.
   */
  update(
    intent: PointerPreview | null,
    client: PreviewClient | null,
    revision: number | string,
    identity = "",
  ): OrderPreviewSnapshot {
    const gesture =
      intent && client
        ? JSON.stringify([intent.kind, { ...intent.request, facing: undefined }, identity])
        : "";
    if (gesture !== this.gesture || client !== this.client) {
      const kind = gesture ? intent!.kind : null;
      this.carried =
        client === this.client && kind === this.kind
          ? this.answer
            ? availability(this.answer.destinations, this.answer.building)
            : this.carried
          : undefined;
      this.kind = kind;
      this.generation += 1;
      this.gesture = gesture;
      this.client = client;
      this.answer = null;
    }

    const key = gesture && intent && JSON.stringify([intent, revision]);
    if (intent && client && key && !this.active && this.answer?.key !== key) {
      this.start(intent, client, key);
    }
    return this.snapshot;
  }

  private start(intent: PointerPreview, client: PreviewClient, key: string) {
    const request = this.generation;
    this.active = request;
    const query =
      intent.kind === "move"
        ? client
            .previewMove(intent.request)
            .then((destinations) => ({ destinations, building: null }))
        : client
            .previewBuilding(intent.request)
            .then((building) => ({ destinations: [], building }));
    void query
      .then(
        ({ destinations, building }) => {
          if (request !== this.generation) return;
          this.answer = {
            key,
            destinations,
            building,
          };
        },
        () => {
          if (request !== this.generation) return;
          this.answer = {
            key,
            destinations: [],
            building: null,
            state: intent.kind === "building" ? "blocked" : "pending",
          };
        },
      )
      .finally(() => {
        if (this.active === request) this.active = null;
      });
  }
}

import type { OwnUnitView } from "@web/battle/sim/observationView";
import type { SimClient } from "@web/battle/sim/client";
import type { BuildingPlacement, MoveDestination } from "@web/battle/sim/protocol";
import type { PointerPreview } from "./pointerPaint";

type PreviewAvailability = "idle" | "pending" | "ready" | "blocked";

export interface OrderPreviewSnapshot {
  destinations: MoveDestination[];
  building: BuildingPlacement | null;
  state: PreviewAvailability;
}

type PreviewClient = Pick<SimClient, "previewMove" | "previewBuilding">;

interface Answer {
  key: string;
  destinations: MoveDestination[];
  building: BuildingPlacement | null;
}

interface ActiveRequest {
  generation: number;
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
  return building?.entrant || destinations.some((destination) => destination.placed)
    ? "ready"
    : !building || building.unproven
      ? "pending"
      : "blocked";
}

/** Owns asynchronous move/building admission while pointer paint only draws it. */
export class OrderPreview {
  private client: PreviewClient | null = null;
  private gesture = "";
  private generation = 0;
  private active: ActiveRequest | null = null;
  private answer: Answer | null = null;

  get snapshot(): OrderPreviewSnapshot {
    if (this.answer) {
      const { destinations, building } = this.answer;
      return { destinations, building, state: availability(destinations, building) };
    }
    return this.gesture ? { destinations: [], building: null, state: "pending" } : IDLE;
  }

  /**
   * Advances the current preview on the same frame cadence as the cursor.
   * Facing is excluded from the gesture identity, so its last answer remains
   * available while the replacement query is in flight. The selected-unit
   * input stays at this seam with the existing controller call; PointerPaint
   * still projects the current unit inputs into markers.
   */
  update(
    intent: PointerPreview | null,
    _selected: readonly OwnUnitView[],
    client: PreviewClient | null,
    revision: number | string,
    identity = "",
  ): OrderPreviewSnapshot {
    const gesture =
      intent && client
        ? JSON.stringify([intent.kind, { ...intent.request, facing: undefined }, identity])
        : "";
    if (gesture !== this.gesture || client !== this.client) {
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
    const request: ActiveRequest = { generation: this.generation };
    this.active = request;
    const query =
      intent.kind === "move"
        ? client
            .previewMove(intent.request)
            .then((destinations) => ({ destinations, building: null }))
        : client.previewBuilding(intent.request).then((building) => ({
            destinations: building.destinations,
            building,
          }));
    void query
      .then(
        ({ destinations, building }) => {
          if (request.generation !== this.generation) return;
          this.answer = {
            key,
            destinations,
            building,
          };
        },
        () => {
          if (request.generation !== this.generation) return;
          this.answer = { key, destinations: [], building: null };
        },
      )
      .finally(() => {
        if (this.active === request) this.active = null;
      });
  }
}

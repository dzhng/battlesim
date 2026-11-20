import type { SimClient } from "@web/battle/sim/client";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import type {
  BuildingPlacement,
  MoveDestination,
  MovePreviewRequest,
} from "@web/battle/sim/protocol";
import type { PointerPreview } from "./pointerPaint";

type PreviewAvailability = "idle" | "pending" | "ready" | "blocked";

export interface OrderPreviewSnapshot {
  destinations: MoveDestination[];
  building: BuildingPlacement | null;
  state: PreviewAvailability;
  /** The page's own mark for a move the authority has not answered within
   *  `PROVISIONAL_AFTER_MS` (`provisionalDestinations`); else empty. An answer
   *  with no destinations is an answer; a query that failed is not. */
  provisional: MoveDestination[];
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
  provisional: [],
};

/** How long a gesture waits for the authority's destinations before the page
 *  marks the point itself: under what reads as a delay, over what a local
 *  authority takes, so offline play never shows the page's own mark. */
export const PROVISIONAL_AFTER_MS = 50;

/** What a move to a point shows before the authority has answered: one mark
 *  at the point, for the unit that leads. A remote authority's answer is a
 *  round trip away, and an order that shows nothing until then feels lost;
 *  the answer, when it lands, is the only word on where each unit stands. */
function provisionalDestinations(
  move: MovePreviewRequest,
  own: readonly OwnUnitView[],
): MoveDestination[] {
  const lead = own.find((unit) => unit.id === move.units[0]);
  if (!lead) return [];
  const [x, y] = move.goal;
  const facing = move.facing ?? Math.atan2(y - lead.position[1], x - lead.position[0]);
  return [{ unit: lead.id, goal: [x, y], facing, placed: true, spots: [] }];
}

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
  /** The move in hand, the units it may lead, and when its gesture began. */
  private move: MovePreviewRequest | null = null;
  private own: readonly OwnUnitView[] = [];
  private since = 0;

  constructor(private readonly now: () => number = () => performance.now()) {}

  get snapshot(): OrderPreviewSnapshot {
    if (this.answer) {
      const { destinations, building, state } = this.answer;
      return {
        destinations,
        building,
        state: state ?? availability(destinations, building),
        // Only a failed query carries a state of its own.
        provisional: state === undefined ? [] : this.provisional(),
      };
    }
    return this.gesture
      ? {
          destinations: [],
          building: null,
          state: "pending",
          provisional: this.provisional(),
          carried: this.carried,
        }
      : IDLE;
  }

  private provisional(): MoveDestination[] {
    return this.move && this.now() - this.since >= PROVISIONAL_AFTER_MS
      ? provisionalDestinations(this.move, this.own)
      : [];
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
    own: readonly OwnUnitView[] = [],
  ): OrderPreviewSnapshot {
    this.own = own;
    this.move = intent?.kind === "move" && client ? intent.request : null;
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
      this.since = this.now();
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

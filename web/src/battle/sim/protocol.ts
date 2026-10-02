/** The wire contract between the battle authority (one simulation in a worker)
 * and the main-thread client: ordered commands, acknowledgements, and one
 * completed-tick publication per credit. There is no simulation-shaped proxy. */

export type SideName = "blue" | "red";
export type RoutePolicy = "shortest" | "fastest";
/** Mirrors `contract::command::MoveDirection`: a reverse move backs along
 *  its route with the facing held (Q31). */
export type MoveDirection = "forward" | "reverse";

export type Engagement = "fire_at_will" | "return_fire_only";

/** Mirrors `contract::command::TargetRef`: side-scoped handles only. */
export type TargetRef =
  | { kind: "identified"; id: number }
  | { kind: "contact"; id: number }
  | { kind: "ground"; point: [number, number, number] };

/** Mirrors `contract::command::Order` (serde tag = "kind"). */
export type Order =
  | {
      kind: "move";
      units: number[];
      gesture: number;
      goal: [number, number];
      route: RoutePolicy;
      /** Omitted means forward. */
      direction?: MoveDirection;
      /** A right-drag's facing (Q9): world bearing in radians,
       *  counter-clockwise from +X. Omitted: the direction of travel. */
      facing?: number;
    }
  | { kind: "stop"; units: number[] }
  | { kind: "attack"; units: number[]; target: TargetRef }
  | { kind: "attack_move"; units: number[]; gesture: number; goal: [number, number] }
  | { kind: "set_engagement"; units: number[]; policy: Engagement }
  /** Set up in place (true) or pack for movement (false); others ignore it. */
  | { kind: "set_deployment"; units: number[]; deployed: boolean }
  /** Walk to the building and enter it as whole squads (refused if they do not all fit). */
  | { kind: "garrison"; units: number[]; building: number }
  /** Leave the building after a stationary timer. */
  | { kind: "exit_building"; units: number[] }
  | { kind: "upgrade_move"; gesture: number; route: RoutePolicy };

export interface CommandEnvelope {
  side: SideName;
  seq: number;
  order: Order;
  queued: boolean;
}

export interface MovePreviewRequest {
  units: number[];
  goal: [number, number];
  facing?: number;
  direction?: MoveDirection;
}

/** The same per-unit destinations used when the move is committed. */
export interface MoveDestination {
  unit: number;
  goal: [number, number];
  facing: number;
  placed: boolean;
}

export interface OrderError {
  reason: string;
  [detail: string]: unknown;
}

export interface CommandAck {
  seq: number;
  applied_tick: number;
  error: OrderError | null;
  placement?: { gesture: number; destinations: MoveDestination[] };
}

export type AuthorityStatus =
  | "loading"
  | "ready"
  | "running"
  | "paused"
  | "hidden"
  | "waiting-consumer"
  | "failed"
  | "disposed";

export type SimRequest =
  | {
      type: "init";
      scenario: string;
      seed: number;
      side: SideName;
      /** Replay these accepted commands instead of taking input. */
      replay?: string;
      /** Blue is played by this comparison script (`village_report`'s name,
       *  e.g. "scout-suppress-flank"); blue input is then refused. */
      script?: string;
    }
  /** The battle is on screen: nothing ticks before this. */
  | { type: "start" }
  | { type: "command"; command: CommandEnvelope }
  | { type: "move_preview"; id: number; side: SideName; move: MovePreviewRequest }
  /** A consumed publication buffer returned to the producer. */
  | { type: "credit"; buffer: ArrayBuffer }
  | { type: "pause" }
  | { type: "resume" }
  | { type: "hidden"; hidden: boolean }
  /** Advance exactly `ticks` while paused, as fast as credit allows. */
  | { type: "advance"; id: number; ticks: number }
  | { type: "replay" }
  /** Lab diagnostic: publish the other side's observation from now on. Its
   *  ground arrives as a new epoch's full snapshot. */
  | { type: "side"; side: SideName }
  | { type: "dispose" };

export type SimReply =
  | { type: "ready"; layout: string; tickHz: number; tick: number }
  | { type: "ack"; ack: CommandAck }
  | { type: "move_preview"; id: number; destinations: MoveDestination[] }
  | {
      type: "publication";
      tick: number;
      digest: string;
      length: number;
      buffer: ArrayBuffer;
      /** Wall time of this tick's step (script orders included), ms. */
      stepMs: number;
    }
  | { type: "status"; status: AuthorityStatus; slow: boolean }
  | { type: "advanced"; id: number; tick: number }
  | { type: "replay"; json: string }
  | { type: "error"; message: string };

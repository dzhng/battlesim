/** The wire contract between the battle authority (one simulation in a worker)
 * and the main-thread client: ordered commands, acknowledgements, and one
 * completed-tick publication per credit. There is no simulation-shaped proxy. */

export type SideName = "blue" | "red";
export type RoutePolicy = "shortest" | "fastest";

export type Engagement = "fire_at_will" | "return_fire_only";

/** Mirrors `contract::command::TargetRef`: side-scoped handles only. */
export type TargetRef =
  | { kind: "identified"; id: number }
  | { kind: "contact"; id: number }
  | { kind: "ground"; point: [number, number, number] };

/** Mirrors `contract::command::Order` (serde tag = "kind"). */
export type Order =
  | { kind: "move"; units: number[]; gesture: number; goal: [number, number]; route: RoutePolicy }
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

export interface OrderError {
  reason: string;
  [detail: string]: unknown;
}

export interface CommandAck {
  seq: number;
  applied_tick: number;
  error: OrderError | null;
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
  /** A consumed publication buffer returned to the producer. */
  | { type: "credit"; buffer: ArrayBuffer }
  | { type: "pause" }
  | { type: "resume" }
  | { type: "hidden"; hidden: boolean }
  /** Advance exactly `ticks` while paused, as fast as credit allows. */
  | { type: "advance"; id: number; ticks: number }
  | { type: "replay" }
  /** Lab diagnostic: the authoritative ground layer's marked cells, whoever
   *  saw them (the flat cell debug view; slice 08 delivers a side's view). */
  | { type: "ground" }
  /** Lab diagnostic: publish the other side's observation from now on. */
  | { type: "side"; side: SideName }
  | { type: "dispose" };

export type SimReply =
  | { type: "ready"; layout: string; tickHz: number; tick: number }
  | { type: "ack"; ack: CommandAck }
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
  /** `[cell_m]`, then `[x, y, crater, scorch, tracks, trampled]` per marked cell. */
  | { type: "ground"; cells: Float32Array }
  | { type: "error"; message: string };

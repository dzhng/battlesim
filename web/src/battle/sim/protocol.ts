/** The wire contract between the battle authority (one simulation in a worker)
 * and the main-thread client: ordered commands, acknowledgements, and one
 * completed-tick publication per credit. There is no simulation-shaped proxy. */

export type SideName = "blue" | "red";
export type RoutePolicy = "shortest" | "fastest";

/** Mirrors `contract::command::Order` (serde tag = "kind"). */
export type Order =
  | { kind: "move"; units: number[]; gesture: number; goal: [number, number]; route: RoutePolicy }
  | { kind: "stop"; units: number[] }
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
  | { type: "init"; scenario: string; seed: number; side: SideName; replay?: string }
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
  /** Lab diagnostic: publish the other side's observation from now on. */
  | { type: "side"; side: SideName }
  | { type: "dispose" };

export type SimReply =
  | { type: "ready"; layout: string; tickHz: number; tick: number }
  | { type: "ack"; ack: CommandAck }
  | { type: "publication"; tick: number; digest: string; length: number; buffer: ArrayBuffer }
  | { type: "status"; status: AuthorityStatus; slow: boolean }
  | { type: "advanced"; id: number; tick: number }
  | { type: "replay"; json: string }
  | { type: "error"; message: string };

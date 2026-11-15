import type {
  BuildingPlacement,
  BuildingPreviewRequest,
  CommandAck,
  MoveDestination,
  MovePreviewRequest,
  Order,
  PurchasePlacement,
} from "../../../web/src/battle/sim/protocol";
import type { ControllerKind } from "../../../web/src/battle/prepare/protocol";
import type { MapChoice, MapSize, MapType } from "../../../web/src/maps/source";
import type { Faction } from "../../scene-assets/src/units";
import type { SeatId } from "../../skirmish-setup/src/seats";

export type RecordKind = "open" | "publication" | "status" | "state" | "end";
export interface StreamRecord {
  id: number;
  kind: RecordKind;
  flags: number;
  payload: Uint8Array;
}
export interface StreamOpen {
  resume: "continued" | "keyframe";
  layout?: string;
  tickHz?: number;
  /** A battle's status as last said, for a reader that joins after it was. */
  status?: { status: "running" | "finished" | "failed"; slow: boolean };
}

export type UpBatch = {
  seq: number;
  items: { id: number; order: Order; queued: boolean }[];
};
export type UpReply =
  | { seq: number; results: { id: number; ack: CommandAck }[] }
  | { seq: number; refused: "not_seated" | "rate" };

export type PreviewQuery =
  | { kind: "purchase"; variant: string; destination: [number, number] }
  | { kind: "move"; move: MovePreviewRequest }
  | { kind: "building"; building: BuildingPreviewRequest };
export type PreviewAnswer =
  | { purchase: PurchasePlacement }
  | { moves: MoveDestination[] }
  | { building: BuildingPlacement | null }
  | { dropped: true };

export type BuildIdentity = { commit: string; sim: string };

export type BattleSetup = {
  seats: { player: string; side: "blue" | "red"; controller: ControllerKind }[];
  factions: Record<"blue" | "red", Faction>;
  map: MapChoice;
  battleSeed: number;
  lighting: string;
  rules: "skirmish" | "frontline";
};
export type BattleTicket = {
  /** Public player names, keyed by battle player id; never browser credentials. */
  callsigns: Record<string, string>;
  battle: string;
  setup: BattleSetup;
  mapIdentity: string;
  side: "blue" | "red";
  player: string;
};

export type RoomCode = string;
export type SeatView = {
  id: SeatId;
  side: "blue" | "red";
  occupant:
    | { kind: "open" }
    | { kind: "ai" }
    | { kind: "human"; callsign: string; ready: boolean; host: boolean };
};
export type RoomView = {
  code: RoomCode;
  visibility: "public" | "private";
  seats: SeatView[];
  settings: {
    factions: Record<"blue" | "red", Faction>;
    map: MapChoice;
    lighting: string;
  };
  you: SeatId;
  canStart: boolean;
  phase: { kind: "lobby" } | { kind: "starting" } | { kind: "battle"; ticket: BattleTicket };
};
export type RoomList = {
  rooms: {
    code: RoomCode;
    host: string;
    map: MapChoice;
    blue: ("human" | "open" | "ai")[];
    red: ("human" | "open" | "ai")[];
  }[];
};

export type RoomSettingsPatch = {
  map?: {
    type?: MapType;
    size?: MapSize;
    region?: string | null;
    seed?: string;
  };
  factions?: Partial<Record<"blue" | "red", Faction>>;
  lighting?: string;
};

export type RoomAction =
  | { kind: "sit"; seat: SeatId }
  | { kind: "ready"; ready: boolean }
  | { kind: "leave" }
  | { kind: "add_seat"; side: "blue" | "red" }
  | { kind: "remove_seat"; seat: SeatId }
  | { kind: "set_seat"; seat: SeatId; to: "open" | "ai" }
  | { kind: "settings"; patch: RoomSettingsPatch }
  | { kind: "start" };

export type Briefing = {
  battle: string;
  side: "blue" | "red";
  seat: number;
  credits: number;
  scores: [number, number];
  target: number;
  minutesIn: number;
  setup: BattleSetup;
  mapIdentity: string;
  roster: { side: "blue" | "red"; callsign: string | null }[];
};

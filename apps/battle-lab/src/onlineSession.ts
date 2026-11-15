// The ticket and address travel in history state, never credentials.
import type { BattleTicket } from "@packages/multiplayer/src/wire";

export interface OnlineBattleEntry {
  room: string;
  ticket: BattleTicket;
  address: string;
}

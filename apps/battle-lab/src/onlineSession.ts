// P23 dev entry. The ticket and address travel in history state, never credentials.
import type { BattleTicket } from "@packages/multiplayer/src/wire";

export interface OnlineBattleEntry {
  ticket: BattleTicket;
  address: string;
}

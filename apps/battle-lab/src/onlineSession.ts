// P23 dev entry. Credentials stay in this tab's storage, outside router state.
import type { BattleTicket } from "@packages/multiplayer/src/wire";

export interface OnlineBattleEntry {
  ticket: BattleTicket;
  address: string;
}

export function browserId(): string {
  const stored = sessionStorage.getItem("battle.browser-id");
  if (stored) return stored;
  const id = Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  sessionStorage.setItem("battle.browser-id", id);
  return id;
}

/** Only local scaffolding knows a local address. Deployed discovery is slice 09. */
export function devServerAddress(search: string): string {
  return import.meta.env.DEV
    ? (new URLSearchParams(search).get("server") ?? "http://127.0.0.1:8787")
    : "http://127.0.0.1:8787";
}

import { BROWSER_ID_HEADER } from "../../../../packages/multiplayer/src/identity";
import { RequestRefused } from "@packages/multiplayer/src/refusal";
import { browserId } from "./identity";

export const ROOM_CLOSED_NOTICE = "That room has closed.";
export const ROOM_REMOVED_NOTICE = "You are no longer in that room.";

export const ROOM_BATTLE_FAILED_NOTICE = "The battle could not continue. The room has closed.";

export async function onlineRequest<T>(
  address: string,
  path: string,
  value: unknown,
  signal: AbortSignal,
): Promise<T> {
  const response = await fetch(`${address}${path}`, {
    method: "POST",
    headers: { [BROWSER_ID_HEADER]: browserId(), "Content-Type": "application/json" },
    body: JSON.stringify(value),
    signal,
  });
  const result = await response.json();
  if ("refused" in result) throw new RequestRefused(result.refused);
  if (!response.ok) throw new Error("Online request failed.");
  return result as T;
}

export function onlineFailure(error: unknown): string {
  if (error instanceof RequestRefused) {
    switch (error.reason) {
      case "removed":
        return ROOM_REMOVED_NOTICE;
      case "game_running":
        return "You already have a game running.";
      case "full":
        return "The servers are full. Try again later.";
      case "unknown":
        return "Room code not found. Check it and try again.";
      case "no_open_seat":
        return "That room has no open seats.";
      case "started":
        return "That room has already started.";
      default:
        return error.reason;
    }
  }
  return "Could not reach the room. Try again.";
}

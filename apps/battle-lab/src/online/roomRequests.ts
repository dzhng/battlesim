import { StreamRefused } from "@packages/multiplayer/src/stream/http";
import { browserId } from "./identity";

export async function roomRequest<T>(
  address: string,
  path: string,
  value: unknown,
  signal: AbortSignal,
): Promise<T> {
  const response = await fetch(`${address}${path}`, {
    method: "POST",
    headers: { "X-Player": browserId(), "Content-Type": "application/json" },
    body: JSON.stringify(value),
    signal,
  });
  const result = await response.json();
  if ("refused" in result) throw new StreamRefused(result.refused);
  if (!response.ok) throw new Error("Room request failed.");
  return result as T;
}

export function roomFailure(error: unknown): string {
  if (error instanceof StreamRefused) {
    switch (error.reason) {
      case "removed":
        return "You are no longer in that room.";
      case "full":
        return "The servers are full. Try again later.";
      case "unknown":
        return "Room code not found. Check it and try again.";
      case "throttled":
        return "Too many guesses. Wait a moment and try again.";
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

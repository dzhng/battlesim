import { BROWSER_ID_HEADER } from "../../../../packages/multiplayer/src/identity";
import { buildIdentity } from "../buildIdentity";
import { browserId, callsign } from "./identity";

export const ONLINE_BUILD_UNAVAILABLE = "Online play is not available on this build.";
export const ONLINE_UNREACHABLE = "Online play cannot be reached right now.";

/** Only these public reasons may reach the unavailable notice. */
export class OnlineUnavailable extends Error {
  constructor(reason: "build" | "unreachable") {
    super(reason === "build" ? ONLINE_BUILD_UNAVAILABLE : ONLINE_UNREACHABLE);
  }
}
export const onlineUnavailable = (error: unknown) =>
  error instanceof OnlineUnavailable ? error.message : ONLINE_UNREACHABLE;

/** A refused hello stops this visit before any seat or stream is requested. */
export async function hello(address: string, signal: AbortSignal): Promise<string> {
  try {
    const response = await fetch(`${address}/hello`, {
      method: "POST",
      headers: { [BROWSER_ID_HEADER]: browserId(), "Content-Type": "application/json" },
      body: JSON.stringify({ build: await buildIdentity(), callsign: callsign() }),
      signal,
    });
    const result: { callsign: string } | { refused: string } = await response.json();
    if ("refused" in result)
      throw new OnlineUnavailable(result.refused === "build" ? "build" : "unreachable");
    if (!response.ok) throw new OnlineUnavailable("unreachable");
    return result.callsign;
  } catch (error) {
    if (signal.aborted || error instanceof OnlineUnavailable) throw error;
    throw new OnlineUnavailable("unreachable");
  }
}

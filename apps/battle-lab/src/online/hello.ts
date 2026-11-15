import { buildIdentity } from "../buildIdentity";
import { browserId, callsign } from "./identity";

/** A build refusal is terminal for this visit: no seat or stream is requested. */
export async function hello(address: string, signal: AbortSignal): Promise<void> {
  const response = await fetch(`${address}/hello`, {
    method: "POST",
    headers: { "X-Player": browserId(), "Content-Type": "application/json" },
    body: JSON.stringify({ build: await buildIdentity(), callsign: callsign() }),
    signal,
  });
  const result: { callsign: string } | { refused: string } = await response.json();
  if ("refused" in result) {
    if (result.refused === "build") throw new Error("Online play is not available on this build.");
    throw new Error("Online play is not available.");
  }
  if (!response.ok) throw new Error("Online play is not available.");
}

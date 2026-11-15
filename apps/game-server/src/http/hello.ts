import { cleanCallsign } from "../../../../packages/multiplayer/src/callsign.ts";
import type { BuildIdentity } from "../../../../packages/multiplayer/src/wire.ts";

/** Names and credentials belong to this server lifetime, never to a battle ticket. */
export function helloRegistry(build: BuildIdentity) {
  const names = new Map<string, { base: string; callsign: string }>();
  return (id: string, value: unknown): { callsign: string } | { refused: string } => {
    if (!value || typeof value !== "object") return { refused: "invalid" };
    const page = value as { build?: BuildIdentity; callsign?: unknown };
    if (page.build?.commit !== build.commit || page.build?.sim !== build.sim)
      return { refused: "build" };
    if (typeof page.callsign !== "string") return { refused: "invalid" };
    const base = cleanCallsign(page.callsign);
    if (!base) return { refused: "callsign" };
    const own = names.get(id);
    if (own?.base === base || own?.callsign === page.callsign) return { callsign: own.callsign };
    const occupied = new Set(
      [...names].filter(([browser]) => browser !== id).map(([, name]) => name.callsign),
    );
    let callsign = base;
    for (let suffix = 2; occupied.has(callsign); suffix++) callsign = `${base}#${suffix}`;
    names.set(id, { base, callsign });
    return { callsign };
  };
}

// The free commands: what is missing, stale or unresolved (`status`), and what
// has been spent (`costs`), from the pins, the roster and the ledger.
import { execFileSync } from "node:child_process";
import { join } from "node:path";

import { calls, read, unresolved } from "./journal.mjs";
import { readPins } from "./pins.mjs";
import { armies, compose, keyOf, parseKey } from "./prompt.mjs";

/** The family whose pins are soldier bodies keyed by (army, look, kind). */
export const SOLDIERS = "infantry";

/** Each (army, kind) the roster's kits wear, from `infantry_roster.py`, the
 * owner of which armies build which kits. */
export const rosterBodies = (repo) =>
  JSON.parse(
    execFileSync(
      "python3",
      [
        "-c",
        "import json, infantry_roster as r; print(json.dumps(sorted({(a, r.kind_of(k)) for k, kit in r.KITS.items() for a in kit['armies']})))",
      ],
      { cwd: join(repo, "packages/scene-assets/blender"), encoding: "utf8" },
    ),
  );

const armyOf = (key, data) => {
  try {
    return parseKey(key, data).army;
  } catch {
    return null;
  }
};

/** {missing, stale, open}: soldier bodies the roster needs with no pin, pins
 * whose prompt is not today's composed prompt, and unresolved submissions.
 * `family` narrows to one family; `army` to one army's bodies. */
export function status(ctx, family, { army } = {}) {
  const data = armies(ctx.repo);
  const mine = (key) => !army || armyOf(key, data) === army;
  const open = unresolved(read(ctx.repo)).filter(
    (l) => (!family || l.family === family) && mine(l.key),
  );
  if (family && family !== SOLDIERS) return { missing: [], stale: [], open };
  const pins = readPins(ctx.repo, SOLDIERS).filter((p) => p.army && mine(keyOf(p)));
  const pinned = new Set(pins.map(keyOf));
  const missing = (ctx.bodies ?? rosterBodies)(ctx.repo)
    .flatMap(([a, kind]) =>
      Object.keys(data.armies[a]?.looks ?? {}).map((look) => keyOf({ army: a, look, kind })),
    )
    .filter((key) => mine(key) && !pinned.has(key));
  const stale = pins
    .filter((p) => {
      try {
        return p.prompt !== compose(p.army, p.look, p.kind, data);
      } catch {
        return true;
      }
    })
    .map((p) => ({
      key: keyOf(p),
      why: p.prompt ? "its prompt differs from the composed one" : "it has no prompt",
    }));
  return { missing, stale, open };
}

// Money in thousandths of a dollar, so sums are exact.
const mils = (usd) => Math.round((usd ?? 0) * 1000);
const usd = (m) => `$${(m / 1000).toFixed(2)}`;

/** Spend per key, per army (or family) and overall, every attempt counted
 * once: rejected candidates, failures (free) and submissions still running
 * (at their listed price). */
export function costs(ctx, family) {
  const data = armies(ctx.repo);
  const paid = calls(read(ctx.repo)).filter((l) => !family || l.family === family);
  const keys = new Map();
  const groups = new Map();
  let total = 0;
  let running = 0;
  for (const l of paid) {
    const m = mils(l.price_usd);
    const key = keys.get(l.key) ?? { calls: 0, m: 0 };
    keys.set(l.key, { calls: key.calls + 1, m: key.m + m });
    const group = armyOf(l.key, data) ?? l.family;
    groups.set(group, (groups.get(group) ?? 0) + m);
    total += m;
    if (l.status === "submitted") running += m;
  }
  const rows = (map, show) => [...map].sort(([a], [b]) => a.localeCompare(b)).map(show);
  return [
    `${family ?? "all families"}: ${paid.length} paid calls`,
    ...rows(
      keys,
      ([k, v]) => `  ${k.padEnd(40)} ${String(v.calls).padStart(3)} calls  ${usd(v.m).padStart(8)}`,
    ),
    "by army (or family)",
    ...rows(groups, ([g, m]) => `  ${g.padEnd(40)}            ${usd(m).padStart(8)}`),
    `total ${usd(total)}${running ? ` (${usd(running)} of it still running, at listed prices)` : ""}`,
  ].join("\n");
}

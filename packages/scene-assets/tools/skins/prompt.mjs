// The one composer of a soldier body's input-image prompt, from the army data
// in `blender/roster/armies.json`: the studio frame, the army's description,
// the look's clause and the kind's cue. A pin stores the text it was drawn
// from, so a pin whose prompt no longer matches `compose` is stale.
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

/** `roster/armies.json` of the checkout at `repo`. */
export const armies = (repo = REPO) =>
  JSON.parse(readFileSync(join(repo, "packages/scene-assets/blender/roster/armies.json"), "utf8"));

/** The prompt that draws `army`'s body in `look` for `kind`. */
export function compose(army, look, kind, data = armies()) {
  const a = data.armies[army];
  if (!a) throw new Error(`no army ${army} in armies.json`);
  if (!a.looks[look]) throw new Error(`${army} has no look ${look}`);
  if (!data.kinds[kind]) throw new Error(`no body kind ${kind} in armies.json`);
  return [data.studio, a.prompt, a.looks[look], data.kinds[kind].cue].join(" ");
}

/** A soldier body's key, `<army>_<look>_<kind>`, as its parts. */
export function parseKey(key, data = armies()) {
  const m = /^(.+)_([^_]+)_([^_]+)$/.exec(key);
  if (!m || !data.armies[m[1]]?.looks[m[2]] || !data.kinds[m[3]])
    throw new Error(`${key} is not <army>_<look>_<kind> of armies.json`);
  return { army: m[1], look: m[2], kind: m[3] };
}

export const keyOf = ({ army, look, kind }) => `${army}_${look}_${kind}`;

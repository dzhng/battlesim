// The paid-call ledger, `blender/skins/calls.jsonl`, and its only writer. One
// JSON line per paid step:
//
//   {at, family, key, step: image|generate, endpoint, options, inputs,
//    request_id, status: submitted|completed|failed, price_usd,
//    price_source: listed|reported, output}
//
// A generation is logged `submitted` the moment fal accepts it and again when
// it ends. A `submitted` line with no later line for its request is still
// running or was abandoned by a crash: it is resumed, never resubmitted. An
// image is drawn in one synchronous call, so it gets only its terminal line.
import { appendFileSync, existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

export const ledger = (repo) => join(repo, "packages/scene-assets/blender/skins/calls.jsonl");

/** Every line of the ledger, oldest first. */
export const read = (repo) => {
  const path = ledger(repo);
  if (!existsSync(path)) return [];
  return readFileSync(path, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
};

/** Append one line; fields keep the ledger's order. */
export function append(repo, line) {
  const ordered = {
    at: line.at,
    family: line.family,
    key: line.key,
    step: line.step,
    endpoint: line.endpoint,
    options: line.options,
    inputs: line.inputs ?? null,
    request_id: line.request_id ?? null,
    status: line.status,
    price_usd: line.price_usd,
    price_source: line.price_source,
    output: line.output ?? null,
  };
  appendFileSync(ledger(repo), `${JSON.stringify(ordered)}\n`);
  return ordered;
}

/** Each paid call once, as its latest line: a request's terminal line wins
 * over its `submitted` line; a line without a request id stands alone. */
export function calls(lines) {
  const byRequest = new Map();
  const loose = [];
  for (const line of lines) {
    if (line.request_id) byRequest.set(line.request_id, line);
    else loose.push(line);
  }
  return [...loose, ...byRequest.values()].sort((a, b) => (a.at < b.at ? -1 : a.at > b.at ? 1 : 0));
}

/** Submissions still waiting for their result. */
export const unresolved = (lines) => calls(lines).filter((l) => l.status === "submitted");

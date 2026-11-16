// The only writer of `blender/skins/<family>.json`; `blender/skin_pins.py` is
// the exporters' only reader (this tool reads it too, for `status`). A soldier body's pin is keyed by (army, look, kind) and
// says which candidate the export wears, with its hash, the request that made
// it, its input and the prompt that drew the input (`status` compares that
// prompt with `compose` to find stale bodies).
import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { byEndpoint, IMAGE } from "./generators.mjs";
import { read } from "./journal.mjs";
import { armies, parseKey } from "./prompt.mjs";

const file = (repo, family) => join(repo, "packages/scene-assets/blender/skins", `${family}.json`);

export const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

/** The pins of `family`, or none. */
export const readPins = (repo, family) =>
  existsSync(file(repo, family)) ? JSON.parse(readFileSync(file(repo, family), "utf8")) : [];

/** Put `record` in `family`'s pins in place of every pin `replaces` matches. */
export function writePin(repo, family, record, replaces) {
  const pins = readPins(repo, family).filter((p) => !replaces(p));
  pins.push(record);
  writeFileSync(file(repo, family), `${JSON.stringify(pins, null, 2)}\n`);
}

/** The pin of a soldier body `key`, if it has one. */
export function pinOf(repo, family, key) {
  const { army, look, kind } = parseKey(key, armies(repo));
  return readPins(repo, family).find((p) => p.army === army && p.look === look && p.kind === kind);
}

/** Choose the candidate `requestId` made for `key`: free, from the ledger. */
export function pin(ctx, family, key, requestId, { note } = {}) {
  const lines = read(ctx.repo);
  const made = lines.find(
    (l) =>
      l.request_id === requestId &&
      l.family === family &&
      l.key === key &&
      l.step === "generate" &&
      l.status === "completed",
  );
  if (!made)
    throw new Error(`no completed generation ${requestId} for ${family} ${key} in the ledger`);
  const generator = byEndpoint(made.endpoint);
  const [input] = Object.keys(made.inputs ?? {});
  const drawn = lines.find(
    (l) => l.step === "image" && l.status === "completed" && l.output === input,
  );
  const body = parseKey(key, armies(ctx.repo));
  const record = {
    ...body,
    front: generator.front,
    glb: made.output,
    sha256: sha256(join(ctx.repo, made.output)),
    generator: generator.generator,
    request_id: requestId,
    generated: made.at.slice(0, 10),
    terms: generator.terms,
    inputs: made.inputs,
    note:
      note ??
      `The input is a studio T-pose front drawn by ${IMAGE.name} through Bento's model gateway from the composed prompt.`,
    endpoint: made.endpoint,
    options: made.options,
    prompt: drawn?.options.prompt ?? null,
    image_model: drawn?.endpoint ?? null,
    image_request_id: drawn?.request_id ?? null,
  };
  writePin(
    ctx.repo,
    family,
    record,
    (p) => p.army === body.army && p.look === body.look && p.kind === body.kind,
  );
  ctx.log(`pinned ${made.output} for ${key}`);
  return record;
}

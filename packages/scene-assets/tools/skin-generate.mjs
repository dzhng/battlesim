// Generate skins, pin them and log what they cost. Paid calls go through the
// installed `bento` CLI (its login, its workspace), never a vendor key; every
// one is logged in blender/skins/calls.jsonl, a request is never paid twice
// (an unresolved one is resumed), and every candidate is kept beside the
// pinned ones in assets/source/skins/<family>/.
//
//   image    <family> <key>...                 draw each soldier body's input image ($0.04)
//   generate <family> <key>... [--image <png>] [--generator tripo|meshy] [--again]
//                                              generate each body from its latest image
//   pin      <family> <key> <request_id> [--note "<text>"]   choose a candidate (free)
//   status   [<family>] [--army <id>]          missing, stale and unresolved bodies (free)
//   costs    [<family>]                        spend per key, per army and overall (free)
//   airframe <family> <appearance> <views_dir> "<note>"
//            an airframe skin from its four guide views (blender/skin_views.py)
//
// A soldier body's key is `<army>_<look>_<kind>` (blender/roster/armies.json).
// `--again` pays for another candidate of a key that already has a pin;
// `--concurrency` (default 6) bounds the jobs in flight. The modules under
// skins/ say what each part owns.
import { parseArgs } from "node:util";

import { runBento } from "./skins/fal.mjs";
import { pin } from "./skins/pins.mjs";
import { REPO } from "./skins/prompt.mjs";
import { costs, status } from "./skins/report.mjs";
import { airframe, generate, image } from "./skins/steps.mjs";

const { values: flags, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    image: { type: "string" },
    generator: { type: "string", default: "tripo" },
    again: { type: "boolean", default: false },
    concurrency: { type: "string", default: "6" },
    note: { type: "string" },
    army: { type: "string" },
  },
});
const [command, ...args] = positionals;
const ctx = {
  repo: REPO,
  exec: runBento,
  fetch: globalThis.fetch,
  every: 15_000,
  deadline: 40 * 60_000,
  log: (line) => console.log(`${new Date().toISOString().slice(11, 19)} ${line}`),
};
const concurrency = Number(flags.concurrency);
const usage = () => {
  console.error(
    "usage: skin-generate.mjs image|generate|pin|status|costs|airframe … (see the head of this file)",
  );
  process.exit(2);
};

try {
  if (command === "image" && args.length >= 2) {
    await image(ctx, args[0], args.slice(1), { again: flags.again, concurrency });
  } else if (command === "generate" && args.length >= 2) {
    const { generator, again } = flags;
    await generate(ctx, args[0], args.slice(1), {
      image: flags.image,
      generator,
      again,
      concurrency,
    });
  } else if (command === "pin" && args.length === 3) {
    pin(ctx, ...args, { note: flags.note });
  } else if (command === "status") {
    const { missing, stale, open } = status(ctx, args[0], { army: flags.army });
    console.log(`missing (${missing.length}):${missing.map((k) => `\n  ${k}`).join("")}`);
    console.log(`stale (${stale.length}):${stale.map((s) => `\n  ${s.key}: ${s.why}`).join("")}`);
    console.log(
      `unresolved (${open.length}):${open.map((l) => `\n  ${l.family} ${l.key} ${l.request_id} (${l.endpoint}, since ${l.at})`).join("")}`,
    );
    if (open.length) console.log("rerun the generate (or airframe) command for a key to resume it");
  } else if (command === "costs") {
    console.log(costs(ctx, args[0]));
  } else if (command === "airframe" && args.length === 4) {
    await airframe(ctx, ...args);
  } else usage();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

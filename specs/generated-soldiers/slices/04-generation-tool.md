# 04 One generation tool: pins, resume, cost ledger

Node only. No Blender, and no paid calls except the airframe dry-run
comparison, which is free.

## Contract

One CLI pays for skins, pins them and logs every call. It never pays twice for
a request, and it keeps every candidate. Today:
- `packages/scene-assets/tools/skin-generate.mjs` is airframe-only (Hunyuan,
  four views). It calls fal directly with `FAL_KEY` and overwrites its output,
  with no retry and no cost log.
- The pilot used a throwaway shell poller instead.

## Seam

`tools/skin-generate.mjs` becomes the front of the CLI, over four modules
under `packages/scene-assets/tools/skins/`:

| Module | Owns |
|---|---|
| `fal.mjs` | The transport: Bento's fal proxy (`bento service proxy post/get` and storage upload), submit, poll and download. It retries transient failures (504, a reset connection, a timeout), up to 3 times per call. No vendor key ever enters the process. |
| `journal.mjs` | The only writer of `blender/skins/calls.jsonl`, one JSON line per paid step: `{at, family, key, step: image\|generate, endpoint, options, request_id, status: submitted\|completed\|failed, price_usd, price_source: listed\|reported, output}`. A `submitted` line with no terminal line is resumed, never resubmitted. |
| `pins.mjs` | The only writer of `blender/skins/<family>.json`. The pin schema gains `endpoint`, `options`, `prompt`, `image_model` and `image_request_id`. `skin_pins.py` (slice 01) stays the only reader. |
| `prompt.mjs` | The only composer: `compose(army, look, kind)` from `armies.json` (the studio frame, the army prompt, the look clause, the kind cue). The pin stores the composed text, so drift can be seen. |

The subcommands:

| Command | Does |
|---|---|
| `image <family> <key>` | Pays $0.04. Writes `<key>-front-<req8>.png` beside the skins and logs it. |
| `generate <family> <key> --image <png> [--generator tripo\|meshy]` | Pays, downloads `<key>-<req8>.glb`, and never overwrites. |
| `pin <family> <key> <request_id>` | Chooses a candidate. Free. |
| `status [<family>] [--army <id>]` | Lists missing pins, stale pins (stored prompt ≠ composed prompt) and unresolved submissions. Free. |
| `costs [<family>]` | Totals per key, per army and overall, rejected attempts included. Free. |
| `airframe …` | Today's flow, on the shared transport. |

Generator settings are data in the tool:
- Tripo: `tripo3d/h3.1/image-to-3d`, `texture_quality` and `geometry_quality`
  detailed, `face_limit` 40000, `orientation: align_image`, `pbr: false`,
  `front: "+x"`.
- Meshy: `meshy/v7.1/image-to-3d`, `pose_mode: t-pose`, `target_polycount`
  40000, `front: "-y"`.

Prices are listed per endpoint and option (README, cost). When Bento reports a
charge, it is recorded as `reported`.

Also in this slice:
- The pilot's two calls go into the ledger as history lines: the image $0.04
  and Tripo request `01a1deba-e7f9-…` $0.60.
- The direct `FAL_KEY` path is deleted.

## What you can run

`node packages/scene-assets/tools/skin-generate.mjs status infantry` lists the
29 missing bodies, and the pilot pin as stale, because it has no composed
prompt. `costs infantry` prints $0.64.

## Verification

One `node:test` file with a fake transport. This is money logic, where a fast
unit test earns its keep. It covers:
- a 504, then success;
- a `FAILED` status logged as failed, with nothing pinned;
- a restart with a `submitted` line resuming the poll and paying nothing;
- a second `generate` for an accepted key refused unless `--again` is given;
- `pin` writing the schema;
- `compose()` being deterministic;
- the airframe request body matching the old tool's, byte for byte.

There are no pictures, so no visual gate.

## Decisions delegated

The CLI's spelling, file layout inside `tools/skins/`, polling intervals and
concurrency (6 by default), and log formatting.

## Must stay green

The airframe pins and exports (unchanged), and the size ratchet.

## Feedback that would change it

A rule on retry spend (for example "never more than $X per body") becomes a
flag here.

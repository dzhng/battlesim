# Battle growth: a new unit type costs only its own art

**Status:** the engine, the transport and the stand-in are on `staging`
(slices 00–11), with the page-memory fix and the download warm-up. The GPU
preload is cancelled: the player's rule is zero configuration, so nothing
ships off by default. An infantry team's first sighting is down from
0.3–0.4 s to frames of about 95 ms (slices 13 and 12); slice 14 is the floor
under every arrival.
**Updated:** 2026-11-15.

This plan replaces the "shared GPU assets across generations" placeholder. That
placeholder's motive was memory during a swap. The motive now is a freeze the
player sees, and sharing GPU assets is one part of the fix.

## Next Agent Prompt

You are fixing the multi-second freeze that happens whenever a running battle
first fields a unit type it had not shown. The admission work report exists,
`battle-growth` prints it, and the [ledger](decisions.md) holds what it and
the reference battle measured. What is left, in order:

1. **Slice 14, fielding's result follows the new kinds**
   ([slice](slices/14-fielding-follows-the-new-kinds.md)): the floor under
   every arrival. Each growth rebuilds and serialises the whole fielded
   battle's description (2.6 million characters, a frame of about 59 ms).
   Not started.
2. Closeout: correct the stale WASM note in one-renderer, run everything
   once, then `close-spec`.

Settled since the engine shipped, all in the ledger: the page's lingering
gigabyte (fixed, below the base now), the pre-ship review's findings, and
the download warm-up (on for every skirmish, no smoothness cost measured).

Read, in this order: this file, the [decisions ledger](decisions.md), the slice
you are about to build, the [renderer contract](../../crates/renderer-core/README.md)
and [`renderer`](../../.agents/skills/renderer/SKILL.md). Before a slice that
changes what the player sees (10 and 11), read
[`game-ui`](../../.agents/skills/game-ui/SKILL.md).

Warnings:

- Every decision in the ledger is settled. Do not reopen one; if the work shows
  it is wrong, stop, record the evidence in the ledger and reslice.
- Each slice lists facts to confirm first. They were read from the code or
  inferred, not exercised. Confirm them before depending on them, and write
  what you found into the ledger.
- A decision the slice does not make and does not delegate is a gap in this
  plan. Record it in the ledger; do not settle it silently.
- Art work at an arrival shares one per-frame budget (`ART_WORK_BUDGET_MS`).
  Anything new that runs at staging goes through it, or it brings the burst
  back.
- Each checkout's Rust build adds 3–22 GB. Check free space before a cold
  build, and remove a checkout's build output when its branch has merged. A
  burst of unrelated failures after a build usually means it ran out of space.
- The scenes and GPU tests of every lane share one GPU lock, so a long probe
  in one lane holds up the others' scenes. Start scenes in the background and
  wait for the lock.
- Two implementers so far deleted or shortened comments to fit the file-size
  ratchet. Reject that in review: move code out by concept instead.

Checklist:

- [x] [00 Proof and probes](slices/00-proof-and-probes.md)
- [x] [01 Fielding and install](slices/01-fielding-and-install.md)
- [x] [02 Content identity](slices/02-content-identity.md)
- [x] [03 Geometry reuse](slices/03-geometry-reuse.md)
- [x] [04 Texture layers](slices/04-texture-layers.md)
- [x] [05a Cards bake together](slices/05a-cards-bake-together.md)
- [x] [05 Card reuse](slices/05-card-reuse.md)
- [x] [06 Statics carry](slices/06-statics-carry.md)
- [x] [07 Catalog delta](slices/07-catalog-delta.md)
- [x] [08 Browser transport and the growth gate](slices/08-browser-transport-and-growth-gate.md)
- [x] [08a The staging frame](slices/08a-the-staging-frame.md)
- [x] [09 Warm-up and large arrivals](slices/09-preload.md)
- [x] [10 Stand-in mechanics](slices/10-stand-in-mechanics.md)
- [x] [11 Stand-in look](slices/11-stand-in-look.md)
- [x] [12 An arrival over frames](slices/12-an-arrival-over-frames.md)
- [x] [13 A body draws its type's mesh](slices/13-a-body-draws-its-types-mesh.md)
- [ ] [14 Fielding's result follows the new kinds](slices/14-fielding-follows-the-new-kinds.md)
- [ ] Closeout: correct the stale WASM note in
  [one-renderer](../done/one-renderer/README.md) (confirm first that the
  exhaustion predates texture sharing), run everything once, then
  [`close-spec`](../../.agents/skills/close-spec/SKILL.md).

Before you end a pass, update this section: the status line, the checklist, the
next pickup point, and anything the next agent must not trip over.

## Goal

When a battle gains a unit type, the work done is proportional to the new art,
not to the catalog. The player never notices a type arriving.

## Contracts that change

- **Admission work report.** Every admission reports what it did:
  `AdmissionWork`, counted at the site that does the work (bundles decoded,
  vertex bytes packed, buffer bytes uploaded, texture layers written, cards
  baked, scenery placements run, and why a full rebuild ran if one did). It
  travels with the page's `art committed` diagnostics event, beside the
  browser's own counts (bundles encoded and sent, contents inflated, library
  installs). This is the proof for the whole plan.
- **The art store.** Shared Rust owns one store per scene and GPU device that
  outlives asset generations. A generation holds shared references into it. The
  first scene, a lost device and slot exhaustion are the same admission with an
  empty store, not a second path.
- **The WASM seam.** The shell declares the wanted art as name and content
  identity, and passes bytes only for content Rust does not hold.
- **The stand-in.** A unit the side sees whose art is not committed is drawn
  as one block per body, sized from its physical body, and can be selected and
  targeted. This is the only change to the picture.

Unchanged: shaders, the vertex layout, battle digests, side knowledge, and the
atomic swap (the old picture stays drawable while a generation prepares; stale,
failed or dropped work leaves live art intact).

## Measured baseline

One growth in the `battle-growth` skirmish at high graphics
(`SCENE_GRAPHICS_QUALITY=high`), commit `080bcbe`, with temporary timers
(since removed). The freeze was 2.5–2.9 s, followed by a
1.4 s wait.

| Cause | Cost | Fixed by |
| --- | --- | --- |
| 1. All model geometry repacked and re-uploaded, 1.3 GB (joining vertex arrays byte by byte 0.62 s, packing 0.32 s, upload 0.13 s, rest poses and other loop work about 0.5 s) | 1.56 s, main thread | 03 |
| 1b. Model texture arrays rebuilt; small textures upscaled per pixel | 0.26–0.38 s, main thread | 04 |
| 2. JS re-encodes all 201 art files (710 MB) and copies them into WASM | 0.42–0.50 s, main thread | 07, 08 |
| 3. Impostor cards re-baked one row at a time, nothing cached | 1.4 s, asynchronous | 05 |
| 4. Every fielded type's files inflated, joined and parsed again; the art library appears to install twice | 0.9–1.5 s stall before the freeze | 01 |
| Scenery placement, grass, bookkeeping, commit | about 0.08 s | 06 |

The reference workload is the player's large battle in
[the freeze report](assets/freeze-report-2026-11-14.json): five freezes of
2.9–9.9 s, each within 50 ms of `battle staged`, and 11–40 s from staging to
`art committed`. Its link is in the report.

## Slice graph

```
00 proof ─┬─ 01 fielding and install ────────────────────┐
          ├─ 02 identity ─┬─ 03 geometry ─ 04 textures ─ 05 cards ─┤
          │               └─ 07 catalog delta ─────────────────────┼─ 08 transport + gate ─ 09 warm-up
          ├─ 05a cards bake together ─ (05) ──────────────────────┤
          ├─ 06 statics carry (conditional) ─────────────────────┘
          └─ 10 stand-in mechanics ─ 11 stand-in look
```

Build in number order unless a reason appears. 01 is early because it is
independent and removes the stall and the doubled rebuild at once. 10 and 11
are an independent track.

| Slice | What you can run or see when it lands |
| --- | --- |
| 00 | `battle-growth` prints what one growth costs today; the ledger holds the confirmed facts |
| 01 | The stall before each freeze is gone; one install per new type |
| 02 | Nothing visible; identity tests |
| 03 | The largest part of the freeze is gone; geometry bytes at growth equal the new type's |
| 04 | Texture work at growth equals the new type's; swap memory no higher |
| 05a | The wait after staging drops from a minute to seconds |
| 05 | The wait after staging shrinks to the new type's cards |
| 06 | No scenery or grass work at growth |
| 07 | Nothing visible; the seam accepts a delta |
| 08 | The reference battle plays without a freeze at `battle staged`; the gate is green |
| 08a | The long frame at `battle staged` shrinks to what the new art costs |
| 09 | A battle opened by link warms its art in the background; the largest arrival no longer hitches |
| 10 | A unit with no art is a block you can click and target |
| 11 | The block's approved look |
| 13 | An infantry type costs a third of its geometry; bodies look as they did |
| 12 | An infantry team appears without a hitch |

## One owner per concept

| Concept | Owner | Deleted when it lands |
| --- | --- | --- |
| Content identity | The runtime catalog's hashes, carried by `ModelAsset` into Rust's `Library` | Identity by name |
| GPU resource lifetime | Shared handles; the last holder destroys | `Drop` for `Models` and `Cards` destroying outright |
| Material slots and texture layers | The art store | First-seen row order |
| Geometry and card reuse | The art store's indexes, keyed by content and pose | The per-admission geometry map; per-row card bakes |
| What an admission did | `AdmissionWork` | "No scene rebuild" as the scene's only assertion |
| Which art is wanted, and in what order | `useBattleKinds` and the art queue beside it | The menu-only prefetch gate |
| Requesting an admission | `SceneAppearances`, one candidate at a time | "Any install changed everything" |
| Facts derived from a bundle | One deriver (settled in 01) | The second derivation, if 01 finds they are the same facts |
| Stand-in size, drawing and picking | Presentation's body box | The silent skip as the visible outcome |

The end state reads as designed today: one admission path that takes a store,
which may be empty. No "incremental" path beside a "full" one, no compatibility
layer, no migration.

No switch, flag or default-off path anywhere: the game is zero-config. The
GPU preload this plan first allowed behind one was measured and cancelled.

## Standing gates

- **Count work, do not time it.** A slice that removes work proves it with
  `AdmissionWork` in a deterministic test. A long-frame check is a loose
  tripwire only. Numbers in this plan are tripwires, never targets.
- **Write the test first** where behaviour changes, and see it fail. Use
  [`write-tests`](../../.agents/skills/write-tests/SKILL.md). Renderer and
  presentation tests use synthetic bundles and stand-in units, never roster
  art. `battle-growth` uses the roster, so its assertions are relational (work
  equals the delta of staged names).
- **Pixels prove a rendering change is safe.** A reuse slice compares the
  committed frame with a clean build of the same catalog and expects equal
  pixels.
- **Visual slices** (10's fixture shots and all of 11) follow the visual rules
  in [AGENTS.md](../../AGENTS.md): `game-ui` first, `compare-screenshots`
  against the stated target, an unprimed `screenshot-critique` as the last
  check, then `preview-shots`.
- **Climb the check ladder only as far as the slice needs**
  ([development.md](../../development.md#check-ladder)). The commands live in
  the [renderer README](../../crates/renderer-core/README.md) and the
  [web README](../../web/README.md). Run everything once, at closeout.
- **Asset-backed tests and scenes need the runtime art** materialized from
  LFS first; the renderer README says how.
- **Docs move with the code.** A slice that changes a contract statement in an
  owning README changes that README.
- **Every slice keeps green:** the renderer and presentation packages, the
  WASM check, battle digests, the visual suite and the native client's build.

## Firewalls

Out of scope: shader or vertex-layout changes; a grow path for the native
client; spreading admission work over frames or moving it to a worker (see the
reslice triggers); per-size texture arrays; cancelling an in-flight build;
tuning for weak machines; any new visual beyond the stand-in; redesigning the
armory or roster pages, which share this path and only get faster.

Seen and not planned here, so they are not lost:

- The first scene's own build still joins vertex arrays byte by byte (0.62 s
  of the measured rebuild) and upscales small textures per pixel. Both can
  become block copies with no change to the picture, at any time.
- 57 of the 59 colour layers are upscaled to the largest texture's size. That
  is memory a per-size grouping would save.
- The art is held three times (parsed in JS, decoded in WASM, packed on the
  GPU). [One-renderer memory](../done/one-renderer/memory.md) records why two
  of them coexist.

## Reslice triggers

- **A multi-type arrival still visibly stutters after slice 08.** Add a slice
  that spreads admission work over frames in shared Rust.
- **A reuse key turns out to miss an input**, so a reused resource differs
  from a fresh one. Stop and widen the key in its owning slice.
- **The ledger keeps filling from one slice.** Split it.

## Open until measured

| Question | Unblocked by |
| --- | --- |
| Does scenery placement cost a frame on a large map? | Slice 06 times it first; slice 00 measured its bytes, not its time |
| Does a large arrival still stutter? | Frames of about 95 ms for an infantry team after slices 13 and 12; the rest is slice 14 and unattributed commit and bake frames |

## Background

- **WASM memory.** The build is 32-bit (`wasm32-unknown-unknown`), so the
  ceiling is 4 GB; the 16 GB figure belongs to 64-bit WASM. The whole catalog
  needs about 2.45 GB today (2.30 GB of geometry, 145 MB of textures stored
  once). The archived finding that loading the whole catalog exhausted WASM
  address space appears to predate texture sharing, when textures were repeated
  per bundle (2.22 GB more); that order is inferred from the archived notes,
  not re-run. A 64-bit build is not needed and not planned: Rust's wasm64 target
  needs a nightly toolchain and 64-bit code runs slower. Sources:
  [SpiderMonkey on Memory64](https://spidermonkey.dev/blog/2025/01/15/is-memory64-actually-worth-using.html),
  [the 16 GB cap](https://github.com/WebAssembly/spec/issues/1892).
- **Art sizes.** One faction's unit art is roughly 0.5–0.7 GB of geometry and
  200–260 MB to download, matched by name (slice 09 measures it from the
  roster). It is held three times: parsed in JS, decoded in WASM, packed on the
  GPU. All unit and wreck art uses 144 unique textures; the renderer requests
  the adapter's own limits, so the array-layer limit is not a constraint.
- **Precedents** for identity and lifetime, kept from the placeholder:
  [Unity Addressables](https://docs.unity3d.com/Packages/com.unity.addressables@1.21/manual/MemoryManagement.html)
  (reference-counted assets) and
  [Bevy asset handles](https://docs.rs/bevy_asset/0.20.1/bevy_asset/enum.Handle.html).
  Neither implements this atomic catalog swap.
- **Earlier evidence.** [One-renderer memory](../done/one-renderer/memory.md)
  owns the whole-browser memory method slice 09 reuses.
  [The mock](visualizations/first-contact-mock.html) is what the player picked
  the stand-in from.

# 09 Preload

**Question:** does preloading the enemy faction make first contact free of any
wait, and what does it cost in whole-browser memory?

**Depends on:** 08.

## What exists today

[`prefetchRuntimeAssets`](../../../apps/battle-lab/src/gameAppearances.ts)
warms the browser's cache with the whole catalog a few seconds after the first
page, without decoding or uploading. Only the menu enables it, so a battle
opened by its link never runs it. The art a battle wants is the set
`useBattleKinds` keeps: the fielded kinds, the player's own purchase roster,
and what the side has observed.

## Confirm first

- That the menu-only gate is why the prefetch was idle in the freeze report.
- One faction's exact art size, from the roster.

## Contract

From the first frame the battle presents, deployment included, the enemy
faction's types become ready one at a time. A type the side has just spotted
goes before the rest of the queue.

## Seam

- **The art queue:** a pure function beside `useBattleKinds` that, from what
  is spotted, what is to preload and what is loaded, names the next type or
  none. Order: spotted, then the enemy faction, then the rest of the download
  warm-up.
- The battle page enables the existing prefetch; the prefetch follows the
  queue's order. No second downloader.
- **Depth, by slice 08's measurement:**
  - If a one-type arrival showed no visible wait once its files were cached,
    the preload is download-only and this slice ends at ordering and enabling.
  - Otherwise the preload goes to the GPU by feeding enemy types one at a time
    into the set `useBattleKinds` owns. Each is an ordinary one-type growth.
- **The switch:** the deeper preload lands off by default. It is the one
  short-lived seam in this plan. When the player accepts the memory report it
  is removed and the preload is unconditional; if the player rejects it, the
  deeper preload code is deleted.

## Verification

Write first:

1. The queue on fake observations: order, no type twice, a spotted type first.
2. A new scene, `battle-preload`, asserting event order only: before any enemy
   is seen each admission adds one type; a type spotted mid-queue commits
   before the rest; first contact with a preloaded type stages nothing.

`battle-growth` runs with the preload off, so it keeps proving growth.

## Memory report

Measure with the method in
[one-renderer memory](../../done/one-renderer/memory.md): the whole Chromium
process tree, interleaved baseline and candidate runs, all variance kept, in
the reference battle. Report the engine alone against today (it must not raise
the peak) and the preload against the engine.

## Review checkpoint (non-blocking)

Show the report with
[`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md) and allow
about five minutes. If the player says nothing, leave the switch off, record
the numbers and the open decision in the README, close the shots and proceed.
Turning the preload on by default is the player's call.

## Delegated

The queue's data structure, the switch's name and home, how progress is shown
in diagnostics.

## Stays green

The menu's prefetch, `battle-growth`, the armory.

## Feedback that would change this slice

The player's answer to the memory report: on by default, off, or a shallower
depth.

# 09 Warm-up and large arrivals

**Question:** with no GPU preload, is a battle's art already in the browser's
cache when it is first needed, and does the largest arrival still hitch?

**Depends on:** 08a.

Rewritten 2026-11-15. This slice was the enemy-faction preload. It was built
to the GPU behind a switch, measured at 3.0 GB more settled memory, and
rejected by the player: the game is zero-config, so nothing ships off by
default, and a preload that is not on is not built. The
[ledger](../decisions.md) keeps the measurement. What is left is on for every
player.

## What exists today

[`prefetchRuntimeAssets`](../../../apps/battle-lab/src/gameAppearances.ts)
warms the browser's cache with the whole catalog a few seconds after the first
page, without decoding or uploading. Only the menu enables it, so a battle
opened by its link never runs it, and its order is the catalog's.

A battle admits everything its side learned since the last admission as one
generation. The reference battle's largest arrival (9 bundles, 27 cards) costs
one frame of 399 ms: 141 ms packing, 61 ms fielding, 47 ms decoding, 40 ms
rest poses.

## Contract

- After a battle's first presented frame, deployment included, the browser
  warms the remaining runtime files into its cache: the types the side has
  learned, then the public enemy faction's roster, then the rest. Warming
  decodes nothing, uploads nothing and adds no wanted kind. Cached art grants
  no unit visibility.
- An arrival's longest frame does not grow with the number of types learned
  together.

## Confirm first

Whether the 399 ms arrival is several types learned together or one type with
many bundles. The second contract line is built only for the first answer.

## Seam

- The battle page enables the existing prefetch, and the prefetch takes its
  order from the battle. No second downloader, no switch.
- If the large arrival is several types: learned types are admitted one per
  commit, the next when the previous has committed. The
  [stand-in](10-stand-in-mechanics.md) covers a type that waits. Kept only if
  the longest frame falls in the reference battle; the wait of the last type
  is reported.
- If it is one type: stop. Spreading one admission's work over frames is a
  different slice, decided on that breakdown.

**Fixed:** no setting, URL flag or default-off path. No GPU work for a type
the side has not learned.

## Verification

Write first:

1. A battle opened by its link starts the warm-up after its first presented
   frame, in the contract's order, and no admission happens because of it.
2. If pacing is built: types learned together commit one at a time, in the
   reference battle the longest arrival frame falls, and `battle-growth` keeps
   its gate with its tripwire unchanged.

## Stays green

The menu's prefetch, `battle-growth`, `stand-in`, the armory.

## Outcome

The warm-up is built and on for every skirmish. The large arrival is one type
(the RPG team: nine bundles, 27 cards), so pacing by type cannot divide it;
that became [slice 12](12-an-arrival-over-frames.md).

## Feedback that would change this slice

A paced arrival whose last type waits long enough to notice behind the
stand-in: then pacing is the wrong trade and the frame's work has to be spread
instead.

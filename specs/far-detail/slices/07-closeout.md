# 07: Closeout

## What this unlocks

The plan is finished, checked once as a whole, and recorded as rationale.

## Steps

1. Reconcile every row of [the decisions](../decisions.md) against the shipped
   code: each is implemented, or marked as needing no code.
2. Confirm no scratch hook, probe or temporary control reached the tree, and
   that both spikes' checkouts are gone.
3. Run the whole repository once, by
   [the check ladder](../../../development.md#check-ladder). Pull the LFS sets
   a full run needs first.
4. [`review`](../../../.agents/skills/review/SKILL.md) over the plan's whole
   diff, not the last slice.
5. Docs, as principles and pointers:
   - `crates/presentation/README.md`: the tier rule and what an appearance may
     carry past tier 3;
   - `packages/scene-assets/README.md`: the optional far mesh and its rules;
   - `crates/renderer-core/README.md` only if admission wording changed;
   - [`distant-impostors`](../../distant-impostors/README.md): point it here.
     A far mesh is not a card, and that plan's questions stay open.
6. Rewrite the ledger as the final record, then
   [`close-spec`](../../../.agents/skills/close-spec/SKILL.md) to
   `specs/done/far-detail/`, keeping both spike verdicts as rationale.

## What the user can see

One table: GPU time at the three holds before and after, which kinds shipped a
far mesh and which were dropped, and each spike's verdict with its reason.

## Must stay green

Everything. A failure seen here is fixed here, in its own commit when it is
unrelated.

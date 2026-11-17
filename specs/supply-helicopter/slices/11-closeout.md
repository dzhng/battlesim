# 11: Closeout

**Contract.** The feature is whole and the spec is archived.

**Steps.**
1. Pull the LFS sets a full run needs: runtime and third-party audio, roster and
   infantry sources, scene baselines and city.
2. Run the whole check ladder once (`development.md`). Fix every failure,
   including ones this feature didn't cause, each in its own commit.
3. Reconcile every row in [`decisions.md`](../decisions.md) against the shipped
   code. Each is either implemented or marked "no code needed".
4. Run `review` over the whole spec's diff.
5. Rewrite `choices.md` from scratch as the final ledger.
6. Archive with `close-spec` into `specs/done/supply-helicopter/`.
7. Add a pointer from `specs/done/helicopters/README.md` noting that a helicopter
   can now land (the "aircraft is a hull with no ground footprint" invariant now
   reads "while airborne"). Update D17, D21, D27 and D37 references where they
   state what this feature changed.

**Verify.** The full run is green. The supply, transport, air-hover and supply-ai
scenes all pass. The handoff shows nothing left to pick up.

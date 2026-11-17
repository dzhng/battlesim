# 01: The AI's standing supplier drives

**Contract.** The skirmish AI's opening supply buy, its truck replacement, and
its truck-parking loop consider only supply units that move on the ground (D11).
Today no flying supplier exists, so this changes no outcome. It is the guard
that must be in place before slice 10 puts flying suppliers in the decks.

**Seam.** `crates/sim/src/skirmish_ai/`:
- One predicate beside `pick` in `purchase.rs`: `drives(rules, kind) -> bool`,
  true when the type's mobility is a ground mover. It reads mobility, never id,
  label or role.
- Callers:
  - the opening recipes' `("logistics", Category::Sup, 0)` picks (`purchase.rs`, ~288-322);
  - the "no logistics unit yet" truck buy in `reinforcement` (~402-412). Its
    "already have one" check counts only drivers, so an emergency heli never
    stands in for the truck;
  - the support-spot loop in `mod.rs` (~870-935) moves and deploys only drivers.
- Slice 09 adds the flying supplier's own orders; nothing here references it.

**Run / see.** Nothing new in a game. The proof is the tests.

**Verify (test first).** In `crates/sim/src/skirmish_ai/tests/purchases.rs`,
patch the test deck with a flying supplier (a `test_heli`-based fixture row with
`deploy` and `supply`, carded `sup`, at the truck's cost or cheaper, so both
the value rank and the id tie-break would pick it):
- `the_opening_buys_the_supplier_that_drives_even_when_a_flyer_ties`
- `a_lost_truck_is_replaced_by_one_that_drives`
- `the_support_loop_never_moves_a_flyer`

Each asserts the bought card's mobility, not its id. Existing assertions at
`skirmish_ai/tests.rs` (~297, ~371) that check only `has_role("logistics")`
are tightened to check that the supplier drives.

If slice 02's `test_supply_heli` already exists, use it. Otherwise patch the
deck in the test (`sim::fixtures::patch_catalog`), and don't add the row here.

**Digest.** None. Every skirmish replay and the skirmish corpus stay
byte-identical; that is the proof the filter is inert today.

**Delegated.** The predicate's name and exact home; the deck-patch helper.

**Stays green.** All `skirmish_ai` tests, `crates/sim/tests/skirmish*.rs`.

**Feedback that would change it.** None expected; it's a guard.

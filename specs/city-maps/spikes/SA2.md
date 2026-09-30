# SA2 navigation: sparse residency proved; search work remains red

**Verdict:** the sparse storage correction preserves the frozen safe route and village
corpus and admits the recorded fixed-extent open/sparse routes. It does not unlock full SA2.
A 4 km bridge still explores millions of cells and spends seconds per route. No full-size
bridge arm was run simply to repeat that failure.

The seam is unchanged: side-known geometry, mover footprint/push/policy and temporary
avoidance feed NavGrid's fits/snap/plan/route-time/route-fit contracts. Flat cells are
implicit, exact capped clearance and visited scratch own local sample tiles, and the
original route schedule/cost arithmetic remains authoritative. Navigation diagnostics
expose residency and expansion counts without changing digests or publication.

[Evidence and allocation contract](../assets/navigation-proof/README.md) record safe dense
oracles, the 1,800-tick original village trace, serial resource rows, failed candidates and
remaining conditional capacities. Field residency is sparse, not a universal dense-catalog
or search-work bound. Native/wasm full opposing-edge movement and full bridge/road/occupied
catalog admission remain pending.

The failed owner is resliced as [exact search work](../slices/SA2-exact-search-work.md).
A route becoming visible or movement beginning on a different tick would be a named
behavior decision; deferred route jobs cannot be used as an unlabelled parity fix.

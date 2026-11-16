# 01a · Spread entries

**Unlocks:** several squads sent into one house in one order head for
different approach points, so their rings and order lines don't stack.

Found by the unprimed critique of slice 01's preview shot: squads on one side
of a house all take `garrison::approach`'s nearest point, so their rings pile
up half under the house and their lines chain through each other.

## Seam

`garrison::occupy` (`crates/sim/src/garrison/occupy.rs`). The first squad in
the order (in the order's stable unit order) keeps exactly today's approach.
Each later squad takes the nearest approach point that is at least a squad's
spacing away from the approaches already given in this order, using the same
side-known geometry and reach as `garrison::approach`. If no such point
exists, it takes the nearest point as today.

## Verification

- A single-squad order is unchanged: `UNCHANGED_SINGLE_HOSTS` and
  `UNCHANGED_OTHER_SCENES` identical (the defender sends one squad per order).
- Extend the slice 01 test in `crates/sim/tests/shared_hosts.rs` that checks
  every listed squad walks to the house: their approaches are pairwise apart.
- The three-squad preview shot: `compare-screenshots` against slice 01's shot
  in `throwaway/shared-hosts/shots/`, then an unprimed `screenshot-critique`
  as the last check.

## Delegated

How candidate points are sampled along the side-known exposed facades.

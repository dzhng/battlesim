# Exact search pruning candidate

This proof keeps the original Euclidean queue ordering, strict parent improvements,
step order, grid costs, immediate planning tick and smoothing. A cheap feasible walk
supplies only a cost ceiling; its cells never become returned route parents. The frozen
dense planner and complete route/village records remain the oracle. **The 4 km resource
gate is failed:** the small bridge work guard does not establish full-size admission.

The stencil certificate also reuses implicit fit, crossing and cell-cost answers when
all nine cells and the capped-clearance reach lie outside conservative source regions
and temporary avoidance. Map-edge clearance is checked for the whole stencil; the
avoidance reach includes both inflated-box axes and diagonal neighbour reach. The cost
owner computes the same two cell costs and the same half-sum arithmetic, including
nonfinite results. This adds no retained cache or scheduling state.

The optional single-segment certificate also declines invalid speed/cost assumptions
and a potentially overflowing path ceiling. The frozen planner deliberately initializes
first-visit costs even when they are Inf/NaN; that behavior remains. In particular NaN
speed on the small raw-input oracle follows the original three-point route instead of
being replaced with the goal alone. This is fallback parity, not new validation.

## Bound geometry

The metric is the grid's octile metric: orthogonal length 2 m and diagonal length
`2 * SQRT_2`. Its triangle inequality supplies a lower bound on any grid walk. Every
walk crossing a selected row/column visits a cell that fits this search's footprint and
temporary avoidance. Taking the minimum distance through all such opening cells is
therefore another lower bound. These bounds never become queue priorities.

For a contiguous opening the sum of its two octile distances is convex in the free
coordinate. Between the endpoint coordinates its slope changes at each endpoint's
coordinate plus/minus the fixed-axis distance to the cut. Clamp the lower end of the
minimum plateau to each opening interval. This replaces a scan of every opening cell
with one distance evaluation per interval; its floating-point expression retains the
same bound allowance.

Let `q` be the implicit flat cell's positive cost per metre and `q_min` the least cost
among implicit and entered exceptional cells. Each improving parent chain is simple:
nonnegative steps cannot strictly improve an ancestor through its descendant. Each cell
then owns at most two half-diagonal incident costs. Discounting every exceptional faster
cell by `2 * SQRT_2 * max(q - q_cell, 0)` is conservative even when the route never visits
most of them. Together with the global `distance * q_min`, this bounds weighted costs
without assuming that roads, forests, slopes or shove costs equal flat ground.

An independent geometric discount covers **all** faster cells with one axis-aligned
rectangle, expanded from extreme cell centres by half a cell on every side. Every
incident half-edge whose endpoint cost is faster lies inside this rectangle. Relax its
whole interior to `q_min` and ignore obstacles. In the continuous octile metric, a
minimum-cost relaxed path can visit the convex rectangle once: replace an outside
excursion between two boundary points with the cheaper interior chord. The saving
relative to uniform `q` is at most its **octile** diameter times `(q - q_min)`, not its
Euclidean diameter. Taking the smaller of the geometric and all-cell discounts preserves
conservatism. A mandatory opening splits the walk into two legs; each leg may benefit
from the rectangle, so that bound allows **two** diameters. Counting one would overstate
the lower bound when the visit prevents replacing the whole excursion with one chord.

## Floating point and winner retention

Use the actual f64 `cost(cell, ..., 1)` as the reference node weight, not reconstructed
rule arithmetic. The cost owner still computes each candidate and A* edge with the same
orthogonal/diagonal divisions and half-sum operation order. No multiplication replaces
path accumulation. With positive normal finite node weights, an original improving
parent chain has at most `N` cells and a candidate's three monotone legs have at most
`3N` steps, where `N` is the grid cell count.

For unit roundoff `u = EPSILON/2`, the usual positive-sum bound is
`gamma_k = k*u/(1-k*u)`. Relative to these reference weights, reserve eight rounding
errors per step for the reference/length cost divisions and positive half-sum; path
addition contributes one per step. Positive combinations use the greatest operand
relative error, rather than accumulating both operand errors as a cancellation bound.
The octile/cut expressions have a constant number of arithmetic errors per leg. The
all-cell discount has at most `N` positive additions and a constant number of operations
per term. The source coordinate centres and differences are exact integer multiples of
2 m within the admitted small-error grid regime. The old Euclidean heuristic is checked
against the least node weight with an additional small arithmetic tolerance; its rounded
estimate can exceed a true completion cost only within this allowance.

The conservative error accounting, in units of `u`, is:

| Contribution | Allowance |
|---|---:|
| Three-leg candidate ceiling, at most `3N` steps with eight edge errors plus addition | `27N` |
| Original frontier prefix arithmetic relative to node weights | `9N` |
| Original heuristic quotient, Euclidean norm and permitted unit-weight comparison error | `32N` |
| Winning suffix arithmetic relative to the same node weights | `9N` |
| Discount terms/sum, octile and cut arithmetic, rectangle bound | `24N` |
| Final bound, ceiling and comparison arithmetic | `8N` |

The sum is `109N`; use `gamma_(128N)` for composition rather than adding first-order
errors as if they were exact. The regime check makes `128N*u < 0.0005`, so this gamma is
less than `128.065N*u`. A normal final node weight can still contain a subnormal first
division before shove scaling. The smallest admitted shove share is 1/4; the orthogonal/
diagonal length is at most `2*SQRT_2`. Comparing a length cost to its reference unit cost
therefore adds fewer than eight least-positive-subnormal units per step in the worst
underflow case, beyond the relative allowance. Across the three-leg candidate, original
prefix and winning suffix this is at most `40N` units. The heuristic quotient, discount
products and final arithmetic fit within another `24N`. Reserve **`64N`**
least-positive-subnormal units in total. For a
nonempty admitted route the positive operand sum below is at least twice the smallest
normal node weight, so the unused gap to `256N*u` also exceeds this absolute allowance
(`u * smallest_normal = smallest_subnormal/2`).
The start-equals-target case pops its goal before any rejection. Overflow in the operand
sum gives an infinite right side and cannot affirm pruning.

The comparison subtracts no unaccounted negative sum. It reserves
`128 * (N+1) * EPSILON * (g + unsubtracted_bound + discount + feasible_upper)` for path,
bound, discount and old goal-pop arithmetic together. This coefficient exceeds the
combined gamma and absolute allowances above, including the three candidate legs and the original
heuristic's goal-pop ceiling, in the admitted regime where the coefficient is below
0.001. The scaling includes operands before discount cancellation. Overflow in a
comparison cannot affirm pruning. Nonpositive, subnormal or nonfinite cell weights,
invalid original heuristic assumptions, dimension overflow or a larger error regime
disable this optional proof and leave the original exhaustive planner active.

Any feasible path leaves a frontier cell with priority no greater than its cost plus
this allowance until the old planner pops the goal. Thus the original winning goal cost
cannot exceed the feasible ceiling beyond the allowance. A strict lower-bound rejection
cannot belong to that winning parent chain, including equal-cost alternatives. The
remaining queue comparisons and strict parent updates retain their order. A rejected
cell may be reconsidered with a genuinely lower cost; no resource cap returns NoRoute.

## Measured gate and remaining cost

All eight complete 4 km bridge routes match the selected original route strings, including
the red vehicle's Shortest intermediate `(1451,829)`. The first candidate reduced eight-route
instructions from 150.27 billion to 17.75 billion but took 92–149 ms per route. Analytical
opening evaluation plus the implicit stencil reduced this to 7.69 billion instructions
and 31.5–46.8 ms per route. Requested peak heap is 31.43 MB, including the unchanged 18.4 MB
world owner; the preceding candidate was 116.52 MB. The measured source, native binary and
evidence are pinned by `identity.json`; its `candidate.patch` is relative to the recorded
base. That patch precedes the raw-input fallback guard and clone cleanup; the retained
source has a separate identity. The first arm is labelled exploratory rather than given
the final measured candidate's identity.
Both source patches use zero context and require `git apply --unidiff-zero`; their
reconstructed source byte hashes are verified separately from the patch encoding.

The final arm still exceeds the 33 ms maximum and cannot meet the 16 ms p95 target. No
repeat distribution or full-size bridge arm was admitted after that failure. Source/oracle
parity makes this a separately reviewable pruning/lookup improvement, not a full SA2 pass.

Cost bounds cannot reject a cell that lies on an equal-cost completion. Broad uniform
rectangles contain many permutations of orthogonal and diagonal steps; the original
Euclidean queue can expand an area of such cells before its goal. A passed corpus proves
observed parity; it does not remove this potential quadratic work or replace the bound
argument. The first resource gate is the safe 4 km bridge: p95 first route at most 16 ms,
no completed-tick route work above 33 ms, with grid construction reported separately.
Only a passed gate admits full extents. Async tick scheduling and route-changing priority
are outside this proof.

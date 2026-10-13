# 05 — Arbitrary dogfight state

**Contract:** any number of aircraft can participate in a dogfight group. A
right-click air order or mutual automatic fire can create engagement. Every
aircraft has an unlimited gun; AA missiles acquire only inside their authored
forward FOV and self-guide when their row says fire-and-forget. An aircraft
under automatic engagement dodges until a new explicit order. Any explicit move,
attack, ground attack, or Stop breaks its dogfight; range loss breaks it too.
Attack-move keeps its normal policy. Groups beside one another do not merge;
ordinary target selection lets a surviving aircraft join another later.

**Seam:** new `sim::air_combat` state, command validation, target selection,
`units::AirState`, and observation dogfight membership. Stable IDs and sorted
target choices make group behavior deterministic.

**Runnable proof:** `/lab/dogfight` runs 1v1, 2vN, crossing attack-move waves,
new-aircraft join, explicit-order break, and range break.

**Checks:** arbitrary membership, stable target assignment, FOV, gun/missile
priority, dodge/order transitions, no hidden truth, and digest/replay parity.

**Visual gate:** group lock/readability crop with the standard UI and screenshot
gates; dogfight labels must not imply a merged group.

**Scope firewall:** no radar/SEAD, fuel, ingress, ground attack, or roster art.

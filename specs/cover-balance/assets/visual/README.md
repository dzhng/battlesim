# Cover trajectory evidence

These project-owned captures judge whether weaker scatter introduces an obvious
new flight or impact-placement regression. Percentages come from the native
paired-fire report, never from these pictures.

[Baseline fixture](frozen-village.json), production `/battle/village/lean`
renderer, village seed, 1920×1080 DPR1. Camera target `[874,962,0]`, distance 28 m,
pitch 0.25 radians, yaw 0. Rifle at tick 331; HMG at tick 341. Both arms hide
panels to expose the ground. Rifle slots exclude grenade launchers; HMG replaces
only the opposing rifle squad with the existing jeep. Subjects remain ordinary
infantry in the wood-edge firefight. Only the three exterior spread factors
change between arms; building spread, weapon rules, renderer and geometry stay
frozen. The full paired set checked six instants for each weapon.

| | Baseline | Tuned | Local comparison, baseline left |
| --- | --- | --- | --- |
| Rifle | [native](rifle-before.png) | [native](rifle-after.png) | [crop](rifle-comparison.png) |
| HMG | [native](hmg-before.png) | [native](hmg-after.png) | [crop](hmg-comparison.png) |

Every matched native pair differs, and camera values agree exactly. Local image
distance is 0.00216 for the rifle crop and 0.00759 for the HMG crop; these are
change diagnostics, not quality scores. An independent unprimed critic inspected
all native frames before all enlarged crops. It found no conclusive new
impossible trajectory, aiming-height or ground-placement regression. Preview
showed the comparison for about five minutes and then closed.

Acceptance is limited to trajectory plausibility under this tuning. Existing
HMG dust can grow to soldier height, cyan occlusion silhouettes fragment across
trunks, some trunk-hit puffs have ambiguous height, and long thin tracers can
read as wires. These occur in both arms; this pass does not certify their
presentation quality. Sparse stills and an offscreen firing source cannot prove
flight continuity or lifetime. These images are reference evidence and never
runtime assets.

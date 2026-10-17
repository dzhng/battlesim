# Slice 02 — shared spawn groups and size admission

Generate one to three edge-road spawn arms per side, with finite geometry, spacing, fairness, and reachability admission. Slots reference shared groups. Ground uses the admitted road entry; aircraft use `air::edge` from that entry. Enforce size floors: 1 → Small, 2 → Medium, 3–4 → Large, 5 → XL.

Write red contract/mapgen/air/preparation tests first. Prove every slot is represented and every entry is legal.

# 04 — Radar signatures and SEAD contacts

**Contract:** radar-capable ground AA always emits an unobstructed 360° radar
signature. Only SEAD aircraft detect signatures. Detection is a very long-range
radar-only radius and ignores terrain, trees, buildings, and Ground fog. It
publishes a separate radar contact: an approximate area circle plus bearing and
range, labelled “RADAR CONTACT”; it never reveals Ground fog, exact identity,
or exact position. SEAD weapons may target only a visible radar emitter or a
radar contact.

**Seam:** `sim::sensing`, `knowledge`, `contract::observation`, publication
codec, and target validation. Radar contact uncertainty is generated once in
the simulation and is never reconstructed in TypeScript.

**Runnable proof:** `/lab/air-radar` places emitting AA in clearing, forest,
and behind terrain, with an ordinary fighter and a SEAD aircraft. Ground AA
still needs physical LOS to fire.

**Checks:** 360° emission, SEAD-only detection, quantized area/bearing/range,
expiry/refresh, no hidden-truth targeting, side isolation, and replay parity.

**Visual gate:** contact circle and “RADAR CONTACT” text crop; `game-ui`,
`compare-screenshots` where applicable, unprimed critique last, then
`preview-shots`.

**Scope firewall:** no radar obstruction, radar shutdown, electronic jamming,
or UI spectacle beyond readable contact language.

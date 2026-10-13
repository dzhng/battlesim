# 11 — Jet/alarm audio and combined-air closeout

**Contract:** the first aircraft entering a side with no aircraft on field emits
one air alarm. Jet engine sound grows louder as the aircraft approaches;
missile, flare, AA, and radar/SEAD cues come from observed causes, never unit
labels or hidden truth. Use the slice-00 YouTube receipts through
battle-audio's source→runtime workflow. Then close the feature with a fixed
combined battle containing all AA tiers, HighAir/LowAir, radar/SEAD, ingress,
dogfights, ground attacks, fuel retreats, cards, and audio.

**Seam:** simulation/presentation sound causes, `packages/battle-audio`,
`fixtures/sounds.json`, and the existing battle authority/worker.

**Runnable proof:** `/lab/air-audio` plus `/battle/air-combined`.

**Checks:** one alarm per side/episode, distance attenuation, side-local
hearing, deterministic sound frame, full replay/digest parity, balance sample
for SEAD saturation/interception and hit-count intent, then the full repository
`check` and browser `verify` once at closeout.

**Visual/audio gate:** compare approved targets when present, run unprimed
`screenshot-critique` last for every visual capture, and offer `preview-shots`.

**Scope firewall:** no campaign, multiplayer, deck-building, drone mechanics,
transport cargo, or finished cinematics.

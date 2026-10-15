# Air-defence radar networks (placeholder)

**Status:** deferred by the user (2026-10-15): it would complicate the game too much for now. Don't start it unless the user asks.
**Updated:** 2026-10-15.

## Next Agent Prompt

You are picking up an idea the user liked during [air-war flight](../air-war-flight/README.md): air-defence launchers that can't see far on their own, and radar units that find targets for them. When the user starts this:
1. walk the open questions below with them (`explore-unknowns`);
2. turn this page into a sliced plan (`write-spec`).

Load `tweak-mechanics` before changing any rule. Update this section before you end each pass.

## The idea

A real SAM battery is several vehicles. Its launchers (NASAMS, Patriot, IRIS-T SLM, the S-300 family) carry no search radar of their own. A separate radar vehicle finds the targets, for example Sentinel, Patriot's radar or TRML-4D. Kill the radar and the launchers go blind. That gives the air war a loop:
- the defender protects and hides the radar;
- the attacker's SEAD jets hunt it by its emissions;
- recon and shared sight fill the gap while it's down.

Most of it is data, because the sim already shares every sighting across the side:
- **launchers** get short air sight;
- **a radar unit** gets long high-air sight and emits, so SEAD jets can find it.

## What the sim does today (2026-10-15)

- **Seeing aircraft** is per-layer sight on each unit type (`sensors.low_air_m` and `sensors.high_air_m`; `crates/sim/src/sight.rs`, `of_layer`). Air-defence rows carry long air sight: short-range AA about 900 m, medium 3,200 m, long-range 7,000 m against high air. Every other unit sees aircraft at its ground sight range, in 3D.
- **Emitting** (`capabilities.radar.emits`) only makes a ground unit findable by SEAD aircraft. Their own `radar.detection_m` decides the reach (`crates/sim/src/knowledge.rs`, `update_radar`). Medium and long-range AA emit. In the short tier only the Gepard emits: it gained its search radar on 2026-10-15.
- **Sightings are shared by the side**, so any unit may fire at a target another unit sees, within its weapon's reach.

## Open questions

- Every faction, or one faction's flavour? The user floated it for Europe. The physical rule applies to every modern battery, so a faction difference would come from which radar units each side fields.
- Which radar vehicles per faction, and their prices?
- Should launchers keep some sight of their own (the missile seeker, an optical sight), or be blind without a radar?
- How does the skirmish AI buy a battery: one radar plus launchers? The AI follows the same rules as a player.
- Does the Gepard, a short-range gun with its own short search radar, stay self-sufficient?

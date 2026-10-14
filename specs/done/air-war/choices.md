# Air-war choices

Decisions are recorded against the current shipped implementation. Behavioral
evidence is called out where a focused test exists; balance and a few player
interaction paths remain provisional or unverified. Entries are ordered from
least certain (future tuning) to firm architectural constraints.

| Verdict | Decision | Why it matters |
|---|---|---|
| Provisional | Hit-count balance targets 1–3 long, 3–4 medium, and 4–6 short missile hits, with ballistic AA relying on spread. | These are provisional fixture targets; no balance report or equivalent proof is shipped yet. |
| Sound | The alarm, AA launch and missile-warning recordings are admitted as personal-project sources; the jet engine and flare burst are generated sound effects, since the downloaded flyby and flare clips carried crowd voices and were deleted. | The five air cues stay editable in the catalog and use the same reviewed playback path as ground combat audio. |
| Sound | HighAir uses the existing generic air contact bark. | Adding a new voice taxonomy would couple identity to the altitude-layer cutover. |
| Sound | The first visible HighAir episode raises one side-local air alarm; approach audio is a positional HighAir loop. | Aircraft should announce their arrival without leaking hidden units. |
| Sound | Missile, flare and AA sounds remain cause-driven through presentation observations. | Audio cannot invent hidden combat events from unit labels. |
| Simulation | Aircraft are persistent on-map HighAir units; helicopters remain LowAir. | Circling, ingress, dogfights and fuel all require one continuous lifecycle. |
| Simulation | HighAir aircraft never reveal Ground or clear Ground fog, but can sense LowAir. | Air reconnaissance stays separated from ground reconnaissance. |
| Simulation | Radar is always-emitting, 360° and unobstructed; only SEAD aircraft detect it. | A simple separate radar channel creates the intended SEAD-versus-SAM game. |
| Simulation | Radar contacts publish an approximate area with quantized bearing/range and an explicit radar label. | Players get actionable direction without exact identity or ground-fog reveal. |
| Simulation | Every faction has short, medium and long AA plus a dedicated SEAD aircraft. | No faction is structurally missing a counter or an anti-radar role. |
| Simulation | Short ballistic AA can engage Ground, LowAir, HighAir and interceptable missiles; short guided Stinger/Igla rows cannot intercept. | The ballistic gun owns the close defensive role while shoulder missiles stay realistic. |
| Simulation | Medium interception is false by default and authored per weapon; long AA carries a small finite store and slow reload. | Interception is a weapon property, not a platform-name exception. |
| Simulation | SEAD missiles have the longest aircraft range and may target only visible radar emitters or radar contacts. | The first exchange is a radar-contact launch followed by anti-missile pressure and reload exposure. |
| Simulation | Seekers use authored FOV, turn rate, lifetime and decoyability; flares are finite, automatic and physical decoys. | Misses emerge from seeker geometry and fuel, while nearby splash still damages the aircraft. |
| Simulation | Ground AA fire requires terrain/prop/foliage LOS. | Concealment in a forest is useful, but foliage never becomes a free firing lane. |
| Simulation | Automatic air targeting ranks legal range then value, prioritising air when an aircraft carries AA missiles. | Aircraft behave like self-directed combatants without overriding explicit orders. |
| Simulation (implemented; behavioral test pending) | Dogfights are pair links that can form among any number of aircraft; explicit move/attack or range loss breaks a link, and attack-move preserves it. | Emergent joining avoids a special group state while keeping player orders authoritative. |
| Simulation (implemented; behavioral test pending) | Afterburner is an explicit persistent toggle: 1.5× speed, 2× fuel use and worse ground-attack accuracy. | The player trades endurance and bombing precision for missile-evasion speed. |
| Simulation | Fuel starts on map entry; near-empty aircraft auto-return and settle a health/ammunition/fuel-weighted refund. | Off-map traversal is visible but does not consume combat endurance. |
| Limitation | A SAM's launcher turns in the simulation but not on screen: its appearance declares the hull draws it (`"mounts": { "launcher": "hull" }`), and its missiles leave from the published muzzle. | The Buk, Pantsir, Skyranger, NASAMS and M-SHORAD models have no turret rig. A fixed launcher in the simulation would hold fire on wheels (D8), so the art bends, not the rule, until the models get a rig. |
| Simulation | Every aircraft and SAM row takes its art's whole box as its hull, wings included; `hull_limits.air` is 7.4 × 11.7 m, covering the Su-34. | A round that hits a wing hits the aircraft. The old 3.65 × 9.94 m limit was sized for the Apache's stores before fixed-wing rows existed; the air grid's wall margin grows with it. |
| Simulation | The generic short, medium and long AA rows drawn on an IFV's art take that IFV's hull and fire from its turret (their mount id is the IFV's). | A wreck is its unit's size, and the IFV's gun rig draws the launcher where the row puts it. |
| Presentation | Aircraft have no ground circles; compact cards attach to HighAir ghosts while Space shows orders. | Fixed-wing units read as aircraft rather than helicopters or ground movers. |
| Contract | The cutover is hard: no sortie compatibility reader, migration or parallel lifecycle. | Simulation, publication and browser consumers share one current contract. |

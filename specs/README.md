# Plans and rationale

[Feature directories](./) hold active plans. Each plan's README owns its current
handoff, verification checkpoints and unresolved work; its choices ledger records
decisions made where the plan was silent. Read that handoff before implementing a
slice. A planning document can describe future behavior, so it is not proof that
a capability has shipped.

[Unit roster and skirmish requirements](done/unit-roster/README.md) capture factions, unit
variants, the skirmish economy/objectives, basic AI and a phased implementation
ladder with parallel named-model production.

[Battle foundation](done/battle-foundation/README.md) (closed) records the village
checkpoint and the decisions that established the simulation, observation, combat,
and recovery seams. Its later umbrella work was superseded by the dedicated feature
records below and the [team skirmish](done/team-skirmish/README.md) record.

[Helicopters](done/helicopters/README.md) (shipped) made the roster's 19 helicopters playable in a low-air layer. [Transport](done/transport/README.md) (shipped) carries infantry squads in APCs, IFVs, light vehicles and helicopters, with bail-out when a carrier dies and a skirmish AI that moves its infantry by transport.

[Team skirmish](done/team-skirmish/README.md) (shipped) turned the 1v1 skirmish into team play from 1v1 to 5v5: players with their own wallets, armies and controllers, shared entries per side, a coordinating team AI, and an order-of-battle lobby over a real generated map.

[Persistent aircraft and layered air defence](done/air-war/README.md) (shipped)
made fixed-wing aircraft, radar, interception and SEAD one persistent battlefield
contract. [Air-war flight](done/air-war-flight/README.md) (shipped) made jets fly as jets:
their own altitude, circling, attack runs, organic dogfights, edge arrival and
departure, and every planned jet enabled with a reviewed model. [Air-defence radar networks](done/air-defence-radar/README.md) is a closed, deferred idea:
separate radar vehicles supplying otherwise-blind launchers.

[Battle voices and music](done/battle-voices-music/README.md) (shipped) gave units RTS-style voice lines (answers to their owner and simulation-emitted barks) and battles an adaptive background score, generated with ElevenLabs and reviewed in the sound workbench.

[Native rendering parity](done/native-rendering-parity/README.md) (shipped) built a
native `wgpu` client that draws what the browser draws, over one Rust
presentation crate both clients share, and holds the presentation API's contract.
[One renderer](one-renderer/README.md) is a placeholder for the open decision on
whether the browser moves onto native's Rust `wgpu` renderer, so the game has one
renderer instead of two. [Native gaps](native-gaps/README.md) is a placeholder
for the known limitations native shipped with.

[Launcher follow-ups](launcher-followups/README.md) is a placeholder for what
was left when the five long and medium-range air-defence launchers got their own
models: their armour, stale model receipts and shared truck parts.

[Capture](done/capture/README.md) (shipped) turns a JSON capture file (camera and scenes over saved encounters) into MP4s for marketing videos, through the same scene player the main-menu backdrop uses.

[Performance](done/performance/README.md) records shared spatial selection and
local shadow fitting across three gameplay zoom levels. It retains the measured
bounded-work improvement, GPU/memory costs, and rejected distant-car pilot; no
net fixed-view GPU speedup is claimed.

[Distant detail](done/distant-detail/README.md) records the shipped tiny-prop
shadow cutoff and the measured, rejected parked-car card experiment.
[Distant impostors](distant-impostors/README.md) is a placeholder for a future
far-zoom 2D-impostor investigation.

[Finished features](done/) retain the rationale and accepted scope after shipping.
Their README explains why the design works and points to current owners. Choices,
measurements, references and rejected experiments beneath each feature retain their
original workload and date context; frozen numbers are evidence, not today's defaults.
Production commands and settings belong to their manifests, registries and source.

Historical source artifacts may live at the Git revision or tag named by their
receipt rather than in the current checkout. Follow that provenance explicitly;
do not substitute a current script for a frozen comparison baseline.

# Sound workbench

Run `bun run dev` from the repository root, then open `/sound-workbench` at the
localhost address printed by Vite, or use the developer menu. The root
[startup instructions](../../README.md#running-it) own prerequisites and setup.
The library includes clean recordings, unused mechanical actions and the original
synthesis baselines. Audition is explicit; opening or testing the tool stays silent.

Recipes select alternative clips and blend optional synthesis into each alternative.
A recipe's alternatives play the same number of shots: burst clips with burst clips
at one interval, single reports with single reports.
The [production sound bank](../../packages/battle-audio/src/soundBank.ts) owns
decoding and mixing for audition and battle playback. A baseline retains its original
meaning; clone it to create an editable recipe. Source provenance and cleaned media
belong to the audio asset workflow, and cannot be rewritten through this editor.
Clip and recipe auditions use the bank's [shared source calibration](../../packages/battle-audio/README.md).
Recipe and assignment gains remain the authoring controls for intentional level differences.
Audition centers mono material in stereo and uses a memoryless safety ceiling;
safe brief attacks are not attenuated by a newly created compressor's startup.

Firing choices name a weapon row from `game.json`. A row without its own choice
shows and plays its nearest ancestor's, else the default row's; restoring a row's
own choice exposes that fallback, including its effect replacement. Movement is
auditioned read-only: each vehicle class drives at a chosen speed, turret traverse
and reverse, its loops mixed by the battle's own law, and a squad's footsteps fall
once a stride at a chosen pace. Their levels and curves belong to the game's
presentation; a loop or the footstep is replaced for every class through its effect
slot. Near and far may select different
recordings or recipes sharing the same core; the battle applies distance attenuation,
filtering, attack and reverb during playback. Workbench auditions play the recipe
without battlefield distance processing. A cannon's launch is heard as its mount's first weapon row across its ammunition
types; the observed launch contract does not identify the selected cannon round.
Impact choices may distinguish the published round and material; the ricochet row
chooses each round's glance the same way. Stored reloads and
mechanical actions remain useful auditions without inventing battle events.

A draft changes only the workbench. Preview captures and validates the exact source
generation and displays complete before/after JSON. Save publishes that reviewed
server-held candidate through [shared fixture publication](../fixture-publication/README.md),
which owns stale-save rejection, byte-preserving no-ops and outside-edit preservation.
An error keeps the draft available; discard and reload is explicit.

Saving changes the repository sound document under the shared accepted-generation
contract. Sound catalog hot reload is suppressed so it cannot replace a running
battle's choices; reload or open a page to capture the latest saved generation. The development-only HTTP boundary accepts local same-origin writes, logical documents and reviewed candidate
identities; the browser cannot choose publication paths.

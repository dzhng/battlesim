# Sound workbench

Open `/sound-workbench` from the developer menu while the development server runs.
The library includes clean recordings, unused mechanical actions and the original
synthesis baselines. Audition is explicit; opening or testing the tool stays silent.

Recipes select alternative clips and blend optional synthesis into each alternative.
The [production sound bank](../../packages/battle-audio/src/soundBank.ts) owns
decoding and mixing for audition and battle playback. A baseline retains its original
meaning; clone it to create an editable recipe. Source provenance and cleaned media
belong to the audio asset workflow, and cannot be rewritten through this editor.
Clip and recipe auditions use the bank's shared source loudness calibration.
Recorded file levels do not decide weapon balance; recipe and assignment gains do.
Audition centers mono material in stereo and uses a memoryless safety ceiling;
safe brief attacks are not attenuated by a newly created compressor's startup.

Assignments name an exact unit type and authored mount. Types are derived from the
resolved unit catalog, and repeated physical mounts with the same authored name
share a choice. Restoring an override exposes the global firing default or implicit
fallback, including its effect replacement. Near and far may select different
recordings or recipes sharing the same core; the battle applies distance attenuation,
filtering, attack and reverb during playback. Workbench auditions play the recipe
without battlefield distance processing. Cannon launch choices follow the mount across its ammunition types;
the existing observed launch contract does not identify the selected cannon round.
Impact choices may distinguish the published round and material. Stored reloads and
mechanical actions remain useful auditions without inventing battle events.

A draft changes only the workbench. Preview captures and validates the exact source
generation and displays complete before/after JSON. Save publishes that reviewed
server-held candidate through the shared [fixture publication owner](../fixture-publication/publication.ts).
No-op saves preserve source bytes. Changes made elsewhere reject stale publication
and survive rollback; an error keeps the draft available. Discard and reload is explicit.

Saving changes the repository sound document. Running battles retain their captured
generation; reloading or opening a battle page captures the latest one. Sound
catalog hot reload is suppressed for the same reason. The development-only HTTP
boundary accepts local same-origin writes, logical documents and reviewed candidate
identities; the browser cannot choose publication paths.

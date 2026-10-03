# Recorded playback

Keep the shared observed launch derivation. Carry exact unit type and authored mount
name through `EffectShooter`/`EffectMount` and `Launch`, retaining physical mount indices.
`resolveShot` uses unit/mount choice, then weapon-kind override, then original baseline.
The base mix and distance threshold remain fixture-owned; choice gain multiplies it.

`SoundBank` admits catalog recipes, decodes each used clip once per context, mixes
quiet synth support into the buffer, and selects variants deterministically by shot.
One event consumes one voice. Baseline synthesis is still selectable. Live and
offline paths must await the same preparation, including distance and cost probes.
Failure is explicit; dispose aborts loading. Readiness drops stale publications and
starts from fresh evidence. Hidden cues never receive per-unit identity choices.

Tests first: two types with one weapon differ; two mounts remain independent;
baseline fallback; cadence unchanged; hidden cues invariant; delayed admission,
failed decode and disposal. Preserve existing soundFrame and sound scene coverage.
Run the existing sound scene and output a matched before/after firefight WAV. No
renderer visual changes or simulation state/schema changes are part of this slice.

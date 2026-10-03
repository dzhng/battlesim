# Battle audio

Sound observes the same side-visible causes as effects. It never changes firing
cadence, physical impacts or simulation hearing. Anonymous hearing cues stay anonymous;
an exact unit assignment applies only to an observed launch.

The [sound catalog](../../fixtures/sounds.json) separates provenance, clean clips,
reusable recipes and assignments. Each recording in a recipe is an alternative,
not another shot. Optional synthesis supplies restrained body inside that same
buffer, so one cause still costs one voice. Baseline synthesized recipes remain
selectable under their original IDs. A recorded recipe has a distinct ID.

Assignments identify an exact unit type and an authored mount name. Equivalent
physical mounts share their named equipment choice. The unit catalog owns the
roster; the audio catalog does not resolve inheritance. Hull launch evidence
identifies the mount rather than its currently selected ammunition.

The [sound workbench](../../apps/sound-workbench/README.md) auditions the complete
library, including unused reloads and mechanical actions. Storing a clip does not
create a gameplay event. Audio saves affect a new or explicitly restarted page;
an active battle retains its accepted audio generation.

## Recording preparation

Pinned inputs live under `assets/third-party/audio`, and mono PCM WAVs under
`assets/runtime/audio`. Both use Git LFS; pull these paths in a fresh checkout.
The source hash, frame range and processing profile in the catalog are the rebuild
recipe. [The audio asset tool](tools/assets.py) owns processing and checks prepared
bytes against that recipe. Run its help for commands; it needs Python 3, ffmpeg
and ffprobe, with no additional Python packages.

The tool averages stereo channels, keeps the source crop in its original frame
coordinates, applies the selected tonal profile and fades, and normalizes peak
level with headroom. Loop clips use a crossfade. It admits all pinned sources and
all crop ranges before replacing any outputs. Browser Web Audio handles decoding
and sample-rate conversion; audio loading never happens per projectile.

Family labels describe perceived character rather than an unverified weapon
model. Measurements prove onset, clipping and reproducibility; timbre choices
remain reviewable in the workbench. Source quality and designed reuses are explicit
in each clip's provenance.

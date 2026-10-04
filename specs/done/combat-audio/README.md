# Combat audio and sound workbench

The sound library makes combat recordings reusable and initial choices reversible.
The developer workbench lets a contributor audition unused clips and synthesis,
then change a unit type's equipment sounds without editing game rules. The user
delegated extraction, initial assignments and workbench design; future taste changes
belong in the same catalog rather than another set of code constants.

## Why the library is separate

A recording is source material, a recipe is an audible choice, and an assignment
is where that choice belongs. Keeping these separate preserves useful reloads,
mechanical actions and unused weapon families without inventing gameplay events.
The original synthesized recipes retain their identities so an explicit baseline
selection remains meaningful after other assignments change.

Recorded alternatives and restrained synthesis form one voice for one observed
cause. One clean report per round kept the simulation's timing but made sustained
fire a single sample repeated at machine regularity. Automatic weapons therefore
play recorded bursts: source shots re-laid at the gun's own shot interval, one
voice covering the rounds that land on its shots. An unedited recorded burst would
have its own cadence and multiply the weapon's. The bank, rather than the event
loop, owns preparation and variation.

Short PCM clips keep decoding ordinary and rebuilding reproducible. The current
library is small enough to prepare its recipes before accepting observed events.
Asynchronous readiness drops old publications instead of replaying a backlog when
downloads finish. Audio resources belong to the page and its captured catalog;
departure releases them, and a cached browser return starts from fresh evidence.

## What must stay true

- Sound only observes side-visible causes. Per-unit choices apply to observed
  launches; anonymous hearing cues cannot reveal a hidden unit's identity.
- The resolved unit catalog owns equipment identity. An assignment names the
  exact type and authored mount; equivalent physical mounts with that name share
  a choice. The launch feed does not distinguish a cannon mount's selected
  ammunition, so its firing sound follows the mount across AP and HE rounds.
- A near/far pair may share a recording or use different recordings. Battlefield
  distance still owns attenuation, filtering, attack and reverberation. Workbench
  auditions play the recipe before those distance effects.
- Audio edits publish only the sound document. Preview holds the exact reviewed
  candidate; outside edits invalidate it. No-op publication preserves bytes, and
  existing battles retain their captured generation until a new or reloaded page.
- Source permissions, pinned bytes and exact frame crops survive recipe edits.
  A family name describes character unless source evidence identifies the weapon.
  Measurements establish onset, clipping and reproducibility, not auditory identity.
- Prepared recordings and synthesis share a source loudness reference before
  authored mixing gains. Calibration changes playback gain, preserving sample
  waveforms and their decay. The [loudness owner](../../../packages/battle-audio/src/loudness.ts)
  meters short effects without letting long quiet tails dilute the reference.

## Source review and rejected assumptions

The supplied CC0 gunfire collection and the user's listening annotations anchor
the weapon families. The frozen [source review](../../../packages/battle-audio/tools/gunfire-regions.json)
retains numbered takes, mixed passages and exclusions; the [catalog](../../../fixtures/sounds.json)
owns exact retained crops and source permissions. Listening estimates such as
“halfway” guide review rather than claiming an exact transition frame. The bell
and muddled background passages are excluded. Muffled reports remain available
for the user-suggested hard-object impacts as well as optional firing timbres.

Other combat recordings retain their own provenance and quality limits. A public
preview used as impact design material is labeled as such, rather than presented
as a high-quality weapon master. Source footage and user feedback support choices;
Claude's CLI could review the plan but could not hear these recordings. Envelope
peaks can be reflections or multiple attacks and cannot resolve that question alone.

Padding a shorter loop layer with silence weakened its continuous body. Loop layers
instead repeat at their own periods with a softened composite boundary. Reusing
baseline IDs for recordings would make restoration ambiguous; new recipes keep
distinct identities. Changing battle rules to satisfy unrelated browser staging
would confound the audio work: those scenes now establish identification, retreat
and moving fire through admitted commands and actual observations.

## Where the contracts live

[Audio principles](../../../packages/battle-audio/README.md) lead to `SoundCatalog`,
`resolveShot`, `SoundBank`, `SoundFrame` and `BattleAudio`. The [workbench principles](../../../apps/sound-workbench/README.md)
lead to the editor and shared `FixturePublication` save boundary. The [audio asset
tool](../../../packages/battle-audio/tools/assets.py) owns rebuilding and byte checks.

The [catalog tests](../../../web/tests/soundCatalog.test.ts), [bank tests](../../../web/tests/soundBank.test.ts),
[frame tests](../../../web/tests/soundFrame.test.ts), [lifecycle tests](../../../web/tests/battleAudio.test.ts),
[server tests](../../../web/tests/soundWorkbenchServer.test.ts) and [editor tests](../../../web/tests/soundWorkbenchEditor.test.tsx)
pin identity, preparation, playback and reviewed publication. The [sound scene](../../../web/scenes/sound.mjs)
checks rendered audio, distance and cost through the production preparation path.

The workbench was judged as a compact, readable internal developer tool at desktop
and narrow widths. No uploaded baseline or inspiration image defined its design;
the user delegated that choice. Iteration screenshots are scratch evidence rather
than visual requirements. The final architectural decisions live in [choices](choices.md).

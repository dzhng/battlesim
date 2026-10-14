import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";

/** A test catalog of synthesized baselines, completed for the shipped
 *  presentation: each recording the presentation names directly (a vehicle's
 *  engine loop, the air war's cues) gets a synthesized stand-in that loops as
 *  its slot does, so the battle prepares what production prepares. */
export function standInPresentationRecordings(
  catalog: SoundCatalog,
  presentation: AudioPresentation,
) {
  const { sounds } = catalog;
  for (const row of Object.values(presentation.vehicles)) sounds[row.engine] ??= { ...sounds.rotor };
  for (const cue of [presentation.air_alarm, presentation.missile_warning, presentation.flare_deploy])
    sounds[cue.sound] ??= { ...sounds.launch };
}

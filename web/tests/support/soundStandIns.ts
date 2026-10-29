import { presentationSettings } from "@web/presentationSettings";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { battleSounds } from "@packages/battle-audio/src/soundFrame";

/** A test catalog of synthesized baselines, completed for `presentation`: each
 *  sound Rust says the battle plays that the catalog lacks (a recording the
 *  presentation names itself, such as the jet engine or an air-war cue) gets a
 *  synthesized stand-in that loops as its slot does, so the battle prepares what
 *  production prepares. The presentation module must be loaded. */
export function standInPresentationRecordings(
  catalog: SoundCatalog,
  presentation: AudioPresentation,
) {
  const loops = new Set<string | null | undefined>([
    presentation.ambience.sound,
    ...Object.values(presentation.vehicles).flatMap((row) => [
      row.engine,
      row.running,
      row.turret,
      row.reverse,
    ]),
  ]);
  const { smoke, cook_off } = presentationSettings.effects;
  const plays = battleSounds({
    tickHz: 30,
    presentation,
    catalog,
    smokeTimes: smoke,
    cookOff: cook_off,
  });
  for (const name of plays)
    catalog.sounds[name] ??= { ...catalog.sounds[loops.has(name) ? "rotor" : "launch"] };
}

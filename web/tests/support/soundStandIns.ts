import type { AudioMix } from "@packages/battle-audio/src/audioMix";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";

/** A test catalog of synthesized baselines, completed for `presentation`: each
 *  sound the mix names that the catalog lacks (a recording the
 *  presentation names itself, such as the jet engine or an air-war cue) gets a
 *  synthesized stand-in that loops as its slot does, so the battle prepares what
 *  production prepares. The fake must be admissible before Rust can prepare it. */
export function standInPresentationRecordings(catalog: SoundCatalog, presentation: AudioMix) {
  const loops = new Set<string | null | undefined>([
    presentation.ambience.sound,
    ...Object.values(presentation.vehicles).flatMap((row) => [
      row.engine,
      row.running,
      row.turret,
      row.reverse,
    ]),
  ]);
  for (const row of Object.values(presentation.fires)) loops.add(row.sound);
  for (const row of Object.values(presentation.motors)) loops.add(row.sound);
  const plays = new Set<string>([
    ...[...loops].filter((name): name is string => typeof name === "string"),
    ...[presentation.shots, presentation.blasts].flatMap((rows) =>
      Object.values(rows).flatMap((row) => [row.near, row.far]),
    ),
    ...Object.values(presentation.impacts).map((row) => row.sound),
    ...[
      presentation.air_alarm,
      presentation.missile_warning,
      presentation.flare_deploy,
      presentation.ricochet,
      presentation.footsteps,
    ].map((row) => row.sound),
    presentation.cook_off.blast.near,
    presentation.cook_off.blast.far,
    ...Object.values(presentation.cues.sounds),
  ]);
  for (const name of plays)
    catalog.sounds[name] ??= { ...catalog.sounds[loops.has(name) ? "rotor" : "launch"] };
  // A score uses the same library admission as battle sounds, so its fake
  // tracks need source receipts even though the test controls their transport.
  for (const track of Object.values(catalog.music.tracks))
    catalog.sources[track.source] ??= {
      label: "Test score",
      author: "Test",
      license: "CC0-1.0",
      url: "https://example.invalid/score",
      path: "assets/third-party/audio/score.wav",
      sha256: "0".repeat(64),
      notes: "Controlled test transport",
    };
}

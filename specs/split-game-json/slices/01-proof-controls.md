# 01 Proof controls

**Contract:** every later "nothing changed" claim can be disproved cheaply and
exactly.

**Seam:** a scratch probe (a cargo example or an `#[ignore]`d test; your call)
that fires **every one of the 49 weapon rows** through a real battle and dumps,
unrounded:
- sound commands;
- effect instances and lights;
- poses;
- per-tick battle digests.

Separately it dumps `pipelines::builtin_source` for every entry in
`pipeline_sources.txt`. No production behaviour changes. The probe is a
transitional seam, deleted by [16](16-closeout.md).

Why the committed vectors aren't enough:
- they fire base ids only (`rifle`, `tank_ap`, `grenade`, `hmg`, `tank_he`,
  `atgm`);
- they tolerate error (`presentation/tests/sound.rs:175`) or round it
  (`effects.rs:39`);
- they never exercise a derived or encounter-added weapon.

**Run / see:** `throwaway/split-proof/before/` holds the dumps, the native
plates and `firefight.wav`:
- plates: `cargo run -p native-client -- --run fixtures/parity/capture/run --scene paris-corner --out throwaway/split-proof/before/native`
- WAV: `bun run --cwd web scene -- sound`

Include one encounter that adds a weapon by `extends`, and one hull
publication trace with a two-weapon mount (BMP-3) firing both weapons.

**Verify:** the probe run twice gives identical bytes, which proves it is
deterministic. The focused oracles stay green unflagged:
- `cargo test -p presentation --test sound`, `--test effects`, `--test poses`;
- `cargo test -p renderer-core --test pipeline_abi`;
- `cargo test -p sim --test sim exact_numbers`;
- `cargo test -p mapgen --test layout_cli`.

**Moves:** nothing.

**Depends on:** nothing.

**Delegated:** probe packaging, dump format, file layout under `throwaway/`.

**Would change this slice:** the probe being non-deterministic. Fix the
source of the non-determinism; don't add a tolerance.

**Done (proof controls):** the unrounded battle dumps and composed shaders
live under `/Users/server/dev/battlesim/throwaway/split-game-json/before/`.
The transitional executables were removed at closeout after the final digest
comparison.

Two runs matched all 77 files byte for byte (`before/SHA256SUMS` and
`repeat/SHA256SUMS` in shared scratch). Coverage includes all 49 rows, soldier
poses, an encounter-added derived weapon, and both BMP-3 main-mount weapons.
The focused oracles above and the sound scene passed; `firefight.wav` and six
native plates are in shared `before/`. Native wrote all plates but exited with
a pre-existing backdrop count mismatch at plate 1118 (737 versus 628), recorded
in `before/native/capture.log`; no production code or baselines were changed.

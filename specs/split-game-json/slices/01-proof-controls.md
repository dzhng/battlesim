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

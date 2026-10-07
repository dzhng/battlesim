# 05 Stand-ins are explicit test content

**Unlocks:** the fake units tests run on are named as fakes everywhere, and
the game's catalog can't contain them.

## Rule

Tests run on stand-in or fake units unless a test states why it needs the
roster (the roster's own resolution rules, a roster model's fit, the menu
reel). A roster add, remove or edit must not break a test. This is already
the practice on main (`847c813c`, "Test mechanics on stand-in and fake units,
never the roster": `sim::fixtures::stand_in_game`, `stand_in_documents`); this
slice makes it visible in names and enforced for the shipped catalog. Tests
covers Rust and web tests, browser scenes, parity fixtures, and the
encounters on `test` maps (slice 06).

## Work

1. **Test first:** the shipped catalog (`catalog_documents()`, the browser's
   `fixtures/catalog.json`) has no concrete unit or soldier kind that no
   roster card reaches. Red today: the generic units ship.
2. Move `fixtures/units/generic/` to `fixtures/units/stand-in/` with a header
   saying these are test stand-ins, never game content. Rename ids:
   `stand_in_tank`, `stand_in_jeep`, `stand_in_supply`, `stand_in_rifle`,
   `stand_in_recon`, `stand_in_at`; soldier kinds likewise.
3. **Two resolved catalogs** (user, 2026-10-06). `catalog_documents()`
   excludes the folder and writes `fixtures/catalog.json`: the game's, roster
   only. A second resolved file, `fixtures/stand-in-catalog.json`, holds the
   roster plus the stand-ins (`stand_in_documents()` plus the roster), and the
   same Rust test keeps both current. Game routes (main menu, Play, replays,
   menu backdrop) import only the first; labs, scenes, the benchmark and the
   endurance lab that draw stand-ins import only the second. One catalog per
   page, never merged at run time. A test refuses a game route module that
   imports the stand-in catalog.
4. Rename every use: Rust tests and examples, web tests, browser scenes,
   saved test-map encounters, parity fixtures, mechanics editor tests,
   presentation rows (slice 01's classes). Digests move only because type ids
   sort differently; regenerate and name it in choices.md.
5. **Stand-in art is labelled, not remodelled.** Appearances `tank`, `jeep`,
   `supply_truck` and the generic soldier sets (`rifle*`, `at*`, `recon*`)
   become `stand_in_*`, sources move to `assets/source/stand-in/`, and the
   assets readme says they draw test units only. The wreck exports of
   `blender/tank.py`, `jeep.py`, `supply_truck.py` keep their slice 03 names:
   they are live game art.
6. The mechanics editor lists stand-ins apart from roster units, labelled as
   test units.

## Verify

The guard test; the Rust and web suites that use stand-ins (crate by crate);
`asset bake` and `check`. `git grep` for the bare old ids in quotes finds only
the weapon id `rifle` and the roles `at` and `recon`.

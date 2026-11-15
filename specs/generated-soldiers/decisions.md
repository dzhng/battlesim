# Decisions ledger

Decisions made where the spec was silent, appended per pass. Slice 09 rewrites
this ledger from scratch against the shipped code.

## Slice 01

- **`infantry_roster.py` lives in the Blender folder's root, not in
  `roster/`.** Sound.
  - *Why:* Blender doesn't put a script's own folder on `sys.path`, so
    `roster/` modules can't import each other. The crew code (slice 02) needs
    it too.
  - *Change:* the spec's paths updated.
- **`armies.json` holds only what the generated path and the generation tool
  need:** faction, rifle, LMG, gloves, cloth mean, prompt, looks.
  `PROCEDURAL_LOOKS` keeps the shells' own fields (print, gear colours,
  helmet, boots, per-look variants), and `army_look()` merges the two for
  `Kit`. Sound, and temporary: both the merge and `PROCEDURAL_LOOKS` go in
  slice 08.
- **`KITS` moved into `infantry_roster.py`**, keeping one owner of the
  roster's data beside `source_path`. Sound. The spec said `KITS` stays in
  Python; it does.
- **The infantry receipt lists its skins from the pins, not from
  `skin_pins.WORN`.** Sound: the receipt runs in its own Blender process after
  the exports, so `WORN` is empty there. It also hashes `armies.json`, which
  shapes every export.
- **Vehicle receipts are not refreshed in this pass**, although their scripts
  changed (`skin_pins` is now loaded). Sound: no check reads them, and slice 09
  rewrites every receipt.
- **The generator's prompt text is written now, in `armies.json`**, from
  today's look table; slice 04 composes it. Provisional: slice 05 judges
  whether the wording gets the cues.

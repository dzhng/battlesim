# 15 Infantry

**Unlocks:** soldiers who read as the right army and the right team at a glance.

Owners: `blender/infantry_kit.py` (bodies), `roster/infantry_equipment.py`
(kit and team weapons), `blender/weapons.py`. Shared Quaternius rig, skeleton,
clips and sockets do not change; equipment stays bone-attached and baked into
the skin as today.

## Bar

- **Uniform** pattern per army as a texture recipe: US OCP (matches the tan vehicles), USMC MARPAT
  woodland, German Flecktarn, French CE, British MTP, Russian EMR (Digital
  Flora), Chinese Type 07 woodland. Which army each kit is comes from its
  roster faction and reference; Europe's shared kits use one documented choice
  (record it).
- **Helmet** shape per army with cover and NVG mount; **plate carrier** with
  pouches, radio on leaders; boots and gloves dark.
- **Weapons** at their real silhouettes: M4/M27, AK-12, HK416/G36 per army;
  M110, SVD, M107, ASVK; RPG-7, RPG-29; TOW and Kornet on tripods with optics,
  carried as their real loads; materials from slice 10 (steel, polymer black,
  furniture).
- **Budget:** there is no infantry triangle budget today (the "existing
  per-bundle gates" the archived roster spec mentions don't exist). Measure
  today's soldier tiers (about 27k / 7k / 2.4k / 0.75k triangles) and set a
  soldier budget per tier, plus bytes, in choices.md before adding detail;
  soldiers are many on screen, so the bar is silhouette and colour first.
  Soldier impostor cards take 2 texture layers per appearance (168 of 256
  today, `impostorCards.ts:198`) and are baked on the GPU at every battle
  start from tier 0 (`battleFrame.ts:291-308`): a heavier tier 0 costs start-up
  time on the Mac mini; measure it.
- Weapon materials keep a role (slice 10) so vehicle crew can hide a carried
  weapon by role, not by name (slice 14).

## Verify

validate, bake, check (rig, socket and muzzle fit); `asset sheet` contact and
animation strips for each kit with `--references`; compare-screenshots
against the current accepted sheets in `assets/review/`; screenshot-critique
unprimed, last; accept the new sheets (`--accept`). One lab scene per faction
at battle distance. User preview non-blocking as in slice 13.

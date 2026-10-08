# 17 Infantry

Runs in parallel with the other lanes (slices 15–18) once the pilot (14)
sets the bar; workers take one family at a time, export and validate only,
and the coordinator bakes and commits runtime output in batches.

**Unlocks:** soldiers who read as the right army and the right team at a glance.

Owners: `blender/infantry_kit.py` (bodies), `roster/infantry_equipment.py`
(kit and team weapons), `blender/weapons.py`. Shared Quaternius rig, skeleton,
clips and sockets do not change; equipment stays bone-attached and baked into
the skin as today.

Collect each kit's references first (README [References](../README.md#references)).
The `rifle_squad` kit was built in the pilot (14) and sets the bar and the
soldier budget.

From the pilot (2026-10-07): a soldier appearance names a look per faction
(`factions` in `assets/catalog.json`, validated by `structure.faction_look`);
the US OCP and Eastern EMR rifle-squad looks exist. Still to do here: a Europe
look (it wears OCP until then), the other 16 kits' faction looks, colour that
survives at impostor-card range (both armies read as dark marks at ~250 m), and
red's side tint pushing EMR toward olive.

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
  carried as their real loads; materials from slice 13 (steel, polymer black,
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
- Weapons stay their own nodes, so vehicle crew can hide a carried weapon by
  its node (slice 09); weapon materials keep their roles (slice 13).

## Verify

validate, bake, check (rig, socket and muzzle fit); `asset sheet` contact and
animation strips for each kit with `--references`; compare-screenshots
against the current accepted sheets in `assets/review/`; screenshot-critique
unprimed, last; accept the new sheets (`--accept`). One lab scene per faction
at battle distance. User preview non-blocking as in slice 15.

## Delegated

Pouch and kit layout within the references and the soldier budget; which
Europe-faction uniform the shared kits wear (record it).

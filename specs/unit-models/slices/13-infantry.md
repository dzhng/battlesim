# 13 Infantry

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
  carried as their real loads; materials from slice 08 (steel, polymer black,
  furniture).
- Detail held to the infantry body's existing budget and tiers; a soldier is
  small on screen, so silhouette and colour come before small parts.

## Verify

validate, bake, check (rig, socket and muzzle fit); `asset sheet` contact and
animation strips for each kit with `--references`; compare-screenshots
against the current accepted sheets in `assets/review/`; screenshot-critique
unprimed, last; accept the new sheets (`--accept`). One lab scene per faction
at battle distance. User preview non-blocking as in slice 11.

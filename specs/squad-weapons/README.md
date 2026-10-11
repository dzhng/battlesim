# Squad weapons

Real squads carry a mix of weapons. A marksman squad is riflemen with a few
marksman rifles, a grenade launcher and a machine gun, not six identical
marksman rifles. A sniper team is a spotter and a sniper, and the sniper still
has a rifle when his big gun runs dry.

## The rule

Every soldier carries an unlimited hand weapon, his squad gun: a rifle,
carbine, SMG or shotgun. A specialist also carries one finite extra weapon
(`special`): a grenade launcher, a marksman rifle, a light machine gun, a heavy
sniper rifle, or a launcher. He uses one weapon at a time, choosing the extra
whenever it is useful. When he falls, the next living soldier takes the extra
up. So no squad runs dry, and a squad's character comes from its ratio of
specialists, not from a uniform issue.

The mechanism exists already (the grenadier and the anti-tank gunners); this
plan is data, art, and the anti-air row.

## Compositions

| Card | Soldiers |
|---|---|
| Rifle Squad (8) | 5 rifle, 1 grenadier, 1 machine gunner, 1 marksman |
| Assault / Marine Squad (8) | 4 carbine, 2 grenadiers (carbine), 1 machine gunner (carbine), 1 marksman (rifle) |
| Marksman Squad (6) | 2 rifle, 3 marksmen, 1 grenadier |
| Sniper Team / Scout Snipers (2) | a spotter (rifle), a sniper (rifle + heavy sniper) |
| Scouts / Army Scouts / Recon Patrol (4) | 3 rifle, 1 marksman |
| Force Recon (4) | 2 rifle, 1 grenadier, 1 marksman |
| Anti-tank teams (3) | unchanged: a gunner and two riflemen |
| Air-defence teams (3) | a gunner (rifle + MANPADS) and two riflemen |

## Weapons

- **Carbine** (was "Assault Rifle"): the short squad gun.
- **SMG** and **Shotgun**: short-range squad guns for future squad types; no
  card carries them yet.
- **LMG**: a finite extra with long belts, longer range than the rifle and
  heavy suppression.
- **MANPADS** (Stinger, Igla-S, FN-6): a guided missile that reaches only
  aircraft. The gunner keeps the aircraft in sight while it flies, as the
  ground missiles do; a simplification of a fire-and-forget seeker, named as
  one.

## Models

Each specialist looks like what he carries. The soldier's own appearance is his
squad's rifleman; the extra weapon's `operator_appearance` swaps in the
specialist's model while he uses it, and back to the rifleman when he uses his
rifle. A comrade who takes the extra up wears it the same way.

New kits: a light machine gun per army (M249, MG4, PKP), a Marine marksman
(M110 in MARPAT), and the air-defence teams' remaining looks and carried
poses.

## Slices

1. Weapon rows: carbine rename, SMG, shotgun, LMG, icons.
2. Compositions, with the "no squad runs dry" contract.
3. MANPADS rows and the three air-defence teams.
4. Models: LMG kits, Marine marksman, air-defence looks; appearance wiring.
5. Closeout: balance report, full checks, screenshots.

## Handoff

Slice 1 is next.

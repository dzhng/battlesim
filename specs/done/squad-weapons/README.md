# Squad weapons

Real squads carry a mix of weapons. A marksman squad is riflemen with a few
marksman rifles and a grenade launcher, not six identical marksman rifles; a
sniper team is a spotter and a sniper who still has a rifle when the big gun
runs dry. A squad's character comes from its ratio of specialists, not from a
uniform issue.

## The rule

Every soldier carries an unlimited squad gun (rifle or carbine; SMG and
shotgun rows wait for squads that want them). A specialist adds one finite
extra, a `special` mount: grenade launcher, marksman rifle, LMG, heavy sniper
or launcher. He uses one weapon at a time, the extra whenever it is useful, and
a comrade takes it up when he falls. The mechanism is the one the grenadier and
the anti-tank gunners already used; this feature is data. [The catalog
test](../../../crates/sim/tests/catalog.rs) holds every roster soldier to it.

Compositions live in the roster documents
([`fixtures/units/roster/`](../../../fixtures/units/roster/)); the weapon rows
in [`game.json`](../../../fixtures/game.json).

## Air defence

The Stinger, Igla-S and FN-6 reach only aircraft. The guided-missile path
needed no new code: a guided row whose `targets` is the low-air band already
aims, steers and strikes a helicopter. The gunner keeps the aircraft in sight
while the missile flies, as a ground crew does its anti-tank missile. That is
a named simplification of a fire-and-forget seeker; a seeker would need the
missile to know where an unseen target is. One hit downs most helicopters, the
armoured Hinds survive one, and a team carries three.

## Looks

A specialist wears his squad's rifleman and swaps to his weapon's look while
he uses it (`operator_appearance`), so a comrade who takes the weapon up wears
it too. Shared squads name one set whose faction looks pick each army's
weapon (M249, MG4, PKP; M110, G28, SVD). The kits are
[Blender-scripted](../../../packages/scene-assets/blender/roster/infantry_equipment.py);
the machine guns' [references](../../../assets/references/machine_gunner/).

## Measured shift

A throwaway duel probe (each card against a rifle squad, six seeds, 150 to
450 m) showed fights opening further out and costing more: every squad's
marksman engages at 450 m, the machine gun at 400 m. Marksman squads now win
only at range, and sniper teams draw return fire from any squad's marksman.
Costs were left unchanged; balance is its own pass.

## Open

- An unprimed review of the art flagged pre-existing pose issues the new kits
  inherit: the shouldered MANPADS reads like a rifle, the Stinger's IFF block
  floats, idle holds are one-handed, and a slung slim tube's foot stands off a
  leaning soldier. They belong to the shared clips and launcher builders.

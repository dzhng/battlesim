# 14 Chapter 4: Steel on the ridge

**Unlocks:** armour facing, smoke and reverse, an AT ambush from hold fire, and
control groups. See [story.md](../story.md#chapter-4-steel-on-the-ridge). It
follows the chapter slice shape in [13](13-chapter-3.md#shape-of-every-chapter-slice-1317).

**New vocabulary:**

- `UnitFact::Destroyed`;
- `UnitFact::PoppedSmoke`;
- `UnitFact::Reversing`;
- `UnitFact::Ordered(Attack { target: Selector })`, read from the unit's
  current order state, which is a fact, not a command;
- the first recovery beat, using `Grant` with `Covering`.

**Red:** the `tank` (worn hull, fire at will) waits behind the ridge. The
director sends it down `ridge_road` when the chapter opens.

**Blue:** the chapter opens with an `Offer` and a `Grant` covering an AT team.
The coach asks for it in `ambush_woods` on return fire only. Control groups are
a UI-gated coach step (the walkthrough doesn't need them).

**Gate addition:** a director test that the recovery beat re-grants when the AT
team dies before the tank does (R8).

**Delegated:** whether reversing is judged by motion against the hull's facing
or by the move order's direction (prefer the fact the sim already publishes).

# 16 Chapter 6: Eyes in the sky

**Unlocks:** helicopters, jets (arrival, attack passes, fuel), air defence, radar
contacts and SEAD. See [story.md](../story.md#chapter-6-eyes-in-the-sky). It
follows the chapter slice shape in [13](13-chapter-3.md#shape-of-every-chapter-slice-1317).
It is the heaviest chapter: if its ledger keeps filling, split the jet and SEAD
half into its own slice before going on.

**New vocabulary:**

- `UnitFact::Airborne`;
- `UnitFact::Pending` and `UnitFact::Arrived` (a purchase on its way, then on
  the field);
- `Knowledge::RadarContact`;
- `Action::Order { order: ConfirmPurchase }` for red, so red's helicopter arrives
  by an ordinary purchase paid for by a red `Grant`, not by a spawn.

**Red:** the helicopter (bought on cue at `red_entry`, worn and short of
ammunition) and the `sam` on `hill`, emitting radar and pre-placed. Blue can't
see it until a SEAD jet finds it.

**Blue:** offers for air defence, a helicopter, a jet and a SEAD jet, each with a
covering grant. Recovery re-grants air defence if it is lost.

**Gate addition:** run a small air and radar check on test units first (a
director test that `RadarContact` fires for the side whose SEAD found the
emitter, and for no other side), then the full walkthrough.

**Delegated:** which jet card teaches attack passes (P4's pick); whether one jet
can be both the attacker and the SEAD.

# 15 Chapter 5: Keep it running

**Unlocks:** mid-battle purchases, supply (deploy, resupply, repair, replacement)
and attack-ground with a mortar. See [story.md](../story.md#chapter-5-keep-it-running).
It follows the chapter slice shape in [13](13-chapter-3.md#shape-of-every-chapter-slice-1317).

**New vocabulary:**

- `UnitFact::Deployed`;
- `UnitFact::Serviced` (ammunition delivered, health repaired or a soldier
  replaced, read from the supply owner's published state);
- `UnitFact::Ordered(AttackGround { zone })`;
- `Grant` as "top up to cover", meaning the shortfall only, because the wallet
  already holds income.

**Red:** the `treeline` team in `far_treeline`, hidden, on return fire only, so
the player must fire on ground they can't see into.

**Blue:** offers for a supply truck and a mortar. By this point the chapter 3–4
units are worn and short of ammunition, so supply has work to do (no extra
staging).

**Delegated:** which blue units the solution brings in for service.

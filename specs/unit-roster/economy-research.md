# Bounty research and selected starting values

The numerical bounty baseline below was accepted by the user for the first
playtest. It is a starting rule, not evidence of tested balance.

## Dota reference

Dota hero kills combine a direct killer reward with participation rewards. The
direct reward uses a base amount, victim level and a shutdown bonus for ending
the victim's kill streak; it is not a running account of half the value killed.
The base formula is 125 + 8 × victim level + streak bonus, with a separate
first-blood bonus. The streak bonuses introduced in
[official patch 7.31 notes](https://www.dota2.com.cn/article/details/20220224/220244.html)
are 60 / 100 / 150 / 210 / 280 / 360 / 450 / 550.

The [official 7.40 notes](https://www.dota2.com.cn/article/details/20251216/220450.html)
give the revised participation-gold component as
15 + (50 + 0.037 × victim net worth) / participating hero count. The
[current community gold reference](https://liquipedia.net/dota2/Gold) also documents
team-net-worth scaling for participation rewards. Treat that multiplier as a
community-documented mechanic, not something established by the 7.40 note alone.
Hero-specific abilities and exceptional deaths add further rules.

The useful design lesson is to separate the ordinary kill payment from the
bonus for defeating a valuable or successful opponent. Hero levels, item net
worth and respawning heroes do not map directly to permanently destroyed RTS
units, so a Dota numerical ratio is not a ready-made tuning value for this game.

## Accepted first-playtest baseline

Let C be the destroyed unit's original purchase cost and S the equal starting
credit budget. B is the victim team's bounty pool immediately before its loss.

- Pay the killing player ordinary kill credits equal to **25% of C**.
- Add **50% of C** to the killing team's bounty pool, using the victim's cost,
  not the collected credits or bonus.
- Pay a bounty bonus equal to **min(B, 50% of C, B × C / S)** and subtract the
  paid amount from the victim team's pool.
- Chosen loss adjustment: subtract **50% of C** from the victim
  team's remaining pool, stopping at zero. This makes equal-value trades clear
  accumulated pressure rather than growing both pools indefinitely.
- Ordinary plus bounty kill credits are therefore capped at **75% of C**.
  Supply trucks add a separately accepted **25% of original purchase cost**,
  paid once on destruction regardless of remaining stock. Maximum total reward
  for a supply truck is therefore 100% of its cost; pool growth remains 50% C.
- Passive credit income is unchanged. Returning a unit does not clear its team's
  pool, as previously recommended in the main spec.

The fixed starting budget is the selected payout scale rather than a denominator
based on current army value. Buying fresh units cannot dilute the bounty rate,
and the rate does not require calculating hidden enemy army strength.

Illustrative values, not unit prices: with S = 1,000 and C = 200, a pool of zero
pays 50 credits; a pool of 200 pays 90; a pool of 500 pays 150. The latter is
50 ordinary credits plus the capped 100-credit bounty bonus. The losing team's
pool then loses the paid bonus and, under the selected loss adjustment,
another 100 credits. The killing team's pool grows by 100 in every case.

## Selected integration rules and remaining proof

The user selected team bounty, net-combat-success accounting and the full
numerical baseline above. Passive income is 200 credits/minute; starting budget
is five minutes of income, currently 1,000 credits. Supply-truck bonus is 25% C.
The roster spec's lifecycle decisions define full-unit lethal-source attribution,
wallet-only unseen rewards, symmetric same-tick accounting and fractional carry.
The refund range is 15–75%, with its time/health/ammo formula selected by the user.
There is no additional pool cap in the initial baseline; per-death bonus remains
capped at 50% C. Teammate sharing is deferred with team play.

Remaining proof: measure ordinary battles and deliberately symmetric trade cases,
shutdowns against new reinforcements, supply raids, friendly-fire denial and
purchase/refund cycling. The numerical formula is selected, not proven balanced.
Use the existing deterministic simulation/replay verification when implemented;
this planning work runs no battles.

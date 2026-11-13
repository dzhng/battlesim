# 09: roster switch-on (first playable)

## Contract

The roster units named in D2 carry smoke with the charges D2 gives them. A
player can pop smoke in a real skirmish. This is the feature's first playable
checkpoint.

## Seam

Author `capabilities.smoke` on each unit row in
`fixtures/units/roster/{us,europe,eastern,shared}.json`, never on a profile or a
part.
- **Why not profiles:** profiles mix in other units. The light-tank profile also
  covers artillery, and the IFV profile also covers SAM launchers.
- **Why not parts:** parts merge after the row and imply hardware nodes.

**Cloud values (D6):**

| Carrier | Radius | Height | Life | Bloom | Fade |
|---|---|---|---|---|---|
| Tanks and IFVs | 15 m | 6 m | 40 s | 1 s | 10 s |
| Infantry | 12 m | 5 m | 30 s | 1 s | 10 s |

- All carriers: `depth_per_m` 0.2, `concealment` 0.1, 4 bloom steps and 5 fade
  steps.
- Looks: `discharger` for tanks and IFVs, `grenade` for infantry.
- Price and service time follow the flare rows; choose them and record them.

**Charges (D2):**
- **Tanks, by cost:** ≤300 gets 1, 301–399 gets 2, ≥400 gets 3. These are the 19
  non-IFV `mbt`-role rows.
- **IFVs, 1 each:** T-15, Puma, CV90 Mk IV, M2A4.
- **Infantry, 1 each:** `assault_squad` (Europe and Eastern), the US Marine
  squad, and US Force Recon.

**Inheritance:** rows inherit from other rows. Abrams SEP v2 → v2 Trophy and
v3 Trophy → M1E3; Challenger 2 → Challenger 3; Leopard 2A7V → 2A8. A value on a
parent flows down unless it is overridden, so check each row's **resolved**
charges.

**Regenerate** `fixtures/catalog.json` with `BLESS_CATALOG=1`.

## What you can see

A skirmish: buy a Leopard, take fire, press H, reverse out. The enemy's view
keeps the last contact marker where you were.

## Tests and checks

- **Resolved charges:** a scratch script in `throwaway/smoke-screen/` prints
  every row's resolved charges next to its cost band and the D2 list. The human
  reviews the printout; it is not a test, because a roster is data (AGENTS.md).
  Copy the printout into `choices.md`.
- **Named decision:** record the shift in roster battle digests as a named
  decision in `choices.md`, with `BLESS_PARITY` where pinned records include
  roster units.
- A quick skirmish sample while iterating.

## Delegated

You may choose the price and service time (follow the flare rows) and the order
of JSON keys.

## Must stay green

- Test-unit and capture-unit digests.
- The menu reel's event-log judge. Capture units have no smoke, so it should be
  unchanged; confirm it.

## Feedback that would change this slice

- If the user gives the Centauro II smoke, that's one more row (README, Still
  open).
- Changes to charge counts are data only.

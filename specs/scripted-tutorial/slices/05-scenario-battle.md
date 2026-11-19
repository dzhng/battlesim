# 05 Scenario battle: address, preparation, offered shop

**Status: shipped 2026-11-19.** `/battle?scenario=ferryman-valley/operation-ferryman`
opens the tutorial's battle in preparation with 390 CR and a shop narrowed to
the scouts and the rifle squad in its Bradley, under only the REC and INF tabs
([shot](../../../throwaway/evidence/scenario-battle/shop.png) is scratch; the
`scenario-battle` scene re-shoots it). Evidence: `scenario-battle` scene passes;
`crates/sim/tests/scenarios.rs` (narrowed shop refusals and publication round
trip); the picker tests; the `ui` scene with its two pause-menu baselines
re-blessed (only the label changed). Deviations:
[decisions.md](../decisions.md#slice-05).

**Unlocks:** the tutorial battle is playable (no story yet). Open it, buy the
offered cards with the granted credits, place them, start, and fight the
outpost.

**Contracts (C4, C5):**

```ts
// web/src/battle/prepare/protocol.ts
export interface PrepareScenarioMessage {
  type: "prepare-scenario";
  scenario: { map: string; name: string };
  documents: PrepareDocuments; // today's rules, as for a skirmish
}
// PreparedBattle.report gains `scenario?: ScenarioPublic`
```

- The request carries no players, seed or rules, because the document fixes
  them. The Rust mirror is `contract::preparation::PrepareScenarioRequest { map,
  name }`, which refuses unknown fields.
- Address: `/battle?scenario=<map>/<name>` in `battleLinks.ts` (`askedBattle`
  kind `scenario`). After preparation it is the ordinary battle route and
  `BattleView`. The forces never reach the page's main thread.
- **Offered shop:**
  - `SkirmishSetup.offered`, where an absent player means "the faction's whole
    shop" (every generated skirmish).
  - `Skirmish::reserve_at` refuses a purchase outside the offer with a new
    `OrderError::NotOffered`. A squad with its carrier must be offered as that
    pair.
  - The offer is digested. A per-player publication group `offered` of
    `[variant, carrier]` type indices is present only when the player has an
    offer.
  - The purchase picker (`web/src/battle/present/purchasePicker.tsx`) shows only
    offered cards when the group is present. **Hide, don't disable**: the
    tutorial shop should read as a short list, and game-ui confirms it.
- `Skirmish::{offer, grant}` are the functions the director's `Offer` and
  `Grant` actions will call (06). They land here with their tests.

**Playable:** `/battle?scenario=ferryman-valley/operation-ferryman`. The valley
opens in preparation with the outpost in place. The shop shows only chapter 1's
cards, and the wallet covers them. Buy, place with right-drag facing, Start;
the units drive in, and the outpost fires back.

**Gates:**

- `web/tests/prepareBattle.test.ts`: a scenario request is admitted; an unknown
  map, a non-scenario map or an unknown name is refused at its stage.
- `crates/sim/tests/offered_shop.rs` on test units:
  - a purchase outside the offer is refused;
  - no offer means the faction's shop;
  - a squad with its carrier needs the pair offered;
  - offer growth and grants are digested and survive replay.
- A publication reader test for the `offered` group.
- New scene `web/scenes/scenario-battle.mjs`: reaches a playable battle and
  shoots the preparation view and the shop. Use compare-screenshots on the
  picker against today's full shop (variable: the short list reads as
  intentional); run screenshot-critique last. Show both inline (non-blocking).

**Delegated:** the loading screen's subject text (the scenario title is
presentation data in the coach document); worker message plumbing.

**Must stay green:** skirmish preparation (`prepare`), replay and bundle
preparation, existing purchase tests and digests.

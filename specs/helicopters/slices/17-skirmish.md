# 17 — Skirmish: entry and AI

**Status:** planned. **Depends on:** 14, 16. **Owns:** D12, D15, D16, D35, D36.

## Contract

In a skirmish you buy any helicopter. It flies in from your edge at cruise altitude without waiting for a clear road. The AI buys and flies helicopters too. Out of ammo, they wait for orders.

## API seam

- Air entry at the entry site's XY at cruise altitude (`battle.rs`, D36).
- `skirmish_ai.rs`: `Hel` rotation entries, ordered like light vehicles (D35).

## What you can run or see

A skirmish at `/battle?...`.

## Verification

Tests:
- `a_bought_helicopter_flies_in_without_waiting_for_a_clear_road`
- `the_skirmish_ai_buys_and_orders_helicopters`
- `an_empty_helicopter_does_not_return_to_base`

## Delegated to the implementer

How often the AI buys a helicopter within the rotation.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the AI's helicopter use looks silly (for example, suiciding into AA once AA exists), its order policy changes.

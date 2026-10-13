# 03 — Player slots and command ownership

## Contract unlocked

Represent player slots separately from combat sides and factions. Every unit has exactly one slot owner. A controller may issue commands only for its own units, even when slots share a side or observation.

## Seam

Add typed slot identity and admission rules to the contract selected by slice 00. The effective controller rules are:

- no more than one active human;
- a human-requested slot may be controlled by active AI at play time;
- an unoccupied human-requested slot becomes passive AI;
- an AI-only slot always uses active AI;
- neutral slots default to passive policy;
- all controller orders enter the ordinary `Battle::accept` path.

The selected topology must make ownership visible to command validation, replay, publication metadata, and the browser worker without creating a second UI-only ownership table.

## Runnable artifact

A contract/sim fixture with two same-side slots and one neutral slot accepts each slot's own move/fire orders, rejects cross-slot unit ids, and reports normalized controller state.

## Verification

- duplicate/missing slot and unit-owner diagnostics;
- one-human admission and takeover state transition;
- commands addressed to an ally are rejected;
- accepted controller commands retain deterministic ordering and identity;
- same-side knowledge sharing, if retained by the topology verdict, does not widen command authority.

## Delegated implementation choices

Choose the smallest command-envelope extension that preserves one authority path. Do not let the UI bypass slot ownership by submitting raw side commands.


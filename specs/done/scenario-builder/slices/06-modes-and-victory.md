# 06 — Sandbox/economy modes and script-defined victory

## Contract unlocked

The bundle can select authored sandbox or economy behavior, and scripts can end or continue a battle through deterministic victory/event signals.

## Seam

Create one mode owner in the contract/simulation. Sandbox starts authored units and does not construct wallet/purchase/dispatch state. Economy opts into the existing skirmish settlement/objective owner after slice 00's topology verdict; do not duplicate economy rules in the builder.

Script-defined victory/events evaluate through the accepted sandbox host and publish a deterministic result. Neutral/passive slots do not become hostile merely because a victory script observes them.

## Runnable artifact

One sandbox scenario ends through an authored condition; one economy scenario enters the existing preparation/settlement path. Unsupported side/economy combinations refuse with a precise diagnostic.

## Verification

- mode omission/presence changes identity and behavior;
- completion ordering and ties are deterministic;
- economy-off scenarios never allocate partial wallet state;
- script outputs remain observation-safe and replayable;
- targeted objective/skirmish tests stay green.

## Firewalls

No campaign progression, network rewards, arbitrary rules editor, or economy semantics invented outside the existing simulation owner.


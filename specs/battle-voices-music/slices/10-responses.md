# 10 — Responses in battle (R1–R12)

**Contract this unlocks:** a player hears their units answer when selected and when given orders. The owner is the only one who hears these lines, wherever the camera is.

## Seam

**`packages/battle-audio/src/voices/responses.ts` → `responseFor(event)`** maps events to `VoiceRequest{tier:"response"}`.

| Trigger | Event | Source |
|---|---|---|
| R1 selected | the selection changed to include this unit | `setSelection` (`web/src/battle/input/useUnitControl.ts:52-86`) |
| R2 move · R3 attack · R4 attack-move · R5 stop · R6 weapons free · R7 hold fire · R8 enter building · R9 leave building · R10 withdraw | the order, once it is accepted | `issue()` → `CommandAck` (`useUnitControl.ts:151-161`; `Order` kinds in `crates/contract/src/command.rs:111-186`) |
| R11 arrived | a unit's `OwnUnit.goal` goes from `Some` to `None` after a move, or a purchased unit lands | observation diff (not an ack) |
| R12 refused | `CommandAck.error` is `NoValidDestination`, `OutOfBounds`, or another refusal a player caused | `command.rs:205-259` |

Other rules:
- **Group orders:** the `groupKey` is the order's id. The clicked unit answers, or for a box select the highest-value unit.
- **R1 on a box select:** the highest-value unit answers.
- **Wiring:** `BattleAudio.respond(request)` is called from `useUnitControl`'s `onIssue` and selection callbacks, through `apps/battle-lab/src/useBattleSession.ts`.
- **Loading:** `VoiceBank.load` starts for the sets in this battle, **after** `ready`.

## What the human hears
In a skirmish (`/battle?…`):
- select a unit → it answers;
- give an order → acknowledgement;
- an unreachable order → refusal.

## Verification
- `responses.test.ts`: an event maps to the right trigger and tier; a group collapses to one answer.
- One battle-lab scene in which a response is requested and played. Use the sound scene's harness (`web/scenes/sound.mjs`).

## Delegated to you
- the exact R12 error list;
- the R11 debounce for very short moves.

## Must stay green
- input tests;
- `web/scenes/sound.mjs`.

## Checkpoint (non-blocking)
A short skirmish for the user to play, then proceed.

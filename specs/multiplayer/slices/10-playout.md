# 10 Playout

**Stage:** Foundation. **Needs:** 08. **Carries:** P22; the sharp edge at
`web/src/battle/sim/client.ts:277`.

## Contract

Units move evenly however unevenly records arrive. A reliable stream never
loses a record: it stalls and then delivers a burst, and today unit motion is
timed by when records arrive, so a burst shows as a lurch.

## Seam

New `web/src/battle/remote/playout.ts`, between the link and the connection:

```ts
function playout(tickHz: number, delayMs: number): {
  push(reply: SimReply, now: number): void;
  due(now: number): SimReply[];
  queuedMs(): number;          // battle time waiting
  reset(): void;               // the stream was reopened
};
```

- Publications are released one tick apart, `delayMs` behind arrival (about
  100 to 150 ms). Everything else keeps its place in the stream.
- When nothing is waiting it holds, and never extrapolates. The next record
  to arrive is held for the full delay again.
- Records cannot be skipped: the reader needs every delta in order. A page
  holding more than about twice its usual buffer fast-forwards back to it,
  every tick in order, then plays evenly again.
- The caller decides when fast-forwarding is too much: before asking what is
  due it reads `queuedMs()`, and past about two seconds it reopens its stream
  with no `after`, takes the snapshot and calls `reset`. A tab that was
  hidden returns this way.

**Built so far (2026-11-14):** the module and its tests. Time is an argument,
so there is no clock to inject. Putting it between the link and the
connection waits for slice 07, and tuning the delay by eye waits for slice 08.

## Run or see

The dev battle with the browser's network throttled to a jittery profile:
units glide, and a tab hidden for a minute returns to the present within a
second or two.

## Verification

Tests with a clock the test drives:

- Records arriving in bursts leave at even spacing.
- An underrun holds, then resumes without a jump in tick order.
- A backlog under the bound is released in order with none dropped.
- A backlog over the bound asks for a reopen.

## Must stay green

Local play does not pass through playout.

## Delegated

The delay and the bound, tuned by eye on the dev battle.

## Feedback that would change this

If the user finds the added delay worse than the jitter, lower it; the
research gives 100 ms as the floor others use.

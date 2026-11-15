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
function playout(clock: Clock, tickHz: number, delayMs: number): {
  push(reply: SimReply): void;
  due(now: number): SimReply[];
  behindMs(now: number): number;
};
```

- Publications are released one tick apart, about 100 to 150 ms behind
  arrival. Everything else passes straight through.
- On underrun it holds the last tick. It never extrapolates.
- Records cannot be skipped: the reader needs every delta in order. So a page
  that is behind by under about two seconds releases its backlog back to
  back, and a page further behind (a tab that was hidden) reopens its stream
  with no `after` and takes a snapshot.

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

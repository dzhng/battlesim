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

## As built (2026-11-15)

Playout sits in the remote connection, between the stream and what the page
receives.

- **A page's publications leave one tick apart, 120 ms behind arrival.**
  Status and end records wait their turn among the publications around
  them. Acknowledgements and preview answers are not part of the stream and
  are never delayed.
- **Past two seconds of battle waiting, the page reopens its stream** with
  no `after`, drops what it had not played, and starts again from the
  snapshot. The bound is checked before anything is released, so a tab that
  was frozen never plays its backlog.
- **Releases are driven by a timer in a small worker of the connection's
  own,** one per battle tick. A hidden tab's page timers are throttled; a
  worker's are not, so a hidden tab keeps decoding and stays current.
- **One clock** gives the connection its time, its release timer, its batch
  retry and its reconnect waits, so tests drive all of them.
- A link's `records(after, signal)` can now be abandoned mid-read: the
  reopen needs that.

Evidence, twenty ticks a second with records arriving three at a time at
irregular intervals up to 180 ms apart: all 39 gaps between releases were
50 ms, with every tick in order.

The delay (120 ms) and the bound (2 s) are the plan's starting values. They
have not been judged by eye: nobody has watched units move under a jittery
network yet. That is for the user at the next playable checkpoint, and the
slice's "Feedback that would change this" still stands.


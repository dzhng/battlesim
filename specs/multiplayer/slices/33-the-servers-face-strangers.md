# 33 The servers face strangers

**Stage:** Hosting. **Needs:** 31. **Carries:** P5, P25, decisions 14 and
68. **Blocks** putting the lobby's address in the site.

## Why

Before the site points at the lobby, a reader with no history in this
plan read the two servers as an attacker would (2026-11-16). Each finding
below was then checked against the code. The lobby rents a machine on the
owner's account for whoever asks, so the first group is about money as
well as service.

## What was found, and what must hold instead

Anyone can do each of these with a browser id of their own making.

1. **Take every battle place, or rent machines without end.** One id can
   host a room for every place the lobby has, and Start each one; a loop of
   Play and leave rents a machine, draws its maps and gives it back, as
   often as it likes.
   **Instead:** a browser id hosts one room at a time. A client address
   holds a small number of rooms and Frontline starts at once. The lobby
   starts machines no faster than a rate set beside the other host
   settings, and answers `full` beyond it.
2. **Fill the lobby's memory, or everyone's bandwidth.** A room's map seed
   may be any text up to the body limit, and is copied into the public
   list; every action in a room sends the whole list to every reader again,
   changed or not; a reader that stops reading is buffered for megabytes,
   and nothing limits how many streams one client opens.
   **Instead:** a seed is a seed and nothing longer. A list or room view
   that has not changed is not sent. A browser id has one stream of each
   kind, a client address a bounded number, and the server a ceiling. A
   state stream's reader is given a small buffer.
3. **Lock everyone out of joining rooms.** Wrong guesses at a room code
   are counted against the socket's address, which behind the host's proxy
   is the same for every player, and the count refuses right codes too.
   **Instead:** guesses are counted against the client's own address, read
   from the header the host's proxy sets and overwrites (checked on
   2026-11-16: `x-real-ip` on the public route; a caller's own value for it
   does not survive). The header's name is data under `deploy/`.
4. **Find private rooms without guessing.** An action on a room answers
   differently for a room that exists, with no throttle.
   **Instead:** a caller who is not in a room is told `unknown`, whatever
   they ask of it, and that counts as a guess.
5. **End a battle for everyone.** An order may list any number of units
   and be as large as the body limit; the simulation keeps every order it
   is sent for the replay, refused or not.
   **Instead:** an order names no more units and is no larger than a
   bound, checked before the battle sees it, and a batch holds no more
   orders than a seat may give in its window.
6. **Break a seat for whoever sits in it next.** A seat's place in its
   order sequence outlives its occupant, and a huge sequence number is
   accepted.
   **Instead:** a seat's transport state belongs to its occupant and is
   new for the next one.
7. **Predict room codes.** Codes come from the same ordinary random source
   as the map seeds published beside them.
   **Instead:** codes and keys come from the system's secure source.

Also to do here, each small: a client address holds one seat in a battle
(two ids from one person would otherwise read both sides); a log line
cannot be split by a newline in a crash's text; the servers close
connections that have gone quiet.

## Not here

- A cap on how many orders may wait in a unit's queue, and trimming the
  replay's record of refused orders: both change the simulation and its
  digests. Named for a later decision.
- A longer room code. Decision 14 keeps `TANGO-4821`; with guesses counted
  per client it holds for an alpha among friends.
- The battle's seed in a ticket: a player facing AIs could replay the
  battle ahead of it. Accepted for the alpha.

## Verification

One test for each numbered finding, written as the attack: it succeeds on
the code as it is and is refused after. Each against the real servers over
HTTP, as the existing tests do.

## Must stay green

The multiplayer domain; `online-battle`, `room-lobby`, `frontline`.

## Delegated

Every bound's value, as data beside its owner with a sentence saying what
it protects; the refusals' names.

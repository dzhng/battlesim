# Skirmish setup

The lobby's model: seats, sides, factions and the map choice, and how they
become the request a battle is prepared from. Offline skirmish, an online
room and the server all use it, so a seat means the same thing in each.

- [Seats](src/seats.ts): a seat has an id that is made once and never
  renumbered; its number on screen is its lane, derived from where it sits.
- [The lobby](src/lobby.ts): the presets, every edit a host may make, and
  the one function that turns a lobby into a battle request.

Rosters, map sizes and tunable limits are read from the fixtures, never
restated here.

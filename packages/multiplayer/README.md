# Multiplayer contract

What a page and the game servers say to each other, and the rules of online
play that neither end may decide alone. Both ends import from here, so
nothing here may depend on a browser, a server, or the simulation's binding.

- [The wire](src/wire.ts): every type that crosses the network. Its prose
  twin, with the endpoints, is [wire.md](wire.md); change both
  together.
- [The record stream](src/stream/): one framing for every stream (battle,
  room, list, control), and one rule for resuming after a cut. A reader
  resumes by record number and takes a snapshot only after a real gap. Add a
  record kind here before inventing a second stream format anywhere.
- [A seat's link](src/link.ts): the one shape a seat's transport has, whether
  it runs in the same process or over HTTP. A battle sees a seat's port and
  never a credential.
- [The room model](src/room.ts) and [Frontline placement](src/frontline.ts):
  pure rules. The room model also owns shareable code normalisation and defers
  seed admission to the map owner. A server holds the state and applies them; a page only shows
  what the server last sent.
- [Setup](src/setup.ts): how a lobby becomes the settings a ticket carries.
  Both ends build a battle from those settings with one function, which is
  why a page can check its map against the server's.
- [Build admission](src/build.ts) compares the three shared hashes. The
  [build-time owner](buildIdentity.ts), outside browser-neutral `src/`, defines
  the simulation, battle-document and wire inputs for both build tools. A
  source commit is provenance, not compatibility.
- [Callsigns](src/callsign.ts): one rule for what a name may be, for page
  and server.

A browser id and a battle's key are credentials. Nothing in this package
puts one in a type that is sent to another player.

[Browser identity](src/identity.ts) owns credential validation and its header.
Diagnostics scrub credential-shaped text through that same owner; request
refusals carry public codes, never arbitrary request text.
[Publication payloads](src/stream/publication.ts) have one encoder and decoder,
so runtime, page and measurement readers agree on the bytes inside a record.

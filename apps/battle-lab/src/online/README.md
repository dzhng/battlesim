# Online menu pages

The online services own activity, rooms, seats and settings. Pages retain only the
latest view and wait for the stream after sending a request. A request's success
never predicts the next view.

[stateStream](stateStream.ts) uses the shared record reader and resume rule.
The HTTP byte-reading boundary lives in
[the multiplayer package](../../../../packages/multiplayer/src/stream/http.ts),
which battle links use too. A normal refusal stops reading; a dropped connection
resumes. Credentials travel in headers only. Room links contain their code.

The editable callsign is a base name owned by [identity](identity.ts). The
header shows the name returned by `/hello`, including its server-assigned suffix.

[The room driver](roomDriver.ts) feeds the same lobby view as offline skirmish.
Its lobby setup is a transient projection of the latest `RoomView`, never
editable local room state. The local driver alone owns a mutable lobby and its
seat counter. Start is enabled solely by the server's `canStart`. While the server prepares
the battlefield the room says so; its battle view supplies the ticket and the
driver follows it into the battle route. History state holds the ticket and
server address and room code, while the room link can recover them after a page
is reopened. The battle page keeps a state stream open through local preparation and
renderer admission, so loading is presence even between the battle description
and running stream. Rooms use their room state; Frontline uses the shared list.
The last battle observation owns the result after the server frees its room:
online player rows use the ticket's public callsigns and return to Multiplayer.
A battle opened without a ticket is refused.

The [seat banner](../lobby/Seats.tsx) puts the host's move action inside its
occupancy picker so it takes a deliberate choice to leave the current seat.
Guests can take an open seat directly from its banner.

[Room previews](roomPreview.ts) prepare the exact server seed through the existing
map-preview controller, with one admission attempt. A rejected seed stays
rejected for every viewer until the host changes it. The preparation path needs
a fighter on each side, so an unfinished room says so instead of inventing a
fighter or silently drawing another map. Preview failure does not override the
server's permission to start.

[Frontline](Frontline.tsx) reserves a command while its briefing is visible.
Its activity counts ride the room-list stream, so a page observes one online
state source. The band owns admission progress; the briefing owns map progress,
which must remain visible while preparation runs.

[Online map preparation](useOnlineMap.ts) is shared by rooms and Frontline.
Only an exact match with the server's map admits Deploy. The briefing passes
plain admitted preparation through the ticket navigation, avoiding a second
map build; workers and credentials never enter history state. The shared
battle view keeps its loading cover until the first authoritative snapshot
and first drawn frame, including on a slow link.

Backing out returns the reservation; a chosen battle departure returns control
to AI, in a room as in Frontline, and tells the lobby so. Returning from a
finished battle tells it nothing. The room list says which started room is the
reader's own, and the page offers the way back to it through that room's link. An idle stream returns to the Frontline band with its reason. Losing a
connection or reloading keeps the server's return grace: Play asks placement
to resume the command. No client-side seat or credit accounting substitutes
for that decision.

[Online session navigation](../onlineSession.ts) owns return notices and chosen
battle departure. History carries public tickets and notices only. Admission
failures distinguish build incompatibility from an unreachable lobby without
showing network diagnostics to the player.

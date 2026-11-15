# Online menu pages

The room service owns the list, seats and settings. Pages retain only the
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
is reopened. The battle page keeps the room stream open during local map
verification, so loading is presence even before it can open a battle stream.
The last battle observation owns the result after the server frees its room:
online player rows use the ticket's public callsigns and return to Multiplayer.
A battle opened without a ticket is refused.

[Room previews](roomPreview.ts) prepare the exact server seed through the existing
map-preview controller, with one admission attempt. A rejected seed stays
rejected for every viewer until the host changes it. The preparation path needs
a fighter on each side, so an unfinished room says so instead of inventing a
fighter or silently drawing another map. Preview failure does not override the
server's permission to start.

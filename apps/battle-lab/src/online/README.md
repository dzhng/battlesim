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

# 00 — Jet and air-alarm source provenance

**Contract:** freeze the source and processing receipt for jet approach, air
alarm, missile warning, flare, and AA launch sounds before gameplay assigns
them. The existing battle-audio catalog remains the owner.

**Seam:** `fixtures/sounds.json`, `assets/third-party/audio/`,
`assets/runtime/audio/`, and `packages/battle-audio/tools/assets.py`. Each
YouTube source records URL, attribution, rights/licence, hash, frame range,
and processing profile. A generated placeholder recipe keeps tests runnable
until a clip is accepted; downloaded bytes never become an unproven gameplay
event.

**Runnable proof:** an offline catalog/asset-tool fixture that prepares one jet
loop and one alarm cue, and auditions them in the sound workbench.

**Checks:** source receipt and hash validation, deterministic preparation,
loudness calibration, no duplicate recipe assignment, and side-local cue
inputs. Do not claim a clip is licensed until its receipt says so.

**Scope firewall:** no simulation, balance, unit admission, or audio-trigger
logic. Source selection may be delegated; provenance and processing are not.

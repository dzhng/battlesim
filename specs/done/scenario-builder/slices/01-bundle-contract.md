# 01 — Canonical self-contained scenario bundle

## Contract unlocked

Define one strict, versioned `ScenarioBundle` that is portable across Download/Upload and contains everything required to admit a resolved battle without repository fixture or generator lookups.

## Seam

The contract owner should live beside `crates/contract/src/scenario.rs` or in a clearly named contract module, with one browser codec at the file boundary. It contains:

- canonical resolved map bytes plus physical/template identity;
- resolved rules/catalog data needed by the scenario;
- player slots, side ids, faction, requested controller, and unit ownership;
- unit setup/conditions, common events, mode, and victory configuration;
- authored script source/compiled identity as accepted by slice 05;
- built-in asset content references and embedded user asset records;
- format/build/runtime identities and size limits.

Unknown fields, duplicate ids, non-canonical ordering, invalid hashes, missing bytes, unsupported content, and topology violations refuse atomically with structured diagnostics. The canonical bytes and asset hashes feed scenario/config identity. Do not include filesystem paths or generator requests as runtime requirements.

## Container choice

Run a small size/admission probe. Use canonical JSON with embedded base64/blob records if it stays within measured startup and size limits; choose a deterministic binary container only if the probe demonstrates that JSON/base64 is materially unsuitable. Whichever wins is one owner, has a strict manifest, and round-trips byte-for-byte.

## Runnable artifact

A native/WASM validator/probe accepts a minimal bundle, prints canonical identity and diagnostics, and emits a bundle that the browser can download and re-upload. It must include a tiny user audio byte fixture even if playback waits for slice 02.

## Verification

- semantic round trip preserves canonical bytes and identity;
- asset bytes, media type, length, and SHA-256 are checked;
- changed bytes/order/source changes identity deterministically;
- malformed, oversized, duplicate, unknown, and missing-member bundles refuse;
- native and WASM validators agree;
- no IndexedDB/localStorage or fixture publication is called.

## Firewalls

Do not add campaign import, codebase fixture writes, migration readers, or a second scenario JSON schema. Do not make the browser re-derive map geometry or catalog rules.

## Delegated implementation choices

Internal serialization helpers, hash implementation calls, and the exact binary versus base64 representation follow the measured probe. The semantic fields, strictness, identity participation, and refusal behavior are fixed here.


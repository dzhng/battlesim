# Air-war choices

This ledger records the decisions closed in the 2026-10-12–13 interview. The
implementation inherits them; it does not reopen them silently.

| Decision | Choice | Why |
|---|---|---|
| Aircraft lifecycle | Persistent on-map HighAir aircraft | Supports circling, attack-move, dogfights, fuel, and visible ingress. |
| Air layers | Helicopters LowAir; fixed-wing HighAir at ~1,000 m AGL | Keeps the existing helicopter contract and separates jet/heli sensing. |
| Ground vision | Aircraft never reveal Ground or clear Ground fog | Air observation stays an air-layer capability. |
| AA coverage | Every faction gets short, medium, and long AA; add NASAMS/Patriot/IRIS-T/S-300 family rows | Prevents a structurally incomplete air counter roster. |
| Short AA | Include ballistic AA that also targets Ground at reduced accuracy | Gives short AA a visible gun role and missile-interception role. |
| Interception | Authored per weapon; medium defaults false; Stinger/Igla false | Capabilities belong to weapon data, not platform-name branches. |
| Radar | Always-on 360° unobstructed signature; SEAD-only detection | Simple, readable counterplay and no radar LOS implementation. |
| Radar contact | Approximate area plus bearing/range, labelled radar | Preserves fog while making the source actionable. |
| SEAD | EA-18G, Tornado ECR, Su-34 variant; longest range; radar targets only | Gives every faction a dedicated anti-radar identity. |
| Flares | Authored `decoyable`; automatic on seeker lock | Reuses a discrete seeker event and avoids terminal prediction machinery. |
| Dogfights | Arbitrary groups; explicit orders or range break; adjacent groups need not merge | Emergent joining is sufficient and avoids group special cases. |
| Ingress | Nearest friendly edge to ordinary placement ghost; seeded 15–30 s delay | Keeps purchase UX familiar while making arrival physical. |
| Fuel | Starts at map entry; ~3 minutes; afterburner explicit, +50% speed/2x burn | Off-map delay does not consume the player's combat resource. |
| Withdrawal | Health/ammo/fuel-weighted refund | Makes fuel a real resource without making a safe return all-or-nothing. |
| Aircraft markers | No ground circles; compact card on traversal ghost while Space is held | Avoids helicopter ground-marker language and keeps state legible. |
| Ground AA LOS | Buildings and trees obstruct fire; forest hides but cannot fire through | The viewer expects a concealed AA position to be unable to shoot through its cover. |
| Audio | Air alarm, approach-scaled jet sound, missile/flare/AA cues; YouTube sources with receipts | Aircraft must feel like a major battlefield event and assets must remain reproducible. |
| Compatibility | Hard cutover, no migration or adapter | The old sortie contract is unshipped continuation scope, not a supported consumer. |

## Implementation-pass choices

### Sound

- **When:** HighAir contract pass.
- **The choice:** An existing `Air` mobility row with no layer field resolves to
  LowAir. In plain terms, old helicopter rows keep flying in their existing
  band while a new fixed-wing row opts into HighAir explicitly; the alternative
  would be to reject every old row or silently move helicopters upward.
- **The gap:** The new layer was required, but the existing authored helicopter
  rows did not carry a layer field yet.
- **The reach:** Every later aircraft profile can choose its layer without a
  second mobility type, and the shipped helicopter roster remains valid until
  its rows are deliberately rewritten.
- **Verdict:** sound — the default preserves the existing physical contract
  while making HighAir opt-in. **Confidence:** high.

- **When:** HighAir contract pass.
- **The choice:** HighAir contacts use the existing generic air contact bark.
  A player hearing a jet or helicopter contact receives the same broad “air”
  callout until a later audio pass adds more specific copy; the alternative
  would be a new shout taxonomy before aircraft identity exists in the catalog.
- **The gap:** The new enum made contact-trigger matching exhaustive, but the
  feature did not specify a new voice line category.
- **The reach:** HighAir remains compatible with the current voice catalog and
  side-filtered shout publication; slice 11 can add jet-specific sounds without
  changing simulation targeting.
- **Verdict:** sound — it preserves the existing observable contract and keeps
  voice taxonomy out of the layer cutover. **Confidence:** medium.

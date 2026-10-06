# Ground model family handoff

These are worker inputs for slice 03, not a second runtime physics catalog. The
coordinator adopts each chosen frame into the resolved fixture before dispatch;
that resolved catalog becomes authoritative. A worker compares its manifest to
that catalog and stops on any disagreement. Never resize physics, move a muzzle,
widen tolerances, or edit shared rig/helpers merely to pass.

Status, 2026-10-06: 33 hull-family assignments and one infantry/equipment cohort
prepared. Supported ground rows now adopt these frames; named model production is active.
The three resupply family exports await coordinator runtime integration. Family envelope
references are captured below; mount and eye coordinates are **approximate chosen
authoring frames**, not measured factory drawings. `authoring-frame-frozen` means
a fixed proposal awaiting matching fixture adoption, not proven asset admission.
`research-gated` means missing or ambiguous physical evidence still prevents
dispatch. Variant-specific reference review and in-renderer acceptance remain
required even where a base-family envelope is available.

## Dispatch order and ownership

Start Bradley, Stryker and ordinary Abrams, then HEMTT/MAN HX/Ural; refill with
light/recon families and the other tank/IFV families. One coordinator and at most
three workers. Shared platform identities have one worker, irrespective of faction
memberships. A worker owns only the script and export directory named in its JSON.
A large family can be split at variant boundaries only after the coordinator
assigns exclusive files; shared helpers remain coordinator-owned, except this worker cohort exclusively owns
`packages/scene-assets/blender/roster/logistics.py` for HEMTT/MAN HX/Ural.
The coordinator authorized that local helper to avoid duplicated chassis/detail
construction; other family workers must not edit it.

The coordinator alone edits fixture units/weapons/service/parts, `assets/catalog.json`,
runtime bake output, icon/binding publication and shared exporter helpers. Each
variant has its own appearance identity. Shared geometry/components are permitted;
a generic tank/jeep painted differently is not a finished named platform. Infantry
keep shared body/rig/clip owners and reuse equipment where identities genuinely match.

## Engine and mount contract

Metres, +X forward, +Y left, +Z up; hull origin at ground under the body center.
Dimensions describe the **body**, excluding a gun's forward reach. `half_extents_m`
is the selected physical envelope, not a rescaling instruction. All moving-node
scales remain unit scale. Export glTF normally; bake owns Y-up conversion.
Vehicle appearances use `basis_yaw_deg: 0`. Infantry imported rest space faces -Y
and left +X; its existing `basis_yaw_deg: 90` converts once at bake.

Read [the contract mount definition](../../../crates/contract/src/weapons.rs) and
[the one muzzle evaluator](../../../packages/scene-assets/src/mountMuzzle.ts).
A mount `pivot_m` is in the carrier bearing frame **from hull origin**; its muzzle
is an offset rotated by its own bearing. `on` names an earlier turret mount,
not a transform whose translation gets added a second time. Author actual node
parents to reproduce these world pivots when both mounts rotate. For a child
node under a translated turret, its local translation subtracts the parent's
bind pivot; do not blindly copy an absolute `pivot_m` as local translation.

Existing role `gun` requires `turret -> gun -> muzzle`; `hmg` requires
`hmg -> hmg_gun -> hmg_muzzle`. Node names are rig vocabulary, so `hmg` may draw
a missile launcher; the simulation mount name and weapon identity still say
launcher. Turret yaw is +Z, pitch +Y, recoil -X; wheels roll +Y. Preserve current
presentation limits from `articulation.ts`: gun -10/+20 degrees, hmg -10/+45.
They are presentation bounds, not sourced mechanical performance claims.
Every simulation mount gets its own declared rig and muzzle. A real co-bore
ammunition choice may share one mount; physically separate guns may not.

Tracked hulls require `track_L` and `track_R`, each with positive
`track_length_m` and `link_pitch_m`. `wheel_*` nodes carry `radius_m`; wheeled
front (`wheel_F*`) pivots lie ahead of rear (`wheel_R*`). Model correct axle and
road-wheel counts from reference, rather than resizing the generic running gear.

## Frozen decisions and dependencies

- Stationary supply service needs no fictional visual deployment hardware.
  HEMTT keeps its actual folded crane as static geometry; MAN HX/Ural remain cargo
  trucks. The coordinator makes deployment-art admission optional/data-driven,
  retaining motion checks for assets that actually declare deployment motion.
- BMP-3 uses a 100 mm 2A70 gun with **gun-launched Bastion**, plus its distinct
  30 mm 2A72. This corrects the planning table's Kornet shorthand. Do not bolt
  invented Kornet boxes onto ordinary BMP-3. The existing second `hmg` rig draws the30mm barrel; no third-rig extension is needed.
- Trophy hardware is static paired radar/launcher geometry under the appropriate
  turret. Use `trophy_radar_L/R` and `trophy_launcher_L/R` for the eventual part-node
  contract. Trophy is not an offensive articulated fire mount. The coordinator
  freezes its part/capability binding; those variants remain unavailable until
  the behavior slice admits them.
- No passengers, functioning amphibious propulsion, top-attack/fire-and-forget,
  air targets, drone launchers or unlisted APS follows from a real platform's art.
  Jaguar/Akeron/Javelin art never enables their deferred capability by itself.
- BRM-3K, T-14, Type 15 and T-15 retain explicit physical research gates. Other
  base-family approximations and unresolved enhanced variants are identified in
  each JSON, never presented as exact variant measurements.
- TOW/Kornet infantry require real tripod/tube/optics active and carried kits.
  Existing shoulder-launcher animation is not proof of those holds; freeze their
  rig/socket fit before dispatch. Large sniper kits also need actual grip/optic
  fit while preserving shared joints and clips.

## Assets and tooling

Use reproducible Blender family scripts importing existing `parts.py`,
`textures.py`, `common.py`, and mesh-tier/export helpers. Reuse component recipes,
not the generic complete vehicle outline. Split variant hardware in the family
script, export each variant independently, and return a variant-node manifest.

Pinned Blender **5.2.1 LTS**, build `9e2066aef7ef`, is already installed at:

```
/Users/server/dev/battlesim/throwaway/blender/Blender.app/Contents/MacOS/Blender
```

The default `/Applications/Blender.app` is absent. Set `BLENDER` to the existing
executable when invoking the asset CLI. No alternate renderer or dependency install
is needed for hull scripts. Infantry additionally verifies cached, hash-pinned
Quaternius inputs using `packs.py` and `packs.json`; missing cache is explicit
acquisition work, not permission to replace the rig. Worker build output and
scratch live in ignored `throwaway/`; share installed dependencies, not build output
between differing checkouts. Reference access failures are recorded in scratch.

## Reference confidence and receipts

Each JSON pins its fetched reference URL and SHA256, relevant dimensions and
reference image URLs where available. Pages/images are inspection references,
not third-party geometry copied into the source. Check the image's original file
page/provenance before reproducing marks or importing any media. Most dimensions
are secondary Wikipedia family references; manufacturer/Army sites often denied
fetch. This is openly lower confidence than a variant manufacturer drawing.
The primary Rheinmetall Boxer CRV brochure was retrieved and pinned in `boxer.json`;
it informs proportions but does not equate CRV with APC or RCT30.

Fetched bytes, extraction scripts, failed-fetch receipts and candidate imagery
are in `throwaway/unit-roster-references/`. They are scratch, not a production
asset archive. Preserve URL/hash and the actual facts needed for the worker in
its manifest; refetch by pinned identity or have the coordinator provide captured
bytes at dispatch. A page's thumbnail alone is not complete front/side/rear variant
coverage. Inspect subtype photos before accepting exact equipment claims.

## Existing gates, unchanged

Use the live schema/validator: four genuine mesh tiers; texture power-of-two/mips
and `TEXTURE_MAX_PX`; required node hierarchy; unit-scale basis; physical hull,
eye and muzzle fit including yaw sweeps; part nodes; shared skeleton/joint/clip
and socket validation. The catalog currently owns fit tolerances; no worker may
change them. Unit-specific numeric budgets not present in the current gate must
not be invented or inferred from scenery/kit caps. Asset download/residency remains
an integration gate; do not fetch future disabled models just to populate cards.

Return reproducible source + GLB + provenance + node/variant manifest + local
validation evidence. Coordinator runs bake/check, derives icons, tests mount fit
and observes family silhouettes in the actual renderer. Compare reference crops
and existing before/after shots, run an unprimed screenshot critique last, and show
shots in Preview. A Blender beauty render does not establish game readability.

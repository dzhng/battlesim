//! The layout every client reads a publication by: its header, groups and
//! fields, and what each field means.
use super::*;
use contract::observation::WeaponPose;

/// The layout of every publication in `battle`: its round kinds are the
/// rules' weapon rows (in name order), its ground grid the ground layer's.
pub fn layout_json(battle: &Battle) -> String {
    let round_kinds: Vec<&str> = battle
        .arsenal()
        .weapons
        .iter()
        .map(|w| w.id.as_str())
        .collect();
    let ground = battle.ground();
    let groups: Vec<_> = GROUP_LAYOUTS
        .iter()
        .map(|g| {
            let sections: Vec<_> = g
                .sections
                .iter()
                .map(|s| serde_json::json!({ "name": s.name, "count": s.count, "fields": s.fields }))
                .collect();
            serde_json::json!({ "name": g.name, "count": g.count, "fields": g.fields, "sections": sections })
        })
        .collect();
    let mut player_ids = battle
        .observe(Side::Blue)
        .skirmish
        .as_ref()
        .into_iter()
        .flat_map(|s| s.players.iter().map(|p| (p.index.0, p.id.clone())))
        .chain(
            battle
                .observe(Side::Red)
                .skirmish
                .as_ref()
                .into_iter()
                .flat_map(|s| s.players.iter().map(|p| (p.index.0, p.id.clone()))),
        )
        .collect::<Vec<_>>();
    player_ids.sort_by_key(|(index, _)| *index);
    player_ids.dedup_by_key(|(index, _)| *index);
    let player_ids: Vec<_> = player_ids.into_iter().map(|(_, id)| id).collect();
    serde_json::json!({
        "header": HEADER.as_slice(),
        "groups": groups,
        "groupDelivery": { "fields": DELIVERY_FIELDS, "range": RANGE_FIELDS, "copy": COPY_FIELDS, "copyAlignments": (0..GROUPS).map(|g| fixed_row_width(g).max(1)).collect::<Vec<_>>(), "encodings": GROUP_ENCODINGS,
            "packed": {
                "bitOrder": "lsb-first in raw u32 carrier words",
                "form": "u8: replacement=0, snapshot=1, copies=2",
                "integer": "canonical unsigned LEB128, at most four bytes",
                "replacement": "operation count, then start/length/literals",
                "copies": "source+1/length; source zero means literals",
                "literal": "u4 tag: 0..4 raw byte count; 5..9 baseline-XOR byte count",
                "snapshot": "raw literals only; no baseline predictor",
                "padding": "zero bits to the next u32 carrier boundary"
            }
        },
        "fog": { "count": "fogFloats", "maxWords": MAX_FOG_WORDS },
        // A run is one 16×16 tile and start + len * 256 within it;
        // craterScorch is crater + scorch * 256; tracksTrampledCleared is tracks + trampled * 256 + cleared * 65536.
        "ground": {
            "count": "groundRunCount",
            "fields": GROUND_FIELDS,
            "packed": {
                "bitOrder": "lsb-first in raw u32 carrier words",
                "tile": "delta from prior tile (initially zero); canonical unsigned LEB128, at most four bytes",
                "span": "u8 start, u8 length-minus-one",
                "marks": "u5 presence mask in crater/scorch/tracks/trampled/cleared order; u8 nonzero values for set bits",
                "padding": "zero bits to the next u32 carrier boundary",
                "count": "groundRunCount canonical four-word rows; final record tail"
            },
            "tileSize": 16,
            "maxRecordBytes": MAX_PUBLICATION_BYTES,
            "cellM": ground.cell_m(),
            "cols": ground.cols(),
            "rows": ground.rows(),
            "sides": names(&Side::ALL),
        },
        "limbBits": LIMB_BITS,
        "roundKinds": round_kinds,
        "hitKinds": names(&SEGMENT_HITS),
        "unitKinds": battle.rules().catalog.ids(),
        "playerIds": player_ids,
        "controllers": names(&[ControllerKind::Human, ControllerKind::ActiveAi, ControllerKind::PassiveAi]),
        "moveStates": names(&MOVE_STATES),
        "policies": names(&POLICIES),
        "directions": names(&DIRECTIONS),
        "contactSources": names(&CONTACT_SOURCES),
        "layers": names(&LAYERS),
        "airTransitDirections": names(&AIR_TRANSIT_DIRECTIONS),
        "soundCategories": names(&SOUND_CATEGORIES),
        "soundBands": names(&SOUND_BANDS),
        "shoutTriggers": names(&ShoutTrigger::ALL),
        "intensities": names(&INTENSITIES),
        "propKinds": battle.world().types().ids(),
        "engagements": names(&ENGAGEMENTS),
        "actionReasons": names(&REASONS),
        "targetKinds": TARGET_KINDS,
        "postures": names(&POSTURES),
        "coverTiers": names(&COVER_TIERS),
        "leanSides": names(&LEAN_SIDES),
        "hostKinds": HOST_KINDS,
        "holdPhases": names(&HOLD_PHASES),
        "orderErrors": ORDER_ERROR_NAMES,
        "serviceStatuses": names(&SERVICE_STATUSES),
        "suppressionTiers": names(&SUPPRESSION_TIERS),
        "encounterResults": names(&ENCOUNTER_RESULTS),
        "objectiveIds": battle.observe(Side::Blue).skirmish.as_ref().map_or_else(Vec::new, |s| s.objectives.iter().map(|o| o.id.clone()).collect()),
        // Mount ammo is rounds left per kind: -1 unlimited, -2 no such kind.
        // goalX/goalY are NaN without a movement order; policy, direction and blocker are -1 when absent.
        // reversing is 1 while the unit drives backwards this tick, else 0.
        // smoking is 1 while it trails smoke (an aircraft below half its
        // health), own or identified alike, else 0.
        // finalFacing is the bearing the unit ends its move at (its yaw
        // without one). A member order is the soldier's spot (his post while
        // holding, where he stands without either); coverNow and coverThere
        // index coverTiers, -1 for none.
        // A member lean is a soldier out past his cover's edge while he fires:
        // side indexes leanSides and x, y is where his body stands meanwhile
        // (his members position stays where he tucks in); side -1 and x, y
        // NaN while he is tucked in. areaX, areaY and areaM are a squad's
        // anchor and area radius, NaN for a vehicle.
        // deployProgress and deployTarget are -1 for units that never deploy.
        // hostKind, both host limbs, holdPhase and holdProgress are -1 without
        // a hold. hostX, hostY, hostHalfX and hostHalfY carry a building
        // footprint and are NaN for a carrier or no hold.
        // Known body IDs and associations use exact limbs; absent pairs are (-1,-1).
        // sightForward is the bearing sight looks along at this tick (never
        // interpolated); reach toward bearing b is sightRange * m, with
        // c = cos(b - sightForward), m = side·(1 − c²) + (c ≥ 0 ? front : rear)·c².
        // A contact's kind indexes unitKinds (-1 for a firing report); its
        // heardLow, heardMid and heardHigh are a bitmask over roundKinds, 24
        // rows a word (bit k of heardLow for row k < 24, bit k - 24 of
        // heardMid for k < 48, bit k - 48 of heardHigh otherwise; all 0 for a
        // last sighting). Its z is its cause's height when the evidence came, and
        // its layer indexes layers; aloft is 1 when the area hangs in the air
        // (more than the low hover over the ground), else 0.
        // A projectile's or blast's kind indexes roundKinds; a segment's
        // shooter is absent (-1) for a vehicle's gun, and nx, ny, nz are 0
        // when hit is none. A segment's path is a polyline of at least two
        // points; each ricochet names the path point where the round glanced
        // off a hull, with the outward normal there, and hit is at the path's
        // last point. A pose's shots rise by one per round launched.
        // A crash is a downed airframe still falling: its id is the side's
        // own unit id when own is 1, else the handle it was identified by;
        // its kind indexes unitKinds; x, y, z is its foot, and yaw, pitch
        // (nose up) and roll (right side down) its attitude, in radians.
        // A shout is a bark this side hears: own is 1 for its own unit (id
        // its unit id) and 0 for an identified enemy (id the handle it was
        // identified by); kind indexes unitKinds, x, y, z is where it was
        // said, trigger indexes shoutTriggers and intensity intensities. An
        // unseen enemy's shout is never one: it is an audible shout cue.
        // A prop display pose is a body shoved this tick that this side sees:
        // propId its known prop's id, x, y, yaw and baseZ where it stands.
    })
    .to_string()
}

pub(super) fn pose(p: &WeaponPose) -> [f32; 8] {
    let [lo, hi] = limbs(p.shots);
    let [operator_lo, operator_hi] = limbs_or_absent(p.operator);
    [
        p.mount as f32,
        p.bearing as f32,
        p.elevation as f32,
        p.round as f32,
        lo,
        hi,
        operator_lo,
        operator_hi,
    ]
}

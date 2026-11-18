//! The named errors a unit type or a soldier kind that breaks its own
//! structure fails at load with.
use super::*;

/// A type's structure the simulation relies on is checked at load, naming
/// the entry: where each mount sits, a wreck's cover, and every number in
/// its range.
#[test]
fn a_structurally_broken_type_fails_at_load_naming_it() {
    let rule = |error: &str| CatalogError::Rule {
        id: "t".into(),
        error: error.into(),
    };
    let tank = |patch: Value| {
        let mut t = base_tank();
        t.as_object_mut().unwrap().remove("abstract");
        contract::catalog::merge(&mut t, &patch);
        error(json!({ "t": t }))
    };
    // The HMG rides a turret no earlier mount is.
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"][1]["on"] = json!("roof");
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"HMG\" is on \"roof\", which is not an earlier turret mount")
    );
    // A gun on the roof gun that rides the turret: one level of carriage only.
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    let mut stacked = t["mounts"][1].clone();
    stacked["id"] = json!("stacked");
    stacked["on"] = json!("HMG");
    t["mounts"].as_array_mut().unwrap().push(stacked);
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"stacked\" is on \"HMG\", which is itself carried by a turret")
    );
    // An arc limits a turret's swing off the hull's heading: on a gun fixed
    // in the hull, or on one another turret carries, it means nothing, and
    // it must leave the gun some swing and not all of it.
    for (turret, on) in [(json!(false), Value::Null), (json!(true), json!("cannon"))] {
        let mut t = base_tank();
        t.as_object_mut().unwrap().remove("abstract");
        t["mounts"][1]["turret"] = turret;
        t["mounts"][1]["on"] = on;
        t["mounts"][1]["arc"] = json!({ "centre_deg": 90, "width_deg": 160 });
        assert_eq!(
            error(json!({ "t": t })),
            rule("mount \"HMG\" has an arc but no turret on the hull to swing through it")
        );
    }
    for width in [0.0, 360.0] {
        let mut t = base_tank();
        t.as_object_mut().unwrap().remove("abstract");
        t["mounts"][1]["on"] = Value::Null;
        t["mounts"][1]["arc"] = json!({ "centre_deg": 90, "width_deg": width });
        assert_eq!(
            error(json!({ "t": t })),
            rule("mount \"HMG\" arc width must lie between 0 and 360 degrees")
        );
    }
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"].as_array_mut().unwrap().reverse();
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"HMG\" is on \"cannon\", which is not an earlier turret mount")
    );
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"][0].as_object_mut().unwrap().remove("muzzle_m");
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"cannon\" on a hull needs muzzle_m")
    );
    // Hull weapons have no infantry operator to wear equipment.
    let mut t = base_tank();
    t.as_object_mut().unwrap().remove("abstract");
    t["mounts"][0]["operator_appearance"] = json!({"active":["launcher"],"carried":["carried"]});
    assert_eq!(
        error(json!({ "t": t })),
        rule("mount \"cannon\" on a hull cannot name operator_appearance")
    );
    // Its wreck keeps its cover tier (Q24): a heavy hull's wreck is heavy cover.
    let light = json!({ "body": { "hull": { "wreck": "light_wreck" } } });
    assert_eq!(
        tank(light),
        rule("its wreck \"light_wreck\" gives light cover, not its heavy weight's heavy (Q24)")
    );
    let stopped = json!({ "mobility": { "tracked": { "turn_deg_s": 0 } } });
    assert_eq!(tank(stopped), rule("mobility: turn_deg_s must be positive"));
    let hollow = json!({ "body": { "hull": { "hp": 0 } } });
    assert_eq!(
        tank(hollow),
        rule("a hull needs finite positive extents and hp")
    );
}

/// Infantry cannot have turret chains or a shared squad bore; a single
/// operator may declare a physical mount offset.
#[test]
fn a_broken_soldier_kind_fails_at_load_naming_it() {
    let with = |soldier: Value| {
        let mut docs = units(json!({}));
        docs.push(json!({ "soldiers": { "s": soldier } }));
        resolve(&docs).unwrap_err()
    };
    let invalid = |error: &str| CatalogError::Invalid {
        section: "soldiers",
        id: "s".into(),
        error: error.into(),
    };
    let soldier = |mount: Value| json!({ "name": "S", "description": "", "hp": 100, "appearance": ["s"], "mounts": [mount] });
    assert_eq!(
        with(soldier(
            json!({ "id": "gun", "name": "Gun", "weapons": ["rifle"], "squad": true, "special": true })
        )),
        invalid("mount \"gun\" is a squad weapon or a special one, not both")
    );
    for placed in [
        json!({ "turret": true }),
        json!({ "on": "gun" }),
        json!({ "pivot_m": [0, 0, 1] }),
        json!({ "squad": true, "muzzle_m": [1, 0, 0] }),
    ] {
        let mut mount = json!({ "id": "gun", "name": "Gun", "weapons": ["rifle"] });
        contract::catalog::merge(&mut mount, &placed);
        assert_eq!(
            with(soldier(mount)),
            invalid("mount \"gun\": infantry cannot use turret/on, share bore offsets, or declare a pivot without a muzzle"),
        );
    }
    let mut dead = soldier(json!({ "id": "gun", "name": "Gun", "weapons": ["rifle"] }));
    dead["hp"] = json!(0);
    assert_eq!(with(dead), invalid("hp must be positive"));
}

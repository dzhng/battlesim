//! Document admission at the same boundary used by native and browser clients.
use presentation::light::LightSettings;
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(format!("../../fixtures/{name}.json")).unwrap()).unwrap()
}

#[test]
fn misspelled_light_field_is_refused() {
    let mut light = fixture("presentation")["light"].clone();
    light["sky"]["clouds"]["softnes"] = json!(0.1);
    let result = serde_json::from_value::<LightSettings>(light);
    assert!(result.is_err(), "a misspelled light key must fail at load");
}

#[test]
fn shipped_documents_load_through_client_readers() {
    presentation::settings::PresentationSettings::from_value(&fixture("presentation")).unwrap();
    presentation::sound::tables::SoundTables::from_parts(
        fixture("audio"),
        fixture("sounds"),
        &fixture("presentation"),
        &fixture("rules")["weapons"],
    )
    .unwrap();
}

fn reject_presentation(pointer: &str, value: Value, path: &str) {
    let mut doc = fixture("presentation");
    *doc.pointer_mut(pointer).unwrap() = value;
    let error = presentation::settings::PresentationSettings::from_value(&doc)
        .err()
        .expect("invalid presentation must be refused");
    assert!(error.contains(path), "{pointer}: {error}");
}

#[test]
fn movement_effect_spacing_must_advance_at_load() {
    for effect in ["dust", "wake"] {
        reject_presentation(
            &format!("/effects/{effect}/spacing_m"),
            json!(0),
            &format!("effects.{effect}.spacing_m"),
        );
    }
}

#[test]
fn mist_edges_remain_strictly_increasing_on_the_gpu() {
    for field in ["mist_height_m", "mist_distance_m"] {
        for edges in [[100.0, 100.0], [100.0, 100.000001]] {
            reject_presentation(
                &format!("/light/haze/{field}"),
                json!(edges),
                &format!("light.haze.{field}"),
            );
        }
    }
}

#[test]
fn rest_settle_denominator_must_be_positive() {
    reject_presentation(
        "/pose/rest/settle_s",
        json!([0, 0]),
        "pose.rest.settle_s[1]",
    );
}

#[test]
fn callout_glow_is_refused_at_document_admission() {
    reject_presentation(
        "/overlay/glow/callouts",
        json!(4.1),
        "overlay.glow.callouts",
    );
}

#[test]
fn overlay_constraints_are_owned_by_document_admission() {
    for (pointer, value) in [
        ("/overlay/paint/albedo", 0.0),
        ("/overlay/paint/fog_keep", 1.1),
        ("/overlay/paint/grass_glow", 8.1),
        ("/overlay/paint/grass_falloff_m", 4.1),
        ("/overlay/paint/saturation", 3.1),
        ("/overlay/orders/line_px", 0.0),
        ("/overlay/orders/mark_px", 0.0),
        ("/overlay/orders/soldier_mark_px", 0.0),
        ("/overlay/orders/soldier_line_px", 0.0),
        ("/overlay/orders/cover_pip_m", 0.0),
        ("/overlay/orders/queued_alpha", 0.0),
        ("/overlay/orders/current_alpha", 0.0),
        ("/overlay/orders/march/amplitude", 1.1),
        ("/overlay/orders/area_draw_scale", 0.0),
        ("/overlay/orders/glow/order", 2.1),
        ("/overlay/orders/glow/selected", 0.0),
        ("/overlay/orders/cover_glow", 0.9),
        ("/overlay/orders/vehicle_marker_margin_m", 0.0),
        ("/overlay/orders/building_marker_margin_m", 0.0),
        ("/overlay/orders/drop_line_px", 0.0),
        ("/overlay/orders/drop_line_spacing", 1.0),
        ("/overlay/orders/drop_line_alpha", 0.0),
        ("/overlay/orders/drop_line_reach", 0.0),
        ("/overlay/orders/drop_line_hold", 1.0),
        ("/overlay/ruler/line_px", 0.0),
        ("/overlay/ruler/tick_px", 0.0),
        ("/overlay/ruler/tick_line_px", 0.0),
        ("/overlay/ruler/end_px", 0.0),
        ("/overlay/xray/own/3", 0.0),
        ("/overlay/xray/selected_alpha", 0.0),
        ("/overlay/xray/ally_alpha", 0.0),
    ] {
        reject_presentation(
            pointer,
            json!(value),
            &pointer[1..].replace('/', ".").replace("own.3", "own[3]"),
        );
    }
}

#[test]
fn cook_off_landing_requires_an_existing_look_and_surface() {
    for (key, value, path) in [
        ("as", "unknown-look", "cook_off.landing.as"),
        ("hit", "hulll", "cook_off.landing.hit"),
    ] {
        let mut audio = fixture("audio");
        audio["cook_off"]["landing"][key] = json!(value);
        let result = presentation::sound::tables::SoundTables::from_parts(
            audio,
            fixture("sounds"),
            &fixture("presentation"),
            &fixture("rules")["weapons"],
        );
        assert!(result.expect_err("landing must be admitted").contains(path));
    }
}

#[test]
fn a_default_recording_cannot_invent_a_landing_material() {
    let mut audio = fixture("audio");
    audio["cook_off"]["landing"]["hit"] = json!("hulll");
    audio["weapon_defaults"]["impacts"]["hulll"] = json!("impact_hull");
    let result = presentation::sound::tables::SoundTables::from_parts(
        audio,
        fixture("sounds"),
        &fixture("presentation"),
        &fixture("rules")["weapons"],
    );
    assert!(result
        .expect_err("unknown landing material")
        .contains("cook_off.landing.hit"));
}

#[test]
fn misspellings_in_every_document_section_name_their_path() {
    let doc = fixture("presentation");
    for section in doc.as_object().unwrap().keys() {
        let mut bad = doc.clone();
        bad[section]["misspelled_field"] = json!(false);
        let error = presentation::settings::PresentationSettings::from_value(&bad)
            .err()
            .expect("unknown section field");
        assert!(
            error.contains(&format!("presentation.{section}"))
                && error.contains("misspelled_field"),
            "{error}"
        );
    }
    let doc = fixture("audio");
    for (section, value) in doc.as_object().unwrap() {
        if !value.is_object() {
            continue;
        }
        let mut bad = doc.clone();
        bad[section]["misspelled_field"] = json!(false);
        let error = presentation::sound::tables::SoundTables::from_parts(
            bad,
            fixture("sounds"),
            &fixture("presentation"),
            &fixture("rules")["weapons"],
        )
        .unwrap_err();
        assert!(
            error.contains(&format!("audio.{section}")) && error.contains("misspelled_field"),
            "{error}"
        );
    }
    let doc = fixture("sounds");
    for section in doc.as_object().unwrap().keys() {
        let mut bad = doc.clone();
        bad[section]["misspelled_field"] = json!(false);
        let error = presentation::sound::tables::SoundTables::from_parts(
            fixture("audio"),
            bad,
            &fixture("presentation"),
            &fixture("rules")["weapons"],
        )
        .unwrap_err();
        assert!(
            error.contains(&format!("sounds.{section}")) && error.contains("misspelled_field"),
            "{error}"
        );
    }
}

#[test]
fn nested_misspellings_including_flat_puffs_are_refused() {
    for pointer in [
        "/terrain/water/normals",
        "/light/sky/clouds",
        "/fog/styles/dusk/lines",
        "/pose/stance",
        "/effects/shot_dust/ground",
        "/effects/blast/plume",
        "/overlay/orders/flash",
        "/camera/clearance",
    ] {
        let mut doc = fixture("presentation");
        doc.pointer_mut(pointer).unwrap()["misspelled_field"] = json!(false);
        let error = presentation::settings::PresentationSettings::from_value(&doc)
            .err()
            .unwrap();
        assert!(
            error.contains(&pointer[1..].replace('/', ".")) && error.contains("misspelled_field"),
            "{error}"
        );
    }
    for pointer in [
        "/sources/airwar-aa-launch",
        "/clips/airwar-aa-launch-recording",
        "/sounds/airwar-aa-launch",
        "/provenance/airwar-aa-launch",
        "/music/moods",
    ] {
        let mut doc = fixture("sounds");
        doc.pointer_mut(pointer).unwrap()["misspelled_field"] = json!(false);
        let error = presentation::sound::catalog::SoundCatalog::from_value(&doc).unwrap_err();
        assert!(
            error.contains(&pointer[1..].replace('/', ".")) && error.contains("misspelled_field"),
            "{error}"
        );
    }
}

#[test]
fn pose_denominators_and_light_ranges_are_admitted_before_use() {
    for pointer in [
        "/pose/gait/fade_s",
        "/pose/corpses/fade_s",
        "/pose/lean/out_s",
        "/pose/lean/back_s",
    ] {
        reject_presentation(pointer, json!(0), &pointer[1..].replace('/', "."));
    }
    reject_presentation(
        "/light/sky/clouds/coverage",
        json!(1.1),
        "light.sky.clouds.coverage",
    );
    reject_presentation("/light/exposure", json!(-1), "light.exposure");
}

#[test]
fn unknown_mix_sounds_are_refused_before_preparation() {
    for pointer in [
        "/vehicles/default/engine",
        "/ambience/sound",
        "/weapon_defaults/blast/near",
    ] {
        let mut doc = fixture("audio");
        *doc.pointer_mut(pointer).unwrap() = json!("unknown-sound");
        let error = presentation::sound::tables::SoundTables::from_parts(
            doc,
            fixture("sounds"),
            &fixture("presentation"),
            &fixture("rules")["weapons"],
        )
        .unwrap_err();
        assert!(
            error.contains(&pointer[1..].replace('/', ".")) && error.contains("unknown-sound"),
            "{error}"
        );
    }
}

#[test]
fn looks_refuse_bad_firing_roles_loops_cadence_and_effect_slots() {
    let base = fixture("sounds");
    let look_doc = fixture("presentation");
    let admit = |library: Value, presentation: Value, audio: Value| {
        presentation::sound::tables::SoundTables::from_parts(
            audio,
            library,
            &presentation,
            &fixture("rules")["weapons"],
        )
    };
    let mut bad = base.clone();
    let near = "airwar-aa-launch";
    let clip = bad["sounds"][near]["clips"][0]
        .as_str()
        .unwrap()
        .to_string();
    bad["clips"][&clip]["role"] = json!("impact");
    assert!(admit(bad, look_doc.clone(), fixture("audio"))
        .unwrap_err()
        .contains("firing clips"));
    let bad = base.clone();
    let mut looks = look_doc.clone();
    looks["weapon_looks"]["rifle"]["sound"]["shot"]["near"] = json!("engine_diesel");
    assert!(admit(bad, looks, fixture("audio"))
        .unwrap_err()
        .contains("firing cannot use a loop"));
    let bad = base.clone();
    let mut mix = fixture("audio");
    mix["effects"]["not-a-synth-slot"] = json!("rifle");
    assert!(admit(bad, look_doc.clone(), mix)
        .unwrap_err()
        .contains("effects.not-a-synth-slot"));
    let bad = base.clone();
    // The near burst is a catalog-selected burst; its far partner must cover
    // those same rounds even when the recording differs.
    let burst_sound = bad["sounds"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, r)| {
            r["clips"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| !bad["clips"][c.as_str().unwrap()]["burst"].is_null())
        })
        .unwrap()
        .0
        .clone();
    let mut looks = look_doc.clone();
    looks["weapon_looks"]["rifle"]["sound"]["shot"]["near"] = json!(burst_sound);
    looks["weapon_looks"]["rifle"]["sound"]["shot"]["far"] = json!("rifle");
    assert!(admit(bad, looks, fixture("audio"))
        .unwrap_err()
        .contains("near and far play different bursts"));
    let mut folded = base;
    folded["sounds"]["rifle"]["gain"] = json!(3.0);

    presentation::sound::catalog::SoundCatalog::from_value(&folded).unwrap();
}

#[test]
fn display_alpha_and_geometry_budgets_fail_at_document_load() {
    reject_presentation(
        "/overlay/xray/selected_alpha",
        json!(2),
        "overlay.xray.selected_alpha",
    );
    reject_presentation("/hud/glass", json!([0.1, 0.2, 0.3, 2.0]), "hud.glass");
    reject_presentation(
        "/buildings/tint_jitter",
        json!(0.8),
        "buildings.tint_jitter",
    );
    reject_presentation(
        "/camera/clearance/lift_steps",
        json!(100),
        "camera.clearance.lift_steps",
    );
}

#[test]
fn shader_geometry_cannot_be_overridden_by_a_document() {
    for key in ["radial_bins", "tile_px"] {
        let mut doc = fixture("presentation");
        doc["fog_geometry"][key] = json!(8);
        let error = presentation::settings::PresentationSettings::from_value(&doc)
            .err()
            .expect("shader geometry is fixed by the CPU/GPU contract");
        assert!(
            error.contains("fog_geometry") && error.contains(key),
            "{error}"
        );
    }
}

#[test]
fn shared_ground_numbers_cannot_be_overridden_by_a_document() {
    for (section, key) in [
        ("forest_verge", "feather"),
        ("bank", "ease_m"),
        ("marks", "clear_m"),
        ("scars", "depth_cap"),
    ] {
        let mut doc = fixture("presentation");
        doc["terrain"][section] = json!({key: 0.5});
        let error = presentation::settings::PresentationSettings::from_value(&doc)
            .err()
            .expect("ground/shader constants have no fixture owner");
        assert!(
            error.contains("terrain") && error.contains(section),
            "{error}"
        );
    }
    let mut doc = fixture("presentation");
    doc["grass"] = json!({"edge_m": 0.5});
    assert!(
        presentation::settings::PresentationSettings::from_value(&doc)
            .err()
            .unwrap()
            .contains("grass")
    );
}

#[test]
fn fog_tiles_cover_partial_screen_edges_and_reuse_their_camera_layout() {
    use presentation::fog::sight::{FogSight, TILE_PX};
    use presentation::records::Camera;
    let settings =
        presentation::settings::PresentationSettings::from_value(&fixture("presentation")).unwrap();
    let mut sight = FogSight::new(settings.fog_geometry).unwrap();
    let width = TILE_PX * 3 + 1;
    let height = TILE_PX * 2 - 1;
    let upload = sight
        .prepare(Some(&Camera::default()), width, height, 0)
        .unwrap();
    for (tiles, pixels) in [(upload.tiles[0], width), (upload.tiles[1], height)] {
        assert!(tiles * TILE_PX >= pixels, "the last partial tile is drawn");
        assert!((tiles - 1) * TILE_PX < pixels, "no empty tile is submitted");
    }
    let probes = sight.prepare(None, 0, 0, 1).unwrap();
    assert_eq!(probes.params.tiles_x, upload.tiles[0]);
    assert_eq!(probes.params.probe_count, 1);
}

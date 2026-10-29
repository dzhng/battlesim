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
        &json!({}),
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
            &json!({}),
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
        let error =
            presentation::sound::tables::SoundTables::from_parts(fixture("audio"), bad, &json!({}))
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
        "/defaults/default",
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
fn pose_denominators_gauge_and_light_ranges_are_admitted_before_use() {
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
    let mut bad = fixture("presentation");
    bad["pose"]["gauge"]["trackde_heavy"] = json!(0.8);
    assert!(
        presentation::settings::PresentationSettings::from_value(&bad)
            .err()
            .unwrap()
            .contains("pose.gauge.trackde_heavy")
    );
}

#[test]
fn unknown_mix_sounds_are_refused_before_preparation() {
    for pointer in [
        "/vehicles/default/engine",
        "/ambience/sound",
        "/blasts/default/near",
    ] {
        let mut doc = fixture("audio");
        *doc.pointer_mut(pointer).unwrap() = json!("unknown-sound");
        let error = presentation::sound::tables::SoundTables::from_parts(
            doc,
            fixture("sounds"),
            &json!({}),
        )
        .unwrap_err();
        assert!(
            error.contains(&pointer[1..].replace('/', ".")) && error.contains("unknown-sound"),
            "{error}"
        );
    }
}

#[test]
fn library_refuses_bad_firing_roles_loops_cadence_and_effect_slots() {
    let base = fixture("sounds");
    let mut bad = base.clone();
    let near = "airwar-aa-launch";
    let clip = bad["sounds"][near]["clips"][0]
        .as_str()
        .unwrap()
        .to_string();
    bad["clips"][&clip]["role"] = json!("impact");
    assert!(presentation::sound::catalog::SoundCatalog::from_value(&bad)
        .unwrap_err()
        .contains("firing clips"));
    let mut bad = base.clone();
    bad["defaults"]["default"]["near"] = json!("engine_diesel");
    assert!(presentation::sound::catalog::SoundCatalog::from_value(&bad)
        .unwrap_err()
        .contains("firing cannot use a loop"));
    let mut bad = base.clone();
    bad["effects"]["not-a-synth-slot"] = json!("rifle");
    assert!(presentation::sound::catalog::SoundCatalog::from_value(&bad)
        .unwrap_err()
        .contains("effects.not-a-synth-slot"));
    let mut bad = base.clone();
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
    bad["defaults"]["default"]["near"] = json!(burst_sound);
    bad["defaults"]["default"]["far"] = json!("rifle");
    assert!(presentation::sound::catalog::SoundCatalog::from_value(&bad)
        .unwrap_err()
        .contains("near and far play different bursts"));
    let mut folded = base;
    folded["sounds"]["rifle"]["gain"] = json!(3.0);
    folded["defaults"]["default"]["gain"] = json!(3.0);
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
fn gauge_classes_follow_the_catalogs_physical_class_vocabulary() {
    let mut doc = fixture("presentation");
    doc["pose"]["gauge"]["tracked_light_logistics"] = json!(0.8);
    presentation::settings::PresentationSettings::from_value(&doc).unwrap();
}

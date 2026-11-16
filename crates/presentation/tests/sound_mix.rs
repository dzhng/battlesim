//! A contact recording must name a surface the battle can publish.
use presentation::sound::tables::SoundTables;
use serde_json::Value;

#[test]
fn default_impact_recordings_refuse_misspelled_surfaces() {
    let fixture = |text| serde_json::from_str::<Value>(text).unwrap();
    let presentation = fixture(include_str!("../../../fixtures/presentation.json"));
    let catalog = fixture(include_str!("../../../fixtures/sounds.json"));
    let weapons = fixture(include_str!("../../../fixtures/rules.json"))["weapons"].clone();
    for surface in ["hulll", "ricochett"] {
        let mut audio = fixture(include_str!("../../../fixtures/audio.json"));
        // An existing recording isolates the surface-name admission from sound lookup.
        audio["weapon_defaults"]["impacts"][surface] =
            audio["weapon_defaults"]["impacts"]["hull"].clone();
        let error = SoundTables::from_parts(audio, catalog.clone(), &presentation, &weapons)
            .expect_err("a recording for an unknown surface must not reach every look");
        assert!(
            error.contains(&format!("weapon_defaults.impacts.{surface}")),
            "{error}"
        );
    }
}

#[test]
fn unknown_profiles_and_invalid_reach_are_refused_before_playback() {
    use presentation::sound::tables::AudioMix;
    let fixture =
        || serde_json::from_str::<Value>(include_str!("../../../fixtures/audio.json")).unwrap();
    let mut audio = fixture();
    audio["vehicles"]["default"]["engine_distance"] = serde_json::json!("missing_profile");
    let error = AudioMix::from_value(&audio).unwrap_err();
    assert!(
        error.contains("audio.vehicles.default.engine_distance"),
        "{error}"
    );
    let mut audio = fixture();
    audio["distance"]
        .as_object_mut()
        .unwrap()
        .remove("world_voices");
    let error = AudioMix::from_value(&audio).unwrap_err();
    assert!(error.contains("audio.distance.world_voices"), "{error}");
    for (field, invalid) in [
        ("ref_m", 0.0),
        ("rolloff", 0.0),
        ("floor", 1.1),
        ("max_m", 0.0),
    ] {
        let mut audio = fixture();
        audio["distance"]["burning"][field] = serde_json::json!(invalid);
        let error = AudioMix::from_value(&audio).unwrap_err();
        assert!(
            error.contains(&format!("audio.distance.burning.{field}")),
            "{error}"
        );
    }
}

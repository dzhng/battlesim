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

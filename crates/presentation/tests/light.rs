//! The world's light, as the shaders read it: the sky, the haze and the sun
//! Rust resolves from the fixture's `presentation.light` are what the browser
//! resolved when it owned them (`skyModelParams`, `skyLight`, `aerial`,
//! `photorealEnvironment`), to the last bit, for the game's light and for a
//! low sun under half overcast (`tests/fixtures/light-vectors.json`, written
//! by the browser's code before it moved here).
use presentation::light::LightSettings;
use presentation::records::{Aerial, Layout, SkyLight, Sun};
use presentation::view::{Camera, View};
use serde_json::Value;

fn vectors() -> Value {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/light-vectors.json"
    );
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap()
}

fn floats(v: &Value) -> Vec<f32> {
    match v {
        Value::Array(a) => a.iter().map(|x| x.as_f64().unwrap() as f32).collect(),
        x => vec![x.as_f64().unwrap() as f32],
    }
}

/// Every named field of `words` (a record of `layout`) equals the vector's.
fn equal(layout: &Layout, words: &[f32], want: &Value, at: &str) {
    let fields: Vec<_> = layout
        .fields
        .iter()
        .filter(|f| !f.name.starts_with("pad"))
        .collect();
    assert_eq!(
        fields.len(),
        want.as_object().unwrap().len(),
        "{at}: every field"
    );
    for field in fields {
        let got = &words[field.offset / 4..field.offset / 4 + field.count];
        let want = floats(&want[camel(field.name)]);
        assert_eq!(got, want.as_slice(), "{at}: {}", field.name);
    }
}

#[test]
fn the_light_is_the_browser_s() {
    for (name, case) in vectors().as_object().unwrap() {
        let light: LightSettings = serde_json::from_value(case["light"].clone()).unwrap();
        let r = light.records();
        let sky = [r.sky];
        let aerial = [r.aerial];
        let sun = [r.sun];
        equal(
            &SkyLight::LAYOUT,
            bytemuck::cast_slice(&sky),
            &case["sky"],
            &format!("{name} sky"),
        );
        equal(
            &Aerial::LAYOUT,
            bytemuck::cast_slice(&aerial),
            &case["aerial"],
            &format!("{name} aerial"),
        );
        equal(
            &Sun::LAYOUT,
            bytemuck::cast_slice(&sun),
            &case["sun"],
            &format!("{name} sun"),
        );
    }
}

/// The sky is drawn along the camera's rays: the middle of the screen looks
/// at the target, its top edge above it and its right edge to the camera's
/// right, at the lens's angle.
#[test]
fn the_sky_looks_where_the_camera_looks() {
    let camera = Camera {
        eye: [10.0, 20.0, 5.0],
        target: [110.0, 20.0, 5.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 1.0,
        size: [1280, 720],
    };
    let v = View::new(&camera).record(&camera);
    let at = |x: f32, y: f32| -> [f32; 3] {
        std::array::from_fn(|i| v.sky_origin[i] + v.sky_dx[i] * x + v.sky_dy[i] * y)
    };
    let unit = |d: [f32; 3]| {
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        d.map(|c| c / l)
    };
    let centre = unit(at(0.5, 0.5));
    assert!((centre[0] - 1.0).abs() < 1e-6 && centre[1].abs() < 1e-6 && centre[2].abs() < 1e-6);
    let top = unit(at(0.5, 0.0));
    assert!(
        (top[2].atan2(top[0]) - 0.4).abs() < 1e-6,
        "half the lens up: {top:?}"
    );
    let right = unit(at(1.0, 0.5));
    assert!(
        right[1] < 0.0,
        "looking along +x, the right is -y: {right:?}"
    );
}

fn camel(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in snake.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn game_light() -> LightSettings {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/presentation.json"
    );
    let game: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    serde_json::from_value(game["light"].clone()).unwrap()
}

/// The sky, the shading and the haze share one sun, and a sun shadow keeps
/// the light's shadow floor of it.
#[test]
fn one_sun_lights_the_sky_and_the_ground() {
    let light = game_light();
    let r = light.records();
    let toward = light.sun_direction().map(|v| v as f32);
    assert_eq!(r.sky.sun_direction, toward);
    assert_eq!(r.sun.sun_direction[..3], toward);
    assert!((toward.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1e-6);
    assert!(
        light.shadow_floor > 0.0,
        "a shadow darker than unseen ground reads as fog"
    );
    assert_eq!(r.sun.settings[1], light.shadow_floor as f32);
}

/// From a sun on the horizon to one overhead, in clear air to full overcast,
/// every number the shaders take is finite, and no light is negative.
#[test]
fn every_sun_and_sky_gives_finite_non_negative_light() {
    for elevation in [0.05, 0.3, 0.7, 1.2, std::f64::consts::FRAC_PI_2] {
        for turbidity in [1.0, 2.0, 6.0, 12.0] {
            let mut light = game_light();
            light.sun_elevation = elevation;
            light.sky.turbidity = turbidity;
            let r = light.records();
            let at = format!("elevation {elevation}, turbidity {turbidity}");
            assert!(r.words().iter().all(|v| v.is_finite()), "{at}");
            let light_terms = r
                .sun
                .sun_radiance
                .iter()
                .chain(&r.sun.fill)
                .chain(&r.aerial.extinction);
            assert!(light_terms.copied().all(|v| v >= 0.0), "{at}");
        }
    }
}

/// A fair sky carries its cumulus; under full overcast the clouds give way
/// to one grey dome.
#[test]
fn clouds_give_way_to_the_overcast_dome() {
    let mut light = game_light();
    assert!(light.sky().cloud_opacity > 0.5);
    light.sky.turbidity = 12.0;
    let sky = light.sky();
    assert_eq!((sky.cloud_opacity, sky.overcast), (0.0, 1.0));
}

#[test]
fn sky_and_environment_share_one_unit_sun() {
    for case in vectors().as_object().unwrap().values() {
        let light: LightSettings = serde_json::from_value(case["light"].clone()).unwrap();
        let records = light.records();
        assert_eq!(records.sky.sun_direction, records.sun.sun_direction[..3]);
        let sun = records.sky.sun_direction;
        let length = sun.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!((length - 1.0).abs() < 1e-6);
        assert!((f64::from(sun[2]) - libm::sin(light.sun_elevation)).abs() < 1e-6);
    }
}

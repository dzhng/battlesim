//! The world's light, as the shaders read it: the sky Rust resolves from the
//! fixture's `presentation.light` is the browser's (`skyModelParams` and
//! `skyLight`) to the last bit, for the game's light and for a low sun under
//! half overcast (`tests/fixtures/sky-vectors.json`, written by the browser).
use presentation::light::LightSettings;
use presentation::records::SkyLight;
use presentation::view::{Camera, View};
use serde_json::Value;

fn vectors() -> Value {
    let at = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sky-vectors.json");
    serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap()
}

fn floats(v: &Value) -> Vec<f32> {
    match v {
        Value::Array(a) => a.iter().map(|x| x.as_f64().unwrap() as f32).collect(),
        x => vec![x.as_f64().unwrap() as f32],
    }
}

#[test]
fn the_sky_is_the_browser_s() {
    for (name, case) in vectors().as_object().unwrap() {
        let light: LightSettings = serde_json::from_value(case["light"].clone()).unwrap();
        let sky = light.sky();
        let words: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&sky));
        let fields: Vec<_> = SkyLight::LAYOUT
            .fields
            .iter()
            .filter(|f| !f.name.starts_with("pad"))
            .collect();
        assert_eq!(fields.len(), case["sky"].as_object().unwrap().len(), "{name}: every field");
        for field in fields {
            let got = &words[field.offset / 4..field.offset / 4 + field.count];
            let want = floats(&case["sky"][camel(field.name)]);
            assert_eq!(got, want.as_slice(), "{name}: {}", field.name);
        }
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
    assert!((top[2].atan2(top[0]) - 0.4).abs() < 1e-6, "half the lens up: {top:?}");
    let right = unit(at(1.0, 0.5));
    assert!(right[1] < 0.0, "looking along +x, the right is -y: {right:?}");
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

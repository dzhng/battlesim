//! Every frame carries its camera as a renderer uploads it: the view, the
//! camera uniform and the sky's corner rays, to the bit of the page's camera
//! math (`camera3d.ts`), which picking uses. `fixtures/camera-vectors.json`
//! recorded that math over a tour of orbit cameras: three targets, four
//! distances, four pitches up to straight down, three yaws, three lenses and
//! five viewports.
use presentation::presentation::{Presentation, Ui};
use presentation::view::Camera;
use serde_json::Value;

/// Little-endian f32 words from the browser's hex.
fn words(hex: &Value) -> Vec<u32> {
    let s = hex.as_str().unwrap();
    (0..s.len() / 8)
        .map(|i| {
            u32::from_le_bytes(std::array::from_fn(|b| {
                u8::from_str_radix(&s[i * 8 + b * 2..i * 8 + b * 2 + 2], 16).unwrap()
            }))
        })
        .collect()
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|f| f.to_bits()).collect()
}

fn v3(v: &Value) -> [f64; 3] {
    std::array::from_fn(|i| v[i].as_f64().unwrap())
}

#[test]
fn every_frames_camera_is_the_browsers() {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/camera-vectors.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    let mut presentation = Presentation::new(None, 30.0).unwrap();
    for (k, case) in cases.iter().enumerate() {
        let camera = Camera {
            eye: v3(&case["eye"]),
            target: v3(&case["target"]),
            up: v3(&case["up"]),
            fov_y: case["fov_y"].as_f64().unwrap(),
            near: case["near"].as_f64().unwrap(),
            size: std::array::from_fn(|i| case["size"][i].as_u64().unwrap() as u32),
        };
        let time = case["time"].as_f64().unwrap();
        let f = presentation
            .frame(time, &camera, &Ui::default(), 0)
            .unwrap();
        // The camera's own math; the wind it carries is the ground's.
        let math = std::mem::offset_of!(presentation::records::Camera, wind) / 4;
        let uniform: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&f.camera));
        assert_eq!(
            bits(&uniform[..math]),
            words(&case["camera"]),
            "case {k} camera"
        );
        assert_eq!(bits(&f.view.view), words(&case["view"]), "case {k} view");
        let rays = [f.view.rays.origin, f.view.rays.dx, f.view.rays.dy].concat();
        assert_eq!(bits(&rays), words(&case["rays"]), "case {k} rays");
    }
    assert_eq!(cases.len(), 144);
}

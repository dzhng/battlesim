use super::*;
use serde_json::{json, Value};

/// A file in the documented layout: `layout` as its JSON, then a blank
/// atlas of `atlas` pixels.
fn file(layout: &Value, atlas: [u32; 2]) -> Vec<u8> {
    let text = layout.to_string();
    let mut bytes = MAGIC.to_vec();
    for n in [VERSION, text.len() as u32] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend(text.as_bytes());
    for n in atlas {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend(vec![0u8; (atlas[0] * atlas[1] * 4) as usize]);
    bytes
}

/// One own card, "AB" over a backing with an icon and a leader, shown in
/// the second of two frames at half opacity, on a 100×50 page.
fn layout() -> Value {
    json!({
        "viewport": [100, 50],
        "scale": 1,
        "cards": ["own-1"],
        "fonts": [{
            "letterSpacing": 1,
            "glyphs": {
                "A": { "advance": 6, "ink": 0, "shadow": 1 },
                "B": { "advance": 6, "ink": 2, "shadow": 3 },
                " ": { "advance": 6, "ink": null, "shadow": null }
            }
        }],
        "sprites": [
            { "at": [0, 0], "size": [8, 10], "offset": [-1, -8] },
            { "at": [8, 0], "size": [8, 10], "offset": [-1, -8] },
            { "at": [16, 0], "size": [8, 10], "offset": [-1, -8] },
            { "at": [24, 0], "size": [8, 10], "offset": [-1, -8] },
            { "at": [32, 0], "size": [12, 12], "offset": [-2, -2] }
        ],
        "leaders": [{
            "width": 1.25,
            "ink": [0.45, 0.86, 1, 0.95],
            "opacity": 0.8,
            "dash": [],
            "shadows": [{ "color": [0, 0, 0, 0.8], "blur": 1 }]
        }],
        "parts": [[
            { "rect": { "box": [-4, -12, 40, 14], "fill": [0, 0, 0, 0.6], "border": [0, 0, 0, 0], "borderPx": 0, "blurPx": 10 } },
            { "text": { "text": "A B", "font": 0, "at": [3, 9], "opacity": 0.82 } },
            { "sprite": { "sprite": 4, "at": [30, 0], "opacity": 1 } }
        ]],
        "frames": [
            [],
            [[0, 0, 0, 0.5, 20, 10, 5, 30, 2, 7, 30, 20, 16, 40, 16]]
        ]
    })
}

const ATLAS: [u32; 2] = [64, 16];

#[test]
fn a_frame_draws_its_cards_as_the_page_laid_them_out() {
    let cards = RecordedCards::parse(&file(&layout(), ATLAS), 2).unwrap();
    assert_eq!(cards.cards_of(0), Vec::<&str>::new());
    assert_eq!(cards.cards_of(1), ["own-1"]);
    assert!(
        cards.draw(0, [200, 100]).quads().is_empty(),
        "a frame without cards draws none"
    );
    // On a display twice the page's size.
    let draw = cards.draw(1, [200, 100]);
    let q = draw.quads();
    let kinds: Vec<f32> = q.iter().map(|q| q.params[0]).collect();
    // The backing, both glyphs' shadows, both inks, the icon, then the
    // leader's ring and its two segments.
    assert_eq!(kinds, [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 3.0, 2.0, 2.0]);
    assert_eq!(q[0].shape, [32.0, -4.0, 120.0, 48.0], "the backing, scaled");
    assert_eq!(q[0].params[2], 20.0, "its blur, scaled");
    assert!(
        (q[0].ink[3] - 0.3).abs() < 1e-6,
        "faded by the card's opacity"
    );
    // "A B": the pen starts at the card plus the run, and steps a glyph and
    // its letter spacing at a time; a space draws nothing.
    let pen = |n: f32| (20.0 + 3.0 + n * 7.0 - 1.0) * 2.0;
    assert_eq!(q[1].px[0], pen(0.0), "A's shadow");
    assert_eq!(q[2].px[0], pen(2.0), "B's shadow, past the space");
    assert_eq!(q[3].px[0], pen(0.0), "A's ink, over every shadow");
    assert_eq!(q[4].px[0], pen(2.0));
    assert_eq!(
        q[3].px[1],
        (10.0 + 9.0 - 8.0) * 2.0,
        "on the run's baseline"
    );
    assert!((q[3].ink[3] - 0.82 * 0.5).abs() < 1e-6);
    assert_eq!(
        q[4].uv,
        [16.0 / 64.0, 0.0, 24.0 / 64.0, 10.0 / 16.0],
        "B's ink"
    );
    assert_eq!(q[5].px, [96.0, 16.0, 120.0, 40.0], "the icon at its box");
    assert_eq!(q[6].shape, [10.0, 60.0, 4.0, 0.0], "the ring on the unit");
    assert_eq!(
        q[7].shape,
        [14.0, 60.0, 40.0, 32.0],
        "the leader to the corner"
    );
    assert_eq!(q[8].shape, [40.0, 32.0, 80.0, 32.0], "then along the edge");
    assert!((q[7].ink[3] - 0.95 * 0.8 * 0.5).abs() < 1e-6);
    assert_eq!(q[7].params[1], 2.5, "its width, scaled");
    assert_eq!(
        q[7].params[2], 1.0,
        "its shadow's sigma, half its blur, scaled"
    );
}

#[test]
fn a_stale_or_malformed_file_is_refused() {
    let good = file(&layout(), ATLAS);
    assert!(RecordedCards::parse(&good, 2).is_ok());
    let refused = |bytes: &[u8], frames: usize, says: &str| {
        let e = RecordedCards::parse(bytes, frames).expect_err(says);
        assert!(e.contains(says), "{e}");
    };
    let edited = |edit: &dyn Fn(&mut Value)| {
        let mut layout = layout();
        edit(&mut layout);
        file(&layout, ATLAS)
    };
    let mut magic = good.clone();
    magic[0] = b'X';
    refused(&magic, 2, "not a recorded card file");
    let mut version = good.clone();
    version[4] = 2;
    refused(&version, 2, "version 2");
    refused(&good, 3, "2 card frames for 3 frame records");
    refused(&good[..good.len() - 2], 2, "ends at byte");
    let mut long = good.clone();
    long.push(0);
    refused(&long, 2, "1 bytes past the atlas");
    refused(
        &edited(&|l| l["stale"] = json!(1)),
        2,
        "unknown field `stale`",
    );
    refused(
        &edited(&|l| l["parts"][0][1]["text"]["text"] = json!("AC")),
        2,
        "writes C, which its font lacks",
    );
    refused(
        &edited(&|l| l["parts"][0][2]["sprite"]["sprite"] = json!(9)),
        2,
        "part 0 names sprite 9 of 5",
    );
    refused(
        &edited(&|l| l["sprites"][4]["at"] = json!([60, 0])),
        2,
        "sprite 4 lies past the 64×16 atlas",
    );
    refused(
        &edited(&|l| l["frames"][1][0][1] = json!(1)),
        2,
        "frame 1 names part 1 of 1",
    );
    refused(
        &edited(&|l| l["frames"][1][0][2] = json!(3)),
        2,
        "frame 1 names leader 3 of 1",
    );
    refused(
        &edited(&|l| l["frames"][1][0] = json!([0, 0, 0])),
        2,
        "the card layout",
    );
}

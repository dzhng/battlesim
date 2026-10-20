//! Prepared surface-depth static cards (`scene-assets/cards.ts`).
use super::{Container, Kind, View};
use serde::Deserialize;

pub const WIDTH: u32 = 512;
pub const HEIGHT: u32 = 128;
pub const PIXEL_BYTES: usize = (WIDTH * HEIGHT * 4) as usize;
const CARD: Kind = Kind(0x4349_4742, 2, "card");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    yaws: u32,
    pitches: [f32; 2],
    cell: u32,
    supersample: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    bundle: String,
    state: String,
    spec: Spec,
    width: u32,
    height: u32,
    center: [f32; 3],
    radius: f32,
    albedo: View,
    normal: View,
    depth: View,
}

pub struct Card {
    pub frame: [f32; 4],
    pub albedo: Vec<u8>,
    pub normal: Vec<u8>,
    pub depth: Vec<u8>,
}

impl Card {
    pub fn decode(bytes: &[u8], appearance: &str, bundle: &str) -> Result<Self, String> {
        let what = format!("card {appearance}");
        let c = Container::<Header>::parse(bytes, CARD, &what)?;
        let h = &c.content;
        if h.bundle != bundle || h.state != "default" {
            return Err(format!("{what}: source bundle or state differs; re-bake"));
        }
        if h.width != WIDTH
            || h.height != HEIGHT
            || h.spec.yaws != 8
            || h.spec.cell != 64
            || h.spec.supersample != 2
            || h.spec.pitches != [0.85, 0.5]
            || !h.center.iter().all(|n| n.is_finite())
            || !h.radius.is_finite()
            || h.radius <= 0.0
        {
            return Err(format!(
                "{what}: unsupported views or invalid frame; re-bake"
            ));
        }
        let albedo = c.u8s(h.albedo)?;
        let normal = c.u8s(h.normal)?;
        let depth = c.u8s(h.depth)?;
        if depth.len() != PIXEL_BYTES / 2
            || albedo.len() != PIXEL_BYTES
            || normal.len() != PIXEL_BYTES
        {
            return Err(format!("{what}: invalid card pixels; re-bake"));
        }
        Ok(Self {
            frame: [h.center[0], h.center[1], h.center[2], h.radius],
            albedo,
            normal,
            depth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bytes(offset: usize) -> Vec<u8> {
        let header = serde_json::json!({"content": {
            "bundle": "a".repeat(64), "state": "default",
            "spec": {"yaws":8,"pitches":[0.85,0.5],"cell":64,"supersample":2},
            "width":512,"height":128,"center":[0,0,1],"radius":2,
            "albedo":{"$view":0},"normal":{"$view":1},"depth":{"$view":2}},
            "views":[["u8",offset,PIXEL_BYTES],["u8",PIXEL_BYTES,PIXEL_BYTES],["u8",PIXEL_BYTES*2,PIXEL_BYTES/2]]});
        let json = serde_json::to_vec(&header).unwrap();
        let mut out = Vec::new();
        for n in [
            CARD.0,
            CARD.1,
            json.len() as u32,
            (PIXEL_BYTES / 2 * 5) as u32,
        ] {
            out.extend(n.to_le_bytes());
        }
        out.extend(json);
        out.resize(out.len() + PIXEL_BYTES, 127);
        out.resize(out.len() + PIXEL_BYTES, 63);
        out.resize(out.len() + PIXEL_BYTES / 2, 31);
        out
    }
    #[test]
    fn shared_card_pixels_and_negative_identity() {
        let bytes = bytes(0);
        let card = Card::decode(&bytes, "test", &"a".repeat(64)).unwrap();
        assert_eq!(card.frame, [0.0, 0.0, 1.0, 2.0]);
        assert!(card.albedo.iter().all(|&v| v == 127));
        assert!(card.normal.iter().all(|&v| v == 63));
        assert!(Card::decode(&bytes, "test", &"b".repeat(64)).is_err());
        assert!(Card::decode(&bytes[..bytes.len() - 1], "test", &"a".repeat(64)).is_err());
        let mut bad = bytes;
        bad[4] = 1;
        assert!(Card::decode(&bad, "test", &"a".repeat(64)).is_err());
    }
    #[test]
    fn overflowing_pixel_view_is_refused_without_panicking() {
        assert!(Card::decode(&bytes(usize::MAX), "test", &"a".repeat(64)).is_err());
    }
}

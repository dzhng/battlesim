//! A falling airframe's row: who it is to the side, its type, how it hangs,
//! and the structure it struck this tick. A row that struck nothing carries
//! a `strikeMps` of -1 and a zero point and normal.
use contract::observation::FallingAirframe;

pub(super) const CRASH_FIELDS: [&str; 16] = [
    "id",
    "own",
    "kind",
    "x",
    "y",
    "z",
    "yaw",
    "pitch",
    "roll",
    "strikeX",
    "strikeY",
    "strikeZ",
    "strikeNx",
    "strikeNy",
    "strikeNz",
    "strikeMps",
];

pub(super) fn crash_row(c: &FallingAirframe) -> [f32; 16] {
    let (point, normal, speed) = c.strike.map_or(([0.0; 3], [0.0; 3], -1.0), |s| {
        (s.point, s.normal, s.speed_mps)
    });
    [
        c.id as f32,
        c.own as u8 as f32,
        c.kind.0 as f32,
        c.position[0] as f32,
        c.position[1] as f32,
        c.position[2] as f32,
        c.yaw as f32,
        c.pitch as f32,
        c.roll as f32,
        point[0] as f32,
        point[1] as f32,
        point[2] as f32,
        normal[0] as f32,
        normal[1] as f32,
        normal[2] as f32,
        speed as f32,
    ]
}

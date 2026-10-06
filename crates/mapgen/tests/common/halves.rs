//! A body abandoned in a road keeps to one half of it.

use contract::ground::segment_distance;

/// Whether every one of `corners` lies on the same side of the middle line
/// of whichever of the carriageway `pieces` (each a centreline segment) is
/// nearest their centre: a body in one half of its road.
pub fn in_one_half(
    pieces: impl IntoIterator<Item = ([f64; 2], [f64; 2])>,
    corners: &[[f64; 2]; 4],
) -> bool {
    let centre = [
        corners.iter().map(|c| c[0]).sum::<f64>() / 4.0,
        corners.iter().map(|c| c[1]).sum::<f64>() / 4.0,
    ];
    let Some((a, b)) = pieces.into_iter().min_by(|x, y| {
        segment_distance(x.0, x.1, centre).total_cmp(&segment_distance(y.0, y.1, centre))
    }) else {
        return false;
    };
    let side =
        |p: &[f64; 2]| ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).signum();
    let first = side(&centre);
    corners.iter().all(|c| side(c) == first)
}

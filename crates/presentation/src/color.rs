//! Linear-light colour conventions shared by CPU preparation and GPU shading.

/// Rec. 709 luminance weights, in RGB order.
pub const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// Luminance of a linear RGB colour.
pub fn luminance(c: [f64; 3]) -> f64 {
    LUMA[0] * c[0] + LUMA[1] * c[1] + LUMA[2] * c[2]
}

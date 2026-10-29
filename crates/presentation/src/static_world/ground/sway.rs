//! How a standing tree sways in the one wind (look only): the numbers the
//! renderer's `sway` shader reads (generated into `records.gen.ts` as
//! `SWAY`), and so the farthest any point of a tree can move from rest,
//! which the scenery walk grows its chunks by, so a crown bent into view or
//! into a cascade is drawn there. One owner: the shader and the bounds
//! cannot disagree.
//!
//! The trunk bends downwind by `height · bend · strength · (lean + gust·g +
//! swing·s·(1 + g)/2)` at the top (the point's height share cubed below it),
//! where `strength` is the wind's gust speed over `full_gust_mps` (at most
//! one), `g` a gust front passing (0 to 1) and `s` the swing (`sin` with a
//! quarter overtone, so at most 1.25 either way). Leaves shiver about that
//! by up to `shiver_m · strength · (1 + g)/2` on each axis at the top.

/// The tuning; see the module's formula.
#[derive(Clone, Copy, Debug)]
pub struct Sway {
    /// The top's bend at full strength, as a share of the tree's height.
    pub bend: f64,
    /// The steady lean, the extra lean a passing gust adds, and the swing.
    pub lean: f64,
    pub gust: f64,
    pub swing: f64,
    /// The swing's rate (rad/s).
    pub rate: f64,
    /// The gust speed (m/s) at and past which the wind is at full strength.
    pub full_gust_mps: f64,
    /// The leaves' shiver at the crown's top in a gust (m), and its rate.
    pub shiver_m: f64,
    pub shiver_rate: f64,
}

pub const SWAY: Sway = Sway {
    bend: 0.18,
    lean: 0.15,
    gust: 0.7,
    swing: 0.35,
    rate: 1.6,
    full_gust_mps: 6.0,
    shiver_m: 0.15,
    shiver_rate: 7.0,
};

impl Sway {
    /// The farthest any point of a tree `height` metres tall can lie from
    /// where it stands at rest, in any wind: the trunk's whole bend and the
    /// leaves' whole shiver together.
    pub fn reach(&self, height: f64) -> f64 {
        let bend = height * self.bend * (self.lean + self.gust + 1.25 * self.swing);
        bend + self.shiver_m * 3f64.sqrt()
    }
}

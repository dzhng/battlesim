//! Combat effects: what a side's publications say happened becomes
//! short-lived effects, and each frame the effect pass's instances and the
//! cast lights at the presentation clock.
mod batch;
pub mod cook_off;
pub mod cook_offs;
pub mod frame;
pub mod high_air_shadow;
pub mod launches;
pub mod lights;
pub mod publication;
pub mod tables;
pub mod transit;

use crate::world::Vec3;

/// Where a shot's muzzle is drawn at this frame: the model's, not the
/// simulation's, so a flash stays on the drawn barrel.
pub trait MuzzleSource {
    /// The drawn muzzle of `shooter`'s (a side key) mount `mount`, or of its
    /// soldier `soldier` when set; None when nothing of it is drawn.
    fn muzzle(&mut self, shooter: f64, mount: usize, soldier: Option<f64>) -> Option<Vec3>;
}

/// One drawn rotor, as the effects draw its blur.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct DrawnRotor {
    pub hub: Vec3,
    /// The axis it turns about, its local +Z.
    pub axis: Vec3,
    /// The direction one of its blades points, across the axis.
    pub blade: Vec3,
    pub radius: f64,
    pub blades: f64,
    pub chord: f64,
    /// The angle it turned since the last drawn frame, radians.
    pub turned: f64,
    /// Its mast from where it meets the airframe to the head's top.
    pub mast: Option<Mast>,
    #[serde(rename = "mastRadius")]
    pub mast_radius: f64,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub struct Mast {
    pub base: Vec3,
    pub top: Vec3,
}

mod tables_read;

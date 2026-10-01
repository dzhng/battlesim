//! One immutable authored centreline and the points every consumer samples.
//! Until C65 rounds the corners, the samples are the authored controls.
#[derive(Clone, Debug)]
pub struct Centerline {
    controls: Vec<[f64; 2]>,
}

impl Centerline {
    pub fn new(controls: Vec<[f64; 2]>) -> Self {
        Self { controls }
    }

    pub fn control_points(&self) -> &[[f64; 2]] {
        &self.controls
    }

    pub fn samples(&self) -> &[[f64; 2]] {
        &self.controls
    }
}

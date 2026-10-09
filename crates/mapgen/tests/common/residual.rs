/// What the generator still leaves of one defect on a sweep of 36 maps (four
/// seeds at every type and size), read over fifteen such sweeps (seeds 1 to
/// 60) rather than one: their mean and standard deviation. Each is a count
/// to bring down: lower it when its defect is closed, never raise it.
pub struct Residual {
    pub mean: f64,
    pub sd: f64,
}

impl Residual {
    /// The most one sweep may show: the mean and three standard deviations.
    /// Chaos-marginal, so another sample of the same generator passes and a
    /// generator that leaves markedly more fails.
    pub fn allowed(&self) -> usize {
        (self.mean + 3.0 * self.sd).ceil() as usize
    }
}

//! Fixed deployment cells shared by own and identified rows.
use super::tags::{tag, POSTURES};
use contract::observation::DeploymentState;

pub(super) fn cells(state: Option<DeploymentState>) -> [f32; 2] {
    state.map_or([-1.0; 2], |d| {
        [d.progress as f32, tag(&POSTURES, &d.target)]
    })
}

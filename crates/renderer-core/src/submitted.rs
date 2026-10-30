//! Actual partial-world draw work, before clipping, scissor or fragment discard.
//! Effects, grass, sky, paint, ghosts, x-ray and display overlays are excluded.
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedDraws {
    pub triangles: u64,
    pub instances: u64,
    pub draws: u64,
}
impl SubmittedDraws {
    pub fn record(&mut self, vertices: u32, instances: u32) {
        self.triangles += u64::from(vertices / 3) * u64::from(instances);
        self.instances += u64::from(instances);
        self.draws += 1;
    }
    pub(crate) fn add(&mut self, other: Self) {
        self.triangles += other.triangles;
        self.instances += other.instances;
        self.draws += other.draws;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedPass {
    pub terrain: SubmittedDraws,
    pub skirt_and_props: SubmittedDraws,
    pub water: SubmittedDraws,
    pub backdrop: SubmittedDraws,
    pub buildings: SubmittedDraws,
    pub scenery: SubmittedDraws,
    pub models_including_props: SubmittedDraws,
}
impl SubmittedPass {
    fn add(&mut self, other: Self) {
        self.terrain.add(other.terrain);
        self.skirt_and_props.add(other.skirt_and_props);
        self.water.add(other.water);
        self.backdrop.add(other.backdrop);
        self.buildings.add(other.buildings);
        self.scenery.add(other.scenery);
        self.models_including_props
            .add(other.models_including_props);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedFrame {
    pub depth: SubmittedPass,
    pub color: SubmittedPass,
    pub high_depth: SubmittedPass,
    pub high_color: SubmittedPass,
    pub cascades: [SubmittedPass; presentation::shadow::CASCADES],
}
impl SubmittedFrame {
    pub fn add(&mut self, other: Self) {
        self.depth.add(other.depth);
        self.color.add(other.color);
        self.high_depth.add(other.high_depth);
        self.high_color.add(other.high_color);
        for (a, b) in self.cascades.iter_mut().zip(other.cascades) {
            a.add(b);
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Pass {
    Depth,
    Color,
    HighDepth,
    HighColor,
    Cascade(usize),
}

/// Each GPU owner retains its own issued work; the runtime combines snapshots.
#[derive(Default)]
pub struct Submissions {
    frame: RefCell<SubmittedFrame>,
    excluded: Cell<bool>,
}

pub(crate) struct Excluded<'a>(&'a Cell<bool>, bool);
impl Drop for Excluded<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}
impl Submissions {
    pub fn reset(&self) {
        *self.frame.borrow_mut() = SubmittedFrame::default();
    }
    pub fn snapshot(&self) -> SubmittedFrame {
        *self.frame.borrow()
    }
    pub(crate) fn exclude(&self) -> Excluded<'_> {
        Excluded(&self.excluded, self.excluded.replace(true))
    }
    pub(crate) fn record(&self, pass: Pass, add: impl FnOnce(&mut SubmittedPass)) {
        if self.excluded.get() {
            return;
        }
        let mut frame = self.frame.borrow_mut();
        add(match pass {
            Pass::Depth => &mut frame.depth,
            Pass::Color => &mut frame.color,
            Pass::HighDepth => &mut frame.high_depth,
            Pass::HighColor => &mut frame.high_color,
            Pass::Cascade(i) => &mut frame.cascades[i],
        });
    }
}

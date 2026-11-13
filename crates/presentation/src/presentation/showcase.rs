//! One unit at rest on a stage of its own, with no battle behind it. It is
//! posed by the battle's own driver and drawn models, fed a unit
//! standing still instead of a publication, so its idle life is the
//! battle's: each soldier's own tempo and gaze, the occasional watcher, the
//! kit each operator carries. It stands on the ground (`FeedUnit::grounded`),
//! so an aircraft is parked: its rotors stand still and draw as blades,
//! with no blur.
use super::*;
use crate::effects::frame::draw_rotors;
use crate::effects::tables::RotorBlurStyle;
use crate::fielded::Fielded;
use crate::units::driver::{FeedFallen, FeedFrame, FeedMount, FeedSoldier, FeedUnit};

/// The unit a page shows: a kind of the fielded units, the side whose look
/// it wears, where it stands and faces, and for a squad each soldier and
/// the operator of each mount (as `game-wasm`'s `Showground` stands them).
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ShowcaseInput {
    pub kind: String,
    pub side: String,
    pub at: [f64; 3],
    /// Radians counter-clockwise from +X.
    pub yaw: f64,
    /// A squad's soldiers, placed in the world; none for a hull.
    pub soldiers: Vec<ShowcaseSoldier>,
    /// Each mount's operator, a soldier id; empty for a hull.
    pub operators: Vec<Option<u32>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowcaseSoldier {
    pub id: u32,
    pub slot: usize,
    /// Where he stands on the ground at the unit's height, `[x, y]`.
    pub at: [f64; 2],
}

pub(super) struct Showcase {
    units: Arc<UnitsSection>,
    tables: Arc<ClipTables>,
    inverse_binds: HashMap<String, Vec<Mat4>>,
    driver: PoseDriver,
    drawn: DrawnModels,
    blur: RotorBlurStyle,
    pub(super) batch: EffectBatch,
    unit: FeedUnit,
    /// No one has fallen: one list, so the driver never reconciles it.
    fallen: Arc<Vec<FeedFallen>>,
    scratch: Scratch,
}

impl Showcase {
    fn new(fielded: Fielded, input: ShowcaseInput) -> Result<Self, String> {
        let ty = fielded
            .units
            .types
            .get(&input.kind)
            .ok_or_else(|| format!("showcase: {} is not fielded", input.kind))?;
        if !ty.draws.contains_key(&input.side) {
            return Err(format!(
                "showcase: {} has no {} look",
                input.kind, input.side
            ));
        }
        if ty.hull != input.soldiers.is_empty() {
            return Err(format!(
                "showcase: {} stands {} soldiers",
                input.kind,
                if ty.hull { "no" } else { "its" }
            ));
        }
        if !input.operators.is_empty() && input.operators.len() != ty.mounts.len() {
            return Err(format!(
                "showcase: {} has {} mounts, {} operators given",
                input.kind,
                ty.mounts.len(),
                input.operators.len()
            ));
        }
        let [x, y, z] = input.at;
        let unit = FeedUnit {
            id: 0.0,
            kind: input.kind.as_str().into(),
            side: input.side.as_str().into(),
            position: [x, y, z],
            yaw: input.yaw,
            soldiers: input
                .soldiers
                .iter()
                .map(|s| FeedSoldier {
                    id: f64::from(s.id),
                    slot: s.slot,
                    position: [s.at[0], s.at[1], z],
                    ..FeedSoldier::default()
                })
                .collect(),
            mounts: (0..ty.mounts.len())
                .map(|m| FeedMount {
                    operator: input.operators.get(m).copied().flatten().map(f64::from),
                    bearing: input.yaw,
                    ..FeedMount::default()
                })
                .collect(),
            grounded: true,
            ..FeedUnit::default()
        };
        let presentation = EffectPresentation::from_value(&fielded.effects.presentation)?;
        let units = Arc::new(fielded.units);
        let tables = Arc::new(fielded.tables);
        Ok(Self {
            inverse_binds: posing::inverse_binds(&units),
            driver: PoseDriver::new(&units)?,
            drawn: DrawnModels::new(units.clone(), tables.clone(), &input.side),
            blur: presentation.rotor_blur,
            batch: EffectBatch::new(presentation.capacity),
            unit,
            fallen: Arc::default(),
            scratch: Scratch::default(),
            units,
            tables,
        })
    }

    /// A skinned row's body as it holds still ([`posing::rest_palette`]).
    pub(super) fn rest_palette(&self, name: &str, lies: u32) -> Option<Vec<Joint>> {
        posing::rest_palette(&self.units, &self.tables, &self.inverse_binds, name, lies)
    }

    pub(super) fn register_rows(&self, world: &mut World) {
        for (name, a) in &self.units.appearances {
            posing::register_appearance(world, name, a);
        }
    }

    /// The unit at presentation time `time`, and its rotors' blur into its
    /// effects.
    pub(super) fn pose(&mut self, time: f64, ground: Option<&Ground>) {
        let feed = FeedFrame {
            time,
            units: vec![self.unit.clone()],
            fallen: self.fallen.clone(),
        };
        let poses = self.driver.update(&feed, &self.units, ground);
        self.drawn.update(poses);
        self.batch.count = 0;
        self.batch.dropped = 0;
        draw_rotors(&self.blur, &self.drawn.rotors(), &mut self.batch);
    }

    /// The unit as last posed into the frame's models, its detail scaled by
    /// `unit_scale`.
    pub(super) fn draw(
        &mut self,
        view: &View,
        world: &World,
        unit_scale: f64,
        f: &mut Frame,
        models: &mut Vec<Model>,
    ) {
        let poses = self.driver.poses();
        let mut detail = self.units.detail;
        detail.lod_px = detail.lod_px.map(|v| v * unit_scale);
        let art = UnitArt {
            units: &self.units,
            tables: &self.tables,
            inverse_binds: &self.inverse_binds,
            world,
            view,
            detail,
        };
        let plain = |_: &str, _: f64| [0.0; 4];
        let culled = art.soldiers(&poses.soldiers, plain, &mut self.scratch.worlds, f, models);
        f.counts.culled += culled;
        // It stands on the ground: drawn and shadowed as a ground vehicle is.
        art.vehicles(
            &poses.vehicles,
            |_| (None, VIEW_PASS),
            plain,
            &mut self.scratch,
            f,
            models,
        );
    }
}

impl Presentation {
    /// Shows `input`'s unit at rest, drawn from what `fielded` resolved for
    /// it, in place of any shown before. Its rows are the presentation's own,
    /// as a battle's are, so the page loads their art as it loads a battle's;
    /// the lab's rows move after them: reinstall the renderer's tables after
    /// this call. A presentation shows a battle or a showcase, never both.
    pub fn set_showcase(&mut self, fielded: Fielded, input: ShowcaseInput) -> Result<(), String> {
        if self.loaded.is_some() {
            return Err("showcase: a battle is loaded".into());
        }
        self.showcase = Some(Showcase::new(fielded, input)?);
        self.catalog_revision = catalog::next_revision();
        self.register_shown_rows();
        Ok(())
    }

    /// Stops showing a unit: its rows go, and the lab's move back.
    pub fn clear_showcase(&mut self) {
        if self.showcase.take().is_some() {
            self.catalog_revision = catalog::next_revision();
            self.register_shown_rows();
        }
    }

    /// The rows after the world's own: the shown unit's, then the lab's.
    pub(super) fn register_shown_rows(&mut self) {
        self.clear_lab_rows();
        if let Some(frontier) = self.showcase_frontier.take() {
            lab::truncate_rows(&mut self.world, frontier);
        }
        if let Some(showcase) = &self.showcase {
            self.showcase_frontier = Some(lab::frontier(&self.world));
            showcase.register_rows(&mut self.world);
        }
        self.register_lab_rows();
    }
}

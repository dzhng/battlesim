//! Copied ground, grass and scar records admitted by the shared native owners.
use super::*;
use presentation::records::Camera;
use presentation::scars::ScarUpload;
use presentation::static_world::ground::grass::Window;
use renderer_core::grass::GrassData;
use renderer_core::ground::GroundData;
use renderer_core::scars::ScarView;
use renderer_core::terrain::TerrainUpload;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GroundSnapshot {
    params: Vec<u32>,
    nodes: Vec<u32>,
    plots: Vec<u32>,
    surfaces: Vec<f32>,
    surface_index: Vec<u32>,
    scars: Vec<u32>,
    heights: Vec<u32>,
    terrain: Vec<f32>,
    skirt: Vec<f32>,
    backdrop: Vec<f32>,
    water: Vec<f32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GrassSnapshot {
    params: Vec<u32>,
    props: Vec<u32>,
    shapes: Vec<f32>,
    max_blades: u32,
    capacity: [u32; 2],
}

#[wasm_bindgen]
impl Renderer {
    pub fn load_ground_world(&mut self, snapshot: JsValue) -> Result<(), JsValue> {
        let snapshot = serde_wasm_bindgen::from_value(snapshot).map_err(js_error)?;
        self.install_ground_world(Arc::new(snapshot))
    }

    /// Canonical authored grass admission, including blade facts needed by presentation.
    pub fn pack_grass_shapes(&self, kinds: js_sys::Array) -> Result<JsValue, JsValue> {
        let names = kinds
            .iter()
            .map(|name| {
                name.as_string()
                    .ok_or_else(|| js_error("grass kinds must be appearance names"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let packed = renderer_core::grass::pack_shapes(self.model_library.as_ref(), &names)
            .map_err(js_error)?;
        serde_wasm_bindgen::to_value(&packed).map_err(js_error)
    }

    pub fn load_grass_world(&mut self, snapshot: JsValue) -> Result<(), JsValue> {
        let snapshot = serde_wasm_bindgen::from_value(snapshot).map_err(js_error)?;
        self.install_grass_world(Arc::new(snapshot))
    }

    /// The window includes the presentation's canonical metres-per-pixel value.
    /// Returns scar bounds only when this frame actually needs grass regrowth.
    pub fn prepare_grass(
        &mut self,
        window: &[f64],
        camera: &[f32],
        params: &[u32],
        changed: bool,
    ) -> Result<Vec<f64>, JsValue> {
        let camera: Camera = record(camera, "grass camera")?;
        let window = match window {
            [] => None,
            [x0, y0, x, y, reach, scale] => Some((
                Window {
                    x0: *x0,
                    y0: *y0,
                    tiles_x: *x as u32,
                    tiles_y: *y as u32,
                    reach: *reach,
                },
                *scale,
            )),
            _ => return Err(js_error("grass window needs six canonical values")),
        };
        let grass = self
            .grass
            .as_mut()
            .ok_or_else(|| js_error("load grass world before preparing grass"))?;
        let expected = self
            .grass_snapshot
            .as_ref()
            .expect("installed grass snapshot")
            .params
            .len();
        if params.len() != expected {
            return Err(js_error(
                "grass frame parameter length differs from its world",
            ));
        }
        if changed {
            grass.regrow();
        }
        grass.prepare_records(&self.queue, window, &camera, |words, _| {
            words.copy_from_slice(params)
        });
        Ok(grass
            .scar_bounds()
            .map_or_else(Vec::new, |bounds| bounds.to_vec()))
    }

    /// Scissor selection stays in the shared owner. The shell supplies uploads
    /// from presentation's memory after resolving grass, then reading regions.
    pub fn prepare_scars(
        &mut self,
        grass: JsValue,
        regions: &[f64],
        prepare: js_sys::Function,
    ) -> Result<(), JsValue> {
        let grass: Option<ScarUpload> = serde_wasm_bindgen::from_value(grass).map_err(js_error)?;
        let (regions, tail) = regions.as_chunks::<4>();
        if !tail.is_empty() {
            return Err(js_error("scar regions need four values each"));
        }
        let camera: Camera = record(
            &self
                .presentation_snapshot
                .as_ref()
                .ok_or_else(|| js_error("upload frame before preparing scars"))?
                .camera,
            "scar camera",
        )?;
        let ground = self
            .ground
            .as_mut()
            .ok_or_else(|| js_error("load ground before preparing scars"))?;
        let mut error = None;
        let resized = ground.material.scars.prepare_records(
            &self.device,
            ScarView {
                grass,
                regions,
                camera: &camera,
                map_box: ground.map_box.as_ref(),
                size: [self.width, self.height],
            },
            |region, clipped| {
                if error.is_some() {
                    return ScarUpload::default();
                }
                let region = region.map_or(JsValue::NULL, |region| {
                    js_sys::Float64Array::from(region.as_slice()).into()
                });
                let result = prepare
                    .call2(&JsValue::UNDEFINED, &region, &JsValue::from_bool(clipped))
                    .and_then(|value| serde_wasm_bindgen::from_value(value).map_err(js_error));
                match result {
                    Ok(upload) => upload,
                    Err(cause) => {
                        error = Some(cause);
                        ScarUpload::default()
                    }
                }
            },
        );
        if let Some(error) = error {
            return Err(error);
        }
        if resized {
            ground.material.rebind_scars(&self.device);
        }
        Ok(())
    }
}

impl Renderer {
    pub(super) fn install_ground_world(
        &mut self,
        snapshot: Arc<GroundSnapshot>,
    ) -> Result<(), JsValue> {
        if [
            &snapshot.terrain,
            &snapshot.skirt,
            &snapshot.backdrop,
            &snapshot.water,
        ]
        .into_iter()
        .any(|mesh| !mesh.len().is_multiple_of(10))
        {
            return Err(js_error("ground meshes need ten floats per vertex"));
        }
        let ground = Ground::from_data(
            &self.device,
            &self.queue,
            &self.production_pipelines,
            GroundData {
                material: TerrainUpload {
                    params: bytemuck::cast_slice(&snapshot.params),
                    nodes: bytemuck::cast_slice(&snapshot.nodes),
                    plots: bytemuck::cast_slice(&snapshot.plots),
                    surfaces: bytemuck::cast_slice(&snapshot.surfaces),
                    surface_index: bytemuck::cast_slice(&snapshot.surface_index),
                    scars: bytemuck::cast_slice(&snapshot.scars),
                },
                heights: &snapshot.heights,
                terrain: &snapshot.terrain,
                skirt: &snapshot.skirt,
                backdrop: &snapshot.backdrop,
                water: &snapshot.water,
            },
        );
        self.shared_fog_sight
            .rebind_heights(&self.device, &ground.heights);
        self.stats.shared_terrain = true;
        self.stats.terrain_params_words = snapshot.params.len() as u32;
        self.stats.terrain_nodes_words = snapshot.nodes.len() as u32;
        self.stats.terrain_plots_words = snapshot.plots.len() as u32;
        self.stats.terrain_surface_words = snapshot.surfaces.len() as u32;
        self.stats.terrain_index_words = snapshot.surface_index.len() as u32;
        self.stats.fog_height_words = snapshot.heights.len() as u32;
        self.ground = Some(ground);
        self.ground_snapshot = Some(snapshot);
        self.grass = None;
        self.grass_snapshot = None;
        self.prepare_ground()
    }

    pub(super) fn install_grass_world(
        &mut self,
        snapshot: Arc<GrassSnapshot>,
    ) -> Result<(), JsValue> {
        let ground = self
            .ground
            .as_ref()
            .ok_or_else(|| js_error("load ground before grass"))?;
        self.grass = Some(Grass::from_data(
            &self.device,
            &self.production_pipelines,
            &ground.heights,
            GrassData {
                params: &snapshot.params,
                props: &snapshot.props,
                shapes: &snapshot.shapes,
                max_blades: snapshot.max_blades,
                capacity: snapshot.capacity,
            },
        ));
        self.grass_snapshot = Some(snapshot);
        Ok(())
    }

    pub(super) fn prepare_ground(&mut self) -> Result<(), JsValue> {
        if let (Some(ground), Some(frame)) = (&mut self.ground, &self.presentation_snapshot) {
            let camera: Camera = record(&frame.camera, "ground camera")?;
            let casters =
                bytemuck::try_cast_slice::<f32, Camera>(&frame.casters).map_err(js_error)?;
            ground.prepare_records(&camera, casters);
        }
        Ok(())
    }
}

impl GroundSnapshot {
    pub(super) fn payload_bytes(&self) -> usize {
        size_of_val(self.params.as_slice())
            + size_of_val(self.nodes.as_slice())
            + size_of_val(self.plots.as_slice())
            + size_of_val(self.surfaces.as_slice())
            + size_of_val(self.surface_index.as_slice())
            + size_of_val(self.scars.as_slice())
            + size_of_val(self.heights.as_slice())
            + size_of_val(self.terrain.as_slice())
            + size_of_val(self.skirt.as_slice())
            + size_of_val(self.backdrop.as_slice())
            + size_of_val(self.water.as_slice())
    }
}

impl GrassSnapshot {
    pub(super) fn payload_bytes(&self) -> usize {
        size_of_val(self.params.as_slice())
            + size_of_val(self.props.as_slice())
            + size_of_val(self.shapes.as_slice())
    }
}

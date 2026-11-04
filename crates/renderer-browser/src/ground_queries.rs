//! Material queries borrow the installed runtime's pipeline owner, then snapshot.
use super::*;
use renderer_core::probes::terrain::{Query, TerrainProbe};
use renderer_core::scars::ScarFrame;
use renderer_core::terrain::{TerrainResources, TerrainUpload};
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen]
impl Renderer {
    /// Query canonical ground without exporting device/resource handles to JavaScript.
    pub fn ground_query(
        &self,
        presentation: &mut Presentation,
        mode: &str,
        words: Vec<f32>,
        reference: Vec<u8>,
        reference_side: u32,
    ) -> Result<js_sys::Promise, JsValue> {
        self.gpu.ensure_live()?;
        let query = match mode {
            "distances" => Query::Distances,
            "paved" => Query::Paved,
            "filters" => Query::Filters,
            _ => {
                return Err(js_error(
                    "ground query mode must be distances, paved or filters",
                ))
            }
        };
        let (rows, remainder) = words.as_chunks::<4>();
        if !remainder.is_empty() {
            return Err(js_error("ground queries need complete four-float rows"));
        }
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before querying ground"))?;
        let core = presentation.core_mut();
        let ground = core
            .ground()
            .ok_or_else(|| js_error("ground query has no canonical ground"))?;
        let tables = ground.terrain_tables(ground.terrain_field().map_err(js_error)?);
        let plots = ground.plot_records(true);
        let mut material = TerrainResources::from_upload(
            &self.gpu.device,
            &self.gpu.queue,
            &runtime.pipelines,
            TerrainUpload {
                params: tables.params,
                nodes: bytemuck::cast_slice(&tables.nodes),
                plots: bytemuck::cast_slice(&plots),
                surfaces: bytemuck::cast_slice(&tables.surfaces),
                surface_index: bytemuck::cast_slice(&tables.surface_index),
                scars: tables.scars,
            },
        );
        if matches!(query, Query::Filters) {
            core.resend_scars();
            let upload = core.prepare_scars(None, false);
            if material.scars.prepare_uploads(
                &self.gpu.device,
                ScarFrame {
                    view: Some(upload.clone()),
                    ..Default::default()
                },
            ) {
                material.rebind_scars(&self.gpu.device);
            }
            material.scars.apply(&self.gpu.queue, &upload);
            // The diagnostic's private material consumed this upload. A later
            // live frame must still publish it to its own retained resources.
            core.resend_scars();
        }
        let probe =
            TerrainProbe::new(&self.gpu.device, &runtime.pipelines, query).map_err(js_error)?;
        let source = probe.source().to_owned();
        let read = probe
            .queue(
                &self.gpu.device,
                &self.gpu.queue,
                &material,
                rows,
                &reference,
                reference_side,
            )
            .map_err(js_error)?;
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let bytes = read.read(&device).await.map_err(js_error)?;
            let values: Vec<_> = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|v| f32::from_le_bytes(*v))
                .collect();
            let result = js_sys::Object::new();
            js_sys::Reflect::set(
                &result,
                &JsValue::from_str("values"),
                &js_sys::Float32Array::from(values.as_slice()),
            )?;
            js_sys::Reflect::set(
                &result,
                &JsValue::from_str("source"),
                &JsValue::from_str(&source),
            )?;
            Ok(result.into())
        }))
    }
}

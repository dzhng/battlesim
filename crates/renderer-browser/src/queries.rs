//! Explicit snapshots release scene borrows before asynchronous GPU mapping.
use super::*;
use renderer_core::probes::{grass, FogPoint, FogProbes};
use renderer_core::readback::RgbaReadback;
use wasm_bindgen_futures::future_to_promise;

fn field(object: &js_sys::Object, key: &str, value: impl Into<JsValue>) -> Result<(), JsValue> {
    js_sys::Reflect::set(object, &JsValue::from_str(key), &value.into()).map(|_| ())
}

#[wasm_bindgen]
impl Renderer {
    /// The returned dimensions and bytes describe this submission, even if
    /// the live canvas is resized while the snapshot is mapping.
    pub fn read_rgba(&self, high: bool) -> Result<js_sys::Promise, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before reading pixels"))?;
        let texture = if high {
            runtime
                .high_air
                .as_ref()
                .ok_or_else(|| js_error("no separate high-air output"))?
                .texture()
        } else {
            &runtime.targets.display_texture
        };
        let (width, height) = (texture.width(), texture.height());
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let read =
            RgbaReadback::encode(&self.gpu.device, &mut encoder, texture).map_err(js_error)?;
        self.gpu.queue.submit([encoder.finish()]);
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let bytes = read.read(&device).await.map_err(js_error)?;
            let image = js_sys::Object::new();
            field(&image, "width", width)?;
            field(&image, "height", height)?;
            field(&image, "rgba", js_sys::Uint8Array::from(bytes.as_slice()))?;
            Ok(image.into())
        }))
    }

    pub fn bake_impostor(&self, row: u32, spec: &str) -> Result<js_sys::Promise, JsValue> {
        let spec = serde_json::from_str::<Option<renderer_core::impostor::Spec>>(spec)
            .map_err(js_error)?
            .unwrap_or_default();
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before baking an impostor"))?;
        let request = runtime
            .models
            .impostor_request(
                &self.gpu.device,
                &self.gpu.queue,
                &runtime.pipelines,
                row,
                &spec,
            )
            .map_err(js_error)?;
        Ok(future_to_promise(async move {
            let atlas = request.read().await.map_err(js_error)?;
            let result = js_sys::Object::new();
            field(&result, "appearance", atlas.appearance)?;
            field(
                &result,
                "spec",
                serde_wasm_bindgen::to_value(&atlas.spec).map_err(js_error)?,
            )?;
            field(&result, "width", atlas.width)?;
            field(&result, "height", atlas.height)?;
            field(
                &result,
                "center",
                serde_wasm_bindgen::to_value(&atlas.center).map_err(js_error)?,
            )?;
            field(&result, "radius", atlas.radius)?;
            field(&result, "hash", atlas.hash)?;
            field(
                &result,
                "albedo",
                js_sys::Uint8Array::from(atlas.albedo.as_slice()),
            )?;
            field(
                &result,
                "normal",
                js_sys::Uint8Array::from(atlas.normal.as_slice()),
            )?;
            field(
                &result,
                "mask",
                js_sys::Uint8Array::from(atlas.mask.as_slice()),
            )?;
            Ok(result.into())
        }))
    }

    pub fn fog_probe(
        &mut self,
        presentation: &mut Presentation,
        json: &str,
    ) -> Result<js_sys::Promise, JsValue> {
        let points: Vec<FogPoint> = serde_json::from_str(json).map_err(js_error)?;
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| js_error("install presentation before probing fog"))?;
        let read = runtime
            .probe_fog(
                &self.gpu.device,
                &self.gpu.queue,
                presentation.core_mut(),
                &points,
            )
            .map_err(js_error)?;
        presentation.refresh_frame_header();
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let bytes = read.read_u8(&device).await.map_err(js_error)?;
            Ok(js_sys::Uint8Array::from(bytes.as_slice()).into())
        }))
    }

    pub fn fog_shape(&self, rows: Vec<f32>) -> Result<js_sys::Promise, JsValue> {
        let (rows, remainder) = rows.as_chunks::<4>();
        if !remainder.is_empty() {
            return Err(js_error("fog shape needs complete four-float rows"));
        }
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before probing fog"))?;
        let read = FogProbes::new(&runtime.pipelines)
            .shape(&self.gpu.device, &self.gpu.queue, rows)
            .map_err(js_error)?;
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let values = read.read_f32(&device).await.map_err(js_error)?;
            Ok(js_sys::Float32Array::from(values.as_slice()).into())
        }))
    }

    pub fn fog_tile_counts(&self) -> Result<js_sys::Promise, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before probing fog"))?;
        let read = FogProbes::tile_counts(&self.gpu.device, &self.gpu.queue, &runtime.fog)
            .map_err(js_error)?;
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let values = read.read_u32(&device).await.map_err(js_error)?;
            Ok(js_sys::Uint32Array::from(values.as_slice()).into())
        }))
    }

    pub fn fog_whole_flags(&self, count: u32) -> Result<js_sys::Promise, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before probing fog"))?;
        let read = FogProbes::whole_flags(
            &self.gpu.device,
            &self.gpu.queue,
            &runtime.fog,
            count as usize,
        )
        .map_err(js_error)?;
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let values = read.read_u8(&device).await.map_err(js_error)?;
            Ok(js_sys::Uint8Array::from(values.as_slice()).into())
        }))
    }

    pub fn grass_counts(&self) -> Result<js_sys::Promise, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before probing grass"))?;
        let read = runtime
            .grass
            .as_ref()
            .filter(|_| runtime.config().controls.grass)
            .map(|field| grass::queue_counts(&self.gpu.device, &self.gpu.queue, field))
            .transpose()
            .map_err(js_error)?;
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let counts = match read {
                Some(read) => read.read(&device).await.map_err(js_error)?,
                None => grass::GrassCounts::default(),
            };
            serde_wasm_bindgen::to_value(&counts).map_err(js_error)
        }))
    }

    pub fn grass_clumps(&self, presentation: &Presentation) -> Result<js_sys::Promise, JsValue> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| js_error("install presentation before probing grass"))?;
        let names = presentation
            .core()
            .ground()
            .map_or_else(Vec::new, |ground| ground.grass_kinds());
        let read = runtime
            .grass
            .as_ref()
            .filter(|_| runtime.config().controls.grass)
            .map(|field| grass::queue_clumps(&self.gpu.device, &self.gpu.queue, field, &names))
            .transpose()
            .map_err(js_error)?;
        let device = self.gpu.device.clone();
        Ok(future_to_promise(async move {
            let clumps = match read {
                Some(read) => read.read(&device).await.map_err(js_error)?,
                None => Vec::new(),
            };
            serde_wasm_bindgen::to_value(&clumps).map_err(js_error)
        }))
    }
}

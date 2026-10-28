//! Browser surface admission and presentation around the shared GPU frame.
use super::*;
use renderer_core::frame::{FrameInputs, OverlayFrame};

#[wasm_bindgen]
impl Renderer {
    pub fn render(&mut self) -> Result<(), JsValue> {
        let ground = self
            .ground
            .as_ref()
            .ok_or_else(|| js_error("load ground before rendering"))?;
        let models = self
            .models
            .as_ref()
            .ok_or_else(|| js_error("load model world before rendering"))?;
        let scenery = self
            .scenery
            .as_ref()
            .ok_or_else(|| js_error("load scenery world before rendering"))?;
        let presentation = self
            .presentation_snapshot
            .as_ref()
            .ok_or_else(|| js_error("upload frame before rendering"))?;
        let frame_view = record(&presentation.view, "view")?;
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("battle-frame"),
            });
        let surface = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface)
            | CurrentSurfaceTexture::Suboptimal(surface) => surface,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.recover_surface();
                return Ok(());
            }
            other => return Err(js_error(format!("surface unavailable: {other:?}"))),
        };
        if let Some(timer) = &mut self.gpu_timer {
            timer.begin(&mut encoder);
        }
        self.frame.encode(
            &FrameInputs {
                device: &self.device,
                queue: &self.queue,
                view: &frame_view,
                targets: &self.targets,
                sky: &self.sky,
                environment: &self.environment,
                shadow: &self.shared_shadow,
                ground,
                grass: self.grass.as_ref(),
                models,
                scenery,
                effects: self.shared_effects.resources(),
                fog: &self.shared_fog_sight,
                fog_mask: &self.shared_fog_mask,
                post: &self.shared_post,
                overlays: (self.overlay_vertex_count > 0).then_some(OverlayFrame {
                    resources: &self.shared_overlay_resources,
                    vertices: &self.overlay_vertices,
                    instances: &self.overlay_instances,
                    vertex_count: self.overlay_vertex_count,
                }),
            },
            &mut encoder,
        );
        let view = surface
            .texture
            .create_view(&TextureViewDescriptor::default());
        self.encode_surface(&mut encoder, &view);
        if let Some(timer) = &self.gpu_timer {
            timer.end(&mut encoder);
        }
        self.queue.submit([encoder.finish()]);
        if let Some(timer) = &mut self.gpu_timer {
            timer.submitted();
        }
        self.queue.present(surface);
        self.stats.shared_passes += 1;
        self.stats.shared_model_prepass = true;
        self.stats.shadow_passes += self.shared_shadow.active() as u32;
        self.stats.shadow_model_passes += self
            .stats
            .shadow_caster_batches
            .iter()
            .take(self.shared_shadow.active())
            .filter(|&&count| count > 0)
            .count() as u32;
        self.stats.frames += 1;
        Ok(())
    }

    /// Match WebGPU's queue completion boundary in the browser control.
    pub fn drain(&self) -> js_sys::Promise {
        js_sys::Promise::new(&mut |resolve, _reject| {
            self.queue.on_submitted_work_done(move || {
                let _ = resolve.call0(&JsValue::UNDEFINED);
            });
        })
    }

    fn encode_surface(&self, encoder: &mut CommandEncoder, surface: &TextureView) {
        let pipeline = self.display_pipelines.bound("composite");
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("display-surface"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: surface,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pipeline.set(&mut pass, &[("sampled", &self.display_group)]);
        pass.draw(0..3, 0..1);
    }
}

//! Surface admission before the runtime consumes canonical incremental work.
use super::*;
use renderer_core::runtime::OverlayGeometry;

#[wasm_bindgen]
impl Renderer {
    pub fn render_presentation(&mut self, presentation: &mut Presentation) -> Result<u32, JsValue> {
        let surface = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface)
            | CurrentSurfaceTexture::Suboptimal(surface) => surface,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.recover_surface();
                if let Some(runtime) = &mut self.runtime {
                    runtime.invalidate_statics();
                }
                return Ok(presentation.refresh_frame_header());
            }
            other => return Err(js_error(format!("surface unavailable: {other:?}"))),
        };
        if self.runtime.is_none() {
            self.install_presentation(presentation)?;
        }
        let runtime = self.runtime.as_mut().expect("installed runtime");
        let resized = runtime.resize(
            &self.device,
            [self.width, self.height],
            presentation.core_mut(),
        );
        runtime
            .prepare(&self.device, &self.queue, presentation.core_mut())
            .map_err(js_error)?;
        self.update_stats(presentation.core());
        if resized {
            self.rebind_display();
        }
        let runtime = self.runtime.as_ref().expect("installed runtime");
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("battle-frame"),
            });
        if let Some(timer) = &mut self.gpu_timer {
            timer.begin(&mut encoder);
        }
        runtime.encode(
            &self.device,
            &self.queue,
            &presentation.core().last_frame().view,
            &mut encoder,
            (self.overlay_vertex_count > 0).then_some(OverlayGeometry {
                vertices: &self.overlay_vertices,
                instances: &self.overlay_instances,
                vertex_count: self.overlay_vertex_count,
            }),
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
        self.stats.frames += 1;
        Ok(presentation.refresh_frame_header())
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
        pipeline.set(
            &mut pass,
            &[(
                "sampled",
                self.display_group.as_ref().expect("installed display"),
            )],
        );
        pass.draw(0..3, 0..1);
    }
}

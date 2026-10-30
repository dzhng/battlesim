//! Surface admission before the runtime consumes canonical incremental work.
use super::*;

#[wasm_bindgen]
impl Renderer {
    pub fn render_presentation(
        &mut self,
        presentation: &mut Presentation,
    ) -> Result<Option<u32>, JsValue> {
        let surface = acquire(&self.surface)?;
        let high = self
            .high_surface
            .as_ref()
            .map(|high| acquire(&high.surface))
            .transpose()?;
        let unavailable = surface.is_none() || high.as_ref().is_some_and(Option::is_none);
        if unavailable {
            self.recover_surface();
            return Ok(None);
        }
        let surface = surface.expect("acquired world output");
        let high = high.flatten();
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| js_error("install presentation and prepare assets before rendering"))?;
        let resized = runtime.resize(
            &self.gpu.device,
            &self.gpu.queue,
            [self.width, self.height],
            presentation.core_mut(),
        );
        runtime
            .prepare(&self.gpu.device, &self.gpu.queue, presentation.core_mut())
            .map_err(js_error)?;
        if resized {
            self.rebind_display();
        }
        let runtime = self.runtime.as_ref().expect("installed runtime");
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("battle-frame"),
            });
        if let Some(timer) = &mut self.gpu_timer {
            timer.begin(&mut encoder);
        }
        runtime.encode(
            &self.gpu.device,
            &self.gpu.queue,
            &presentation.core().last_frame().view,
            &mut encoder,
            self.player_meshes.draw(&self.overlay_instances),
        );
        let view = surface
            .texture
            .create_view(&TextureViewDescriptor::default());
        self.encode_surface(
            &mut encoder,
            &view,
            self.display_group.as_ref().expect("installed display"),
        );
        if let Some(high) = &high {
            let view = high.texture.create_view(&TextureViewDescriptor::default());
            self.encode_surface(
                &mut encoder,
                &view,
                self.high_surface
                    .as_ref()
                    .and_then(|s| s.group.as_ref())
                    .expect("installed high-air display"),
            );
        }
        if let Some(timer) = &self.gpu_timer {
            timer.end(&mut encoder);
        }
        self.gpu.queue.submit([encoder.finish()]);
        if let Some(timer) = &mut self.gpu_timer {
            timer.submitted();
        }
        self.gpu.queue.present(surface);
        if let Some(high) = high {
            self.gpu.queue.present(high);
        }
        self.frames += 1;
        Ok(Some(presentation.refresh_frame_header()))
    }

    /// Match WebGPU's queue completion boundary in the browser control.
    pub fn drain(&self) -> js_sys::Promise {
        js_sys::Promise::new(&mut |resolve, _reject| {
            self.gpu.queue.on_submitted_work_done(move || {
                let _ = resolve.call0(&JsValue::UNDEFINED);
            });
        })
    }

    fn encode_surface(
        &self,
        encoder: &mut CommandEncoder,
        surface: &TextureView,
        group: &BindGroup,
    ) {
        let pipeline = self.display_pipelines.bound("composite");
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("display-surface"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: surface,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pipeline.set(&mut pass, &[("sampled", group)]);
        pass.draw(0..3, 0..1);
    }
}

fn acquire(surface: &wgpu::Surface<'_>) -> Result<Option<SurfaceTexture>, JsValue> {
    match surface.get_current_texture() {
        CurrentSurfaceTexture::Success(texture) | CurrentSurfaceTexture::Suboptimal(texture) => {
            Ok(Some(texture))
        }
        CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => Ok(None),
        other => Err(js_error(format!("surface unavailable: {other:?}"))),
    }
}

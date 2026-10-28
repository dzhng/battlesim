use super::*;
use renderer_core::models::Surface;

struct BrowserFrame<'a> {
    renderer: &'a mut Renderer,
    surface: &'a TextureView,
}

impl renderer_core::frame::FrameInputs for BrowserFrame<'_> {
    fn encode_stage(
        &mut self,
        stage: renderer_core::frame::FrameStage,
        encoder: &mut CommandEncoder,
    ) {
        self.renderer.encode_stage(stage, encoder, self.surface);
    }
}

#[wasm_bindgen]
impl Renderer {
    pub fn render(&mut self) -> Result<(), JsValue> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("shared-renderer-frame"),
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
        let view = surface
            .texture
            .create_view(&TextureViewDescriptor::default());
        {
            let mut frame = BrowserFrame {
                renderer: self,
                surface: &view,
            };
            renderer_core::frame::encode_frame(&mut encoder, &mut frame);
        }
        self.stats.shared_passes += 1;
        self.shared_overlay_resources
            .encode_composite(&mut encoder, &self.targets, &view);
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface);
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

    fn encode_stage(
        &mut self,
        stage: renderer_core::frame::FrameStage,
        encoder: &mut CommandEncoder,
        surface: &TextureView,
    ) {
        use renderer_core::frame::FrameStage;
        match stage {
            FrameStage::SunShadow => self.encode_shadows(encoder),
            FrameStage::Depth => {
                self.depth.encode(
                    &super::render_world::BrowserWorld { renderer: self },
                    encoder,
                );
                self.stats.shared_model_prepass = self.models.is_some();
            }
            FrameStage::FogSight => {
                if self.fog_dispatch_active {
                    self.shared_fog_sight.dispatch(
                        encoder,
                        self.fog_rebuild_dispatch,
                        self.fog_whole_dispatch,
                    );
                }
            }
            FrameStage::World => {
                let inputs = super::render_world::BrowserWorld { renderer: self };
                renderer_core::world::encode_sky(&inputs, &self.targets, encoder);
                renderer_core::world::encode_world(
                    &inputs,
                    &self.targets,
                    encoder,
                    true,
                    true,
                    true,
                );
            }
            FrameStage::Effects => self.shared_effects.encode(encoder, &self.targets),
            FrameStage::FogMask => {
                self.shared_fog_mask.encode(encoder, &self.targets);
                self.encode_ghosts(encoder);
            }
            FrameStage::Overlays => {
                if self.overlay_vertex_count > 0 {
                    self.shared_overlay_resources.encode_geometry(
                        encoder,
                        &self.targets,
                        &self.production_camera_group,
                        &self.overlay_vertices,
                        &self.overlay_instances,
                        self.overlay_vertex_count,
                    );
                }
            }
            FrameStage::Post => self.shared_post.encode(encoder),
            FrameStage::FogRim => {
                let pipeline = self.display_pipelines.bound("composite");
                {
                    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("shared-display-composite"),
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
                self.shared_fog_mask
                    .encode_rim_to(encoder, &self.targets, surface);
            }
        }
    }

    fn encode_shadows(&mut self, encoder: &mut CommandEncoder) {
        let world_pipeline = self.production_pipelines.bound("worldCaster");
        let solid = self.production_pipelines.bound("modelCaster");
        let cutout = self.production_pipelines.bound("modelCasterCutout");
        let passes = self
            .shared_shadow
            .encode_cascades(encoder, |cascade, camera, pass| {
                let camera = ("typegpuCameraLayout", camera);
                world_pipeline.set(pass, &[camera]);
                pass.set_vertex_buffer(0, self.shadow_ground_vertices.slice(..));
                pass.set_vertex_buffer(1, self.overlay_instances.slice(..));
                pass.draw(0..self.shadow_ground_count, 0..1);
                if let Some(models) = &self.models {
                    for (pipeline, surface) in
                        [(&solid, Surface::Solid), (&cutout, Surface::Cutout)]
                    {
                        pipeline.set(pass, &[camera, models.group()]);
                        models.draw_casters(pass, surface, cascade);
                    }
                    if let Some(scenery) = &self.scenery {
                        scenery.draw_casters(pass, camera, models.statics(), cascade);
                    }
                }
            });
        self.stats.shadow_passes += passes;
        self.stats.shadow_model_passes += self
            .stats
            .shadow_caster_batches
            .iter()
            .take(self.shared_shadow.active())
            .filter(|&&count| count > 0)
            .count() as u32;
    }

    fn encode_ghosts(&self, encoder: &mut CommandEncoder) {
        let Some(models) = self.models.as_ref().filter(|models| models.has_ghosts()) else {
            return;
        };
        let pipeline = self.production_pipelines.bound("modelGhost");
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("placement-ghosts"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.targets.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&self.targets.hdr),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.targets.depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pipeline.set(
            &mut pass,
            &[
                ("typegpuCameraLayout", &self.production_camera_group),
                ("environment", self.environment.bindings.group()),
                models.group(),
            ],
        );
        models.draw_ghosts(&mut pass);
    }
}

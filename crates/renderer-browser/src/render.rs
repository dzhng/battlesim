#![allow(clippy::too_many_arguments)]

use super::*;
use renderer_core::shadow::ModelCaster;

struct ProductionDepth<'a> {
    depth: &'a TextureView,
    camera: &'a BindGroup,
    world_pipeline: renderer_core::pipelines::Bound,
    terrain: &'a Buffer,
    terrain_count: u32,
    static_world: &'a Buffer,
    static_count: u32,
    models: &'a Buffer,
    instances: &'a Buffer,
    model_count: u32,
    instance_count: u32,
    materials: &'a BindGroup,
}

impl renderer_core::depth::DepthInputs for ProductionDepth<'_> {
    fn depth_view(&self) -> &TextureView {
        self.depth
    }

    fn camera_group(&self) -> (&'static str, &BindGroup) {
        ("typegpuCameraLayout", self.camera)
    }

    fn draw_ground_depth(&self, pass: &mut RenderPass<'_>) {
        self.world_pipeline
            .set(pass, &[("typegpuCameraLayout", self.camera)]);
        pass.set_vertex_buffer(0, self.terrain.slice(..));
        pass.set_vertex_buffer(1, self.instances.slice(..));
        pass.draw(0..self.terrain_count, 0..1);
        if self.static_count > 0 {
            pass.set_vertex_buffer(0, self.static_world.slice(..));
            pass.draw(0..self.static_count, 0..1);
        }
    }

    fn draw_scenery_depth(&self, _pass: &mut RenderPass<'_>) {}

    fn draw_models_depth(
        &self,
        pass: &mut RenderPass<'_>,
        pipeline: &renderer_core::pipelines::Bound,
        _cutout: bool,
    ) {
        pipeline.set(
            pass,
            &[
                ("typegpuCameraLayout", self.camera),
                ("modelLayout", self.materials),
            ],
        );
        pass.set_vertex_buffer(0, self.models.slice(..));
        pass.set_vertex_buffer(1, self.instances.slice(..));
        pass.draw(0..self.model_count, 0..self.instance_count);
    }
}

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
    #[cfg(target_arch = "wasm32")]
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
        let mut frame = BrowserFrame {
            renderer: self,
            surface: &view,
        };
        renderer_core::frame::encode_frame(&mut encoder, &mut frame);
        frame.renderer.stats.shared_passes += 1;
        drop(frame);
        self.shared_overlay_resources
            .encode_composite(&mut encoder, &self.targets, &view);
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface);
        self.stats.frames += 1;
        self.stats.draw_calls += 5;
        Ok(())
    }

    fn encode_stage(
        &mut self,
        stage: renderer_core::frame::FrameStage,
        encoder: &mut CommandEncoder,
        surface: &TextureView,
    ) {
        match stage {
            renderer_core::frame::FrameStage::SunShadow => self.encode_shadows(encoder),
            renderer_core::frame::FrameStage::Depth => {
                if self.terrain_snapshot.is_some() {
                    self.encode_production_depth(encoder);
                    renderer_core::model_pass::encode_prepass(
                        encoder,
                        &self.targets,
                        &self.production_pipelines,
                        &self.production_camera_group,
                        self.material_bindings.bind_group(),
                        &self.production_model_vertices,
                        &self.production_model_instances,
                        self.production_model_vertex_count,
                        self.production_model_instance_count,
                        self.production_model_runs.as_deref(),
                    );
                    self.stats.shared_model_prepass = true;
                } else {
                    let ground = self.shared_ground.as_ref().unwrap_or(&self.model);
                    let fixture = Fixture {
                        depth: &self.targets.depth,
                        camera: &self.camera_group,
                        solid: &self.pipelines.bound("modelPrepass"),
                        world: &self.pipelines.bound("worldOpaque"),
                        model: ground,
                    };
                    encode_grouped(encoder, "shared-depth", |e| self.depth.encode(&fixture, e));
                }
            }
            renderer_core::frame::FrameStage::FogSight => {
                if self.fog_dispatch_active {
                    self.shared_fog_sight.dispatch(
                        encoder,
                        self.fog_rebuild_dispatch,
                        self.fog_whole_dispatch,
                    );
                }
            }
            renderer_core::frame::FrameStage::World => {
                if self.terrain_snapshot.is_some() {
                    if self.production_terrain_vertex_count > 0 {
                        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                            label: Some("shared-production-terrain-material"),
                            color_attachments: &[
                                Some(RenderPassColorAttachment {
                                    view: &self.targets.hdr_msaa,
                                    depth_slice: None,
                                    resolve_target: Some(&self.targets.lit),
                                    ops: Operations {
                                        load: LoadOp::Clear(Color::BLACK),
                                        store: StoreOp::Store,
                                    },
                                }),
                                Some(RenderPassColorAttachment {
                                    view: &self.targets.fog_mask_msaa,
                                    depth_slice: None,
                                    resolve_target: Some(&self.targets.fog_mask),
                                    ops: Operations {
                                        load: LoadOp::Clear(Color::BLACK),
                                        store: StoreOp::Store,
                                    },
                                }),
                            ],
                            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                                view: &self.targets.depth,
                                depth_ops: Some(Operations {
                                    load: LoadOp::Load,
                                    store: StoreOp::Store,
                                }),
                                stencil_ops: None,
                            }),
                            ..Default::default()
                        });
                        let terrain = self.production_pipelines.bound("worldTerrain");
                        terrain.set(
                            &mut pass,
                            &[
                                ("typegpuCameraLayout", &self.production_camera_group),
                                ("terrainLayout", self.terrain.bind_group()),
                                ("fogLayout", self.shared_fog_sight.group(0)),
                                ("environment", self.environment.bindings.group()),
                            ],
                        );
                        pass.set_vertex_buffer(0, self.production_terrain_vertices.slice(..));
                        pass.set_vertex_buffer(1, self.overlay_instances.slice(..));
                        pass.draw(0..self.production_terrain_vertex_count, 0..1);
                    }
                    if self.production_static_vertex_count > 0 {
                        let pipeline = self.production_pipelines.bound("worldOpaque");
                        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                            label: Some("shared-production-static-world"),
                            color_attachments: &[
                                Some(RenderPassColorAttachment {
                                    view: &self.targets.hdr_msaa,
                                    depth_slice: None,
                                    resolve_target: Some(&self.targets.lit),
                                    ops: Operations {
                                        load: LoadOp::Load,
                                        store: StoreOp::Store,
                                    },
                                }),
                                Some(RenderPassColorAttachment {
                                    view: &self.targets.fog_mask_msaa,
                                    depth_slice: None,
                                    resolve_target: Some(&self.targets.fog_mask),
                                    ops: Operations {
                                        load: LoadOp::Load,
                                        store: StoreOp::Store,
                                    },
                                }),
                            ],
                            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                                view: &self.targets.depth,
                                depth_ops: Some(Operations {
                                    load: LoadOp::Load,
                                    store: StoreOp::Store,
                                }),
                                stencil_ops: None,
                            }),
                            ..Default::default()
                        });
                        pipeline.set(
                            &mut pass,
                            &[
                                ("typegpuCameraLayout", &self.production_camera_group),
                                ("environment", self.environment.bindings.group()),
                                ("fogLayout", self.shared_fog_sight.group(1)),
                            ],
                        );
                        pass.set_vertex_buffer(0, self.production_static_vertices.slice(..));
                        pass.set_vertex_buffer(1, self.overlay_instances.slice(..));
                        pass.draw(0..self.production_static_vertex_count, 0..1);
                    }
                    renderer_core::model_pass::encode_opaque(
                        encoder,
                        &self.targets,
                        &self.production_pipelines,
                        &self.production_camera_group,
                        self.material_bindings.bind_group(),
                        self.shared_fog_sight.group(0),
                        self.environment.bindings.group(),
                        &self.production_model_vertices,
                        &self.production_model_instances,
                        self.production_model_vertex_count,
                        self.production_model_instance_count,
                        self.production_model_runs.as_deref(),
                    );
                } else {
                    let ground = self.shared_ground.as_ref().unwrap_or(&self.model);
                    let fixture = Fixture {
                        depth: &self.targets.depth,
                        camera: &self.camera_group,
                        solid: &self.pipelines.bound("modelPrepass"),
                        world: &self.pipelines.bound("worldOpaque"),
                        model: ground,
                    };
                    encode_grouped(encoder, "shared-world", |e| {
                        renderer_core::world::encode_sky(&fixture, &self.targets, e)
                    });
                    renderer_core::world::encode_world(
                        &fixture,
                        &self.targets,
                        encoder,
                        true,
                        true,
                        true,
                    );
                }
            }
            renderer_core::frame::FrameStage::Effects => {
                self.shared_effects.encode(encoder, &self.targets);
            }
            renderer_core::frame::FrameStage::FogMask => {
                self.shared_fog_mask.encode(encoder, &self.targets);
            }
            renderer_core::frame::FrameStage::Overlays => {
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
            renderer_core::frame::FrameStage::Post => self.shared_post.encode(encoder),
            renderer_core::frame::FrameStage::FogRim => {
                let pipeline = self.pipelines.bound("composite");
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

    fn encode_production_depth(&self, encoder: &mut CommandEncoder) {
        let inputs = ProductionDepth {
            depth: &self.targets.depth,
            camera: &self.production_camera_group,
            world_pipeline: self.production_pipelines.bound("worldPrepass"),
            terrain: &self.production_terrain_vertices,
            terrain_count: self.production_terrain_vertex_count,
            static_world: &self.production_static_vertices,
            static_count: self.production_static_vertex_count,
            models: &self.production_model_vertices,
            instances: &self.production_model_instances,
            model_count: self.production_model_vertex_count,
            instance_count: self.production_model_instance_count,
            materials: self.material_bindings.bind_group(),
        };
        self.production_depth.encode(&inputs, encoder);
    }

    fn encode_shadows(&mut self, encoder: &mut CommandEncoder) {
        let pipeline = self.production_pipelines.bound("worldCaster");
        let passes = self.shared_shadow.encode_ground(
            encoder,
            &pipeline,
            &self.shadow_ground_vertices,
            &self.overlay_instances,
            self.shadow_ground_count,
        );
        self.stats.shadow_passes = self.stats.shadow_passes.saturating_add(passes);
        let model_pipeline = self.production_pipelines.bound("modelCaster");
        let model_passes = self.shared_shadow.encode_models(
            encoder,
            ModelCaster {
                pipeline: &model_pipeline,
                materials: self.material_bindings.bind_group(),
                vertices: &self.production_model_vertices,
                instances: &self.production_model_instances,
                vertex_count: self.production_model_vertex_count,
                routed: &self.shadow_routing,
            },
        );
        self.stats.shadow_model_passes =
            self.stats.shadow_model_passes.saturating_add(model_passes);
    }
}

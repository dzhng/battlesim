#![allow(clippy::too_many_arguments)]

use super::*;
use renderer_core::shadow::ModelCaster;

#[wasm_bindgen]
impl Renderer {
    #[cfg(target_arch = "wasm32")]
    pub fn render(&mut self) -> Result<(), JsValue> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("shared-renderer-frame"),
            });
        self.encode_shadows(&mut encoder);
        if self.terrain_snapshot.is_some() {
            self.encode_production_depth(&mut encoder);
            renderer_core::model_pass::encode_prepass(
                &mut encoder,
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
            encode_grouped(&mut encoder, "shared-depth", |e| {
                self.depth.encode(&fixture, e)
            });
            encode_grouped(&mut encoder, "shared-world", |e| {
                renderer_core::world::encode_sky(&fixture, &self.targets, e)
            });
            renderer_core::world::encode_world(
                &fixture,
                &self.targets,
                &mut encoder,
                true,
                true,
                true,
            );
        }
        // Fog sight maps are read by the terrain and model fragment shaders;
        // build them after depth has established the view and before any
        // production world material samples them.
        if self.fog_dispatch_active {
            self.shared_fog_sight.dispatch(
                &mut encoder,
                self.fog_rebuild_dispatch,
                self.fog_whole_dispatch,
            );
        }
        // A production frame owns the terrain draw below; the fixture world
        // is encoded only on the compatibility path above.
        if self.terrain_snapshot.is_some() && self.production_terrain_vertex_count > 0 {
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
        if self.terrain_snapshot.is_some() && self.production_static_vertex_count > 0 {
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
            &mut encoder,
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
        if self.overlay_vertex_count > 0 {
            self.shared_overlay_resources.encode_geometry(
                &mut encoder,
                &self.targets,
                &self.production_camera_group,
                &self.overlay_vertices,
                &self.overlay_instances,
                self.overlay_vertex_count,
            );
        }
        self.shared_fog_mask.encode(&mut encoder, &self.targets);
        self.shared_post.encode(&mut encoder);
        self.stats.shared_passes += 1;
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
        self.shared_fog_mask
            .encode_rim_to(&mut encoder, &self.targets, &view);
        let pipeline = self.pipelines.bound("composite");
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shared-display-composite"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
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
        self.shared_overlay_resources
            .encode_composite(&mut encoder, &self.targets, &view);
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface);
        self.stats.frames += 1;
        self.stats.draw_calls += 5;
        Ok(())
    }

    /// Depth for the real presentation terrain and static world.  The
    /// compatibility fixture depth pass uses its own camera and 28-byte
    /// fixture model, which cannot establish the depth consumed by the
    /// production 40-byte terrain stream.  Keep this production owner beside
    /// the shared frame ordering so the terrain color pass reads the same
    /// reverse-Z coverage it was authored against.
    fn encode_production_depth(&self, encoder: &mut CommandEncoder) {
        let pipeline = self.production_pipelines.bound("worldPrepass");
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("shared-production-terrain-depth"),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.targets.depth,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pipeline.set(
            &mut pass,
            &[("typegpuCameraLayout", &self.production_camera_group)],
        );
        pass.set_vertex_buffer(0, self.production_terrain_vertices.slice(..));
        pass.set_vertex_buffer(1, self.overlay_instances.slice(..));
        pass.draw(0..self.production_terrain_vertex_count, 0..1);
        if self.production_static_vertex_count > 0 {
            pass.set_vertex_buffer(0, self.production_static_vertices.slice(..));
            pass.draw(0..self.production_static_vertex_count, 0..1);
        }
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

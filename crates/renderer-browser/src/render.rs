#![allow(clippy::too_many_arguments)]

use super::*;

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
        renderer_core::world::encode_world(&fixture, &self.targets, &mut encoder, true, true, true);
        production_model::encode_prepass(
            &mut encoder,
            &self.targets,
            &self.production_pipelines,
            &self.production_camera_group,
            self.material_bindings.bind_group(),
            &self.production_model_vertices,
            &self.production_model_instances,
            self.production_model_vertex_count,
        );
        self.stats.shared_model_prepass = true;
        production_model::encode_opaque(
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
        if self.fog_dispatch_active {
            self.shared_fog_sight.dispatch(
                &mut encoder,
                self.fog_rebuild_dispatch,
                self.fog_whole_dispatch,
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
    }
}

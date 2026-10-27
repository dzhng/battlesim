use super::*;
use js_sys::Date;

impl ProductionRenderer {
    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let (
            depth,
            world,
            fog_mask,
            shadows,
            world_view,
            fog_view,
            depth_view,
            shadow_views,
            bytes,
        ) = targets(device, width, height, self.target_format);
        self.depth = depth;
        self.world = world;
        self.fog_mask = fog_mask;
        self.shadows = shadows;
        self.world_view = world_view;
        self.fog_view = fog_view;
        self.depth_view = depth_view;
        self.shadow_views = shadow_views;
        self.width = width;
        self.height = height;
        self.stats.target_bytes = bytes;
        // The post bind group owns views of the old targets. It is rebuilt by
        // the caller through `rebind_composite` immediately after resize.
    }

    pub fn rebind_composite(&mut self, device: &Device) {
        let layout = self.composite_pipeline.get_bind_group_layout(0);
        self.composite_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-composite-bind"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.world_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.fog_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
    }

    fn upload(
        device: &Device,
        queue: &Queue,
        slot: &mut Option<Buffer>,
        label: &str,
        bytes: &[u8],
        usage: wgpu::BufferUsages,
    ) {
        if slot.as_ref().is_none_or(|b| b.size() < bytes.len() as u64) {
            *slot = Some(buffer(
                device,
                label,
                bytes.len().max(4) as u64,
                usage | wgpu::BufferUsages::COPY_DST,
            ));
        }
        queue.write_buffer(slot.as_ref().unwrap(), 0, bytes);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface: &Surface<'static>,
        terrain: &[f32],
        model_vertices: &[f32],
        model_indices: &[u32],
        instances: &[f32],
        effects: &[f32],
        texture: &[u8],
        texture_width: u32,
        texture_height: u32,
        camera: &[f32],
        shadow_cameras: &[f32],
        environment: &[f32],
        fog_params: &[f32],
    ) -> Result<f64, String> {
        if !terrain.len().is_multiple_of(10)
            || !model_vertices.len().is_multiple_of(9)
            || model_indices.is_empty()
            || !instances.len().is_multiple_of(8)
            || !effects.len().is_multiple_of(8)
            || camera.len() < 16
            || shadow_cameras.len() < CASCADES * 16
            || environment.len() < 16
            || fog_params.len() < 8
        {
            return Err("invalid production workload record".into());
        }
        let started = Date::now();
        Self::upload(
            device,
            queue,
            &mut self.terrain,
            "one-renderer-production-terrain",
            bytemuck::cast_slice(terrain),
            wgpu::BufferUsages::VERTEX,
        );
        self.stats.uploaded_bytes += (terrain.len() * 4) as u64;
        Self::upload(
            device,
            queue,
            &mut self.model_vertex,
            "one-renderer-production-model-vertices",
            bytemuck::cast_slice(model_vertices),
            wgpu::BufferUsages::VERTEX,
        );
        self.stats.uploaded_bytes += (model_vertices.len() * 4) as u64;
        Self::upload(
            device,
            queue,
            &mut self.model_index,
            "one-renderer-production-model-indices",
            bytemuck::cast_slice(model_indices),
            wgpu::BufferUsages::INDEX,
        );
        self.stats.uploaded_bytes += (model_indices.len() * 4) as u64;
        Self::upload(
            device,
            queue,
            &mut self.model_instance,
            "one-renderer-production-model-instances",
            bytemuck::cast_slice(instances),
            wgpu::BufferUsages::VERTEX,
        );
        self.stats.uploaded_bytes += (instances.len() * 4) as u64;
        if !effects.is_empty() {
            Self::upload(
                device,
                queue,
                &mut self.effects,
                "one-renderer-production-effects",
                bytemuck::cast_slice(effects),
                wgpu::BufferUsages::VERTEX,
            );
            self.stats.uploaded_bytes += (effects.len() * 4) as u64;
        }
        queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera[..16]));
        queue.write_buffer(
            &self.environment,
            0,
            bytemuck::cast_slice(&environment[..16]),
        );
        queue.write_buffer(&self.fog, 0, bytemuck::cast_slice(&fog_params[..8]));
        if self.albedo.is_none()
            || self.albedo_width != texture_width
            || self.albedo_height != texture_height
        {
            self.albedo = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("one-renderer-production-albedo"),
                size: Extent3d {
                    width: texture_width,
                    height: texture_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            self.albedo_width = texture_width;
            self.albedo_height = texture_height;
            self.stats.texture_bytes += texture_width as u64 * texture_height as u64 * 4;
        }
        let albedo = self.albedo.as_ref().unwrap();
        queue.write_texture(
            albedo.as_image_copy(),
            texture,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture_width * 4),
                rows_per_image: Some(texture_height),
            },
            Extent3d {
                width: texture_width,
                height: texture_height,
                depth_or_array_layers: 1,
            },
        );
        self.stats.buffer_bytes = self.terrain.as_ref().map_or(0, Buffer::size)
            + self.model_vertex.as_ref().map_or(0, Buffer::size)
            + self.model_index.as_ref().map_or(0, Buffer::size)
            + self.model_instance.as_ref().map_or(0, Buffer::size)
            + self.effects.as_ref().map_or(0, Buffer::size)
            + self.camera.size()
            + self.shadow_camera.size()
            + self.environment.size()
            + self.fog.size();
        let material_layout = self.model_world.get_bind_group_layout(2);
        self.material_bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("one-renderer-production-material-bind"),
            layout: &material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &albedo.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            other => return Err(format!("wgpu surface unavailable: {other:?}")),
        };
        let output = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("one-renderer-production-frame"),
        });
        for cascade in 0..CASCADES {
            queue.write_buffer(
                &self.shadow_camera,
                0,
                bytemuck::cast_slice(&shadow_cameras[cascade * 16..cascade * 16 + 16]),
            );
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-production-shadow-pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shadow_views[cascade],
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.terrain_shadow);
            pass.set_bind_group(0, &self.shadow_camera_bind, &[]);
            pass.set_vertex_buffer(0, self.terrain.as_ref().unwrap().slice(..));
            pass.draw(0..(terrain.len() / 10) as u32, 0..1);
            pass.set_pipeline(&self.model_shadow);
            pass.set_vertex_buffer(0, self.model_vertex.as_ref().unwrap().slice(..));
            pass.set_vertex_buffer(1, self.model_instance.as_ref().unwrap().slice(..));
            pass.set_index_buffer(
                self.model_index.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(
                0..model_indices.len() as u32,
                0,
                0..instances.len() as u32 / 8,
            );
        }
        self.stats.shadow_passes += CASCADES as u32;
        self.stats.draw_calls += (CASCADES * 2) as u32;
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-production-world-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.world_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.terrain_world);
            pass.set_bind_group(0, &self.camera_bind, &[]);
            pass.set_bind_group(1, &self.environment_bind, &[]);
            pass.set_vertex_buffer(0, self.terrain.as_ref().unwrap().slice(..));
            pass.draw(0..(terrain.len() / 10) as u32, 0..1);
            pass.set_pipeline(&self.model_world);
            pass.set_bind_group(0, &self.camera_bind, &[]);
            pass.set_bind_group(1, &self.environment_bind, &[]);
            pass.set_bind_group(2, self.material_bind.as_ref().unwrap(), &[]);
            pass.set_vertex_buffer(0, self.model_vertex.as_ref().unwrap().slice(..));
            pass.set_vertex_buffer(1, self.model_instance.as_ref().unwrap().slice(..));
            pass.set_index_buffer(
                self.model_index.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(
                0..model_indices.len() as u32,
                0,
                0..instances.len() as u32 / 8,
            );
        }
        self.stats.world_passes += 1;
        self.stats.draw_calls += 2;
        if !effects.is_empty() {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-production-effects-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.world_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.effects_pipeline);
            pass.set_bind_group(0, &self.effect_bind, &[]);
            pass.set_vertex_buffer(1, self.effects.as_ref().unwrap().slice(..));
            pass.draw(0..6, 0..effects.len() as u32 / 8);
            self.stats.effect_passes += 1;
            self.stats.draw_calls += 1;
        }
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-production-fog-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.fog_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fog_pipeline);
            pass.set_bind_group(0, &self.fog_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("one-renderer-production-post-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_bind, &[]);
            pass.set_bind_group(1, &self.fog_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        self.stats.fog_passes += 1;
        self.stats.post_passes += 1;
        self.stats.draw_calls += 2;
        self.stats.frames += 1;
        queue.submit([encoder.finish()]);
        queue.present(frame);
        self.last_ms = Date::now() - started;
        Ok(self.last_ms)
    }

    pub fn stats_json(&self) -> String {
        self.stats.json(self.last_ms)
    }
}

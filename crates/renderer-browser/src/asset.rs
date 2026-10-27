use super::*;

#[derive(Clone)]
pub(super) struct AssetSnapshot {
    pub(super) name: String,
    pub(super) bundle: Vec<u8>,
    pub(super) textures: Vec<Vec<u8>>,
}

#[wasm_bindgen]
impl Renderer {
    /// Parse one fetched BGAT bundle through the native client's shared asset
    /// reader and upload its first authored texture to this browser device.
    /// The returned JSON is the exact boundary accounting used by the spike.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen]
    pub fn load_bundle(
        &mut self,
        name: String,
        bundle_bytes: js_sys::Uint8Array,
        textures: js_sys::Array,
    ) -> Result<String, JsValue> {
        let bundle_bytes = bundle_bytes.to_vec();
        let mut texture_bytes = Vec::with_capacity(textures.length() as usize);
        for value in textures.iter() {
            texture_bytes.push(js_sys::Uint8Array::new(&value).to_vec());
        }
        self.load_bundle_bytes(name, bundle_bytes, texture_bytes)
    }

    pub(super) fn load_bundle_bytes(
        &mut self,
        name: String,
        bundle_bytes: Vec<u8>,
        texture_bytes: Vec<Vec<u8>>,
    ) -> Result<String, JsValue> {
        let mut owned = Vec::with_capacity(texture_bytes.len());
        for bytes in texture_bytes {
            owned.push((format!("texture-{}", owned.len()), bytes));
        }
        let refs: Vec<(&str, &[u8])> = owned
            .iter()
            .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
            .collect();
        let library = Library::from_bundle(&name, &bundle_bytes, &refs).map_err(js_error)?;
        let bundle = library.bundle(&name).map_err(js_error)?;
        let mut rows = Vec::new();
        for material in bundle.materials.iter().take(16) {
            let layer = |key: &str| {
                material
                    .textures
                    .get(key)
                    .copied()
                    .filter(|index| *index == 0)
                    .map_or(-1.0, |_| 0.0)
            };
            let cutoff = match material.coverage {
                native_client::bundle::Coverage::Cutout { cutoff } => cutoff,
                _ => 0.0,
            };
            let wear = material.wear.unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let base = material.base_color;
            rows.extend_from_slice(&[
                base[0] as f32,
                base[1] as f32,
                base[2] as f32,
                base[3] as f32,
                material.metallic,
                material.roughness,
                material.tint,
                material.colour_scale.unwrap_or(1.0),
                layer("albedo"),
                layer("normal"),
                layer("orm"),
                if material.wear.is_some() { 1.0 } else { 0.0 },
                wear[0],
                wear[1],
                wear[2],
                wear[3],
                cutoff,
                layer("normal"),
                0.0,
                0.0,
            ]);
        }
        if !rows.is_empty() {
            self.queue
                .write_buffer(&self.material_rows, 0, bytemuck::cast_slice(&rows));
        }
        let vertices: usize = match &bundle.shape {
            native_client::bundle::Shape::Skinned(tiers) => {
                tiers.iter().map(|m| m.vertices()).sum()
            }
            native_client::bundle::Shape::Articulated(nodes)
            | native_client::bundle::Shape::Static(nodes) => nodes
                .iter()
                .flat_map(|node| node.tiers.iter())
                .map(|m| m.vertices())
                .sum(),
        };
        let indices: usize = match &bundle.shape {
            native_client::bundle::Shape::Skinned(tiers) => {
                tiers.iter().map(|m| m.indices.len()).sum()
            }
            native_client::bundle::Shape::Articulated(nodes)
            | native_client::bundle::Shape::Static(nodes) => nodes
                .iter()
                .flat_map(|node| node.tiers.iter())
                .map(|m| m.indices.len())
                .sum(),
        };
        if let Some(texture) = bundle.textures.first() {
            let size = Extent3d {
                width: texture.size,
                height: texture.size,
                depth_or_array_layers: 1,
            };
            let gpu = self.device.create_texture(&TextureDescriptor {
                label: Some("shared-asset-albedo"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if texture.srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &gpu,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &texture.levels[0],
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(texture.size * 4),
                    rows_per_image: Some(texture.size),
                },
                size,
            );
            self.asset_texture = Some(gpu);
            let albedo = array_view(self.asset_texture.as_ref().expect("asset texture uploaded"));
            let surface = array_view(&self.material_surface);
            self.material_bindings.rebuild(
                &self.device,
                self.production_pipelines.layout("modelLayout"),
                &self.material_palette,
                &self.material_rows,
                &albedo,
                &surface,
                &self.material_sampler,
            );
            self.stats.shared_materials = true;
        }
        let mesh = match &bundle.shape {
            native_client::bundle::Shape::Skinned(tiers) => {
                tiers.iter().find(|mesh| !mesh.indices.is_empty())
            }
            native_client::bundle::Shape::Articulated(nodes)
            | native_client::bundle::Shape::Static(nodes) => nodes
                .iter()
                .flat_map(|node| node.tiers.iter())
                .find(|mesh| !mesh.indices.is_empty()),
        };
        if let Some(mesh) = mesh {
            let mut material_ids = vec![0u16; mesh.indices.len()];
            for draw in &mesh.draws {
                for id in &mut material_ids[draw.first..draw.first + draw.count] {
                    *id = draw.material as u16;
                }
            }
            let (production_vertices, production_count) = production_model::mesh_vertices(
                &mesh.positions,
                &mesh.colors,
                &mesh.indices,
                &material_ids,
            )
            .map_err(js_error)?;
            self.production_model_vertices = renderer_core::buffer(
                &self.device,
                "shared-production-asset-vertices",
                BufferUsages::VERTEX,
                &production_vertices,
            );
            self.production_model_vertex_count = production_count;
            // Keep the diagnostic display fixture's camera-compatible stream.
            // The authored 48-byte production stream is consumed by the
            // prepass; its final lit draw remains disabled until the
            // presentation camera/environment groups are complete.
            self.stats.model_vertices = production_count;
        }
        let report = format!(
            r#"{{"name":"{}","bundleBytes":{},"textureBytes":{},"textures":{},"materials":{},"vertices":{},"indices":{}}}"#,
            name,
            bundle_bytes.len(),
            owned.iter().map(|(_, b)| b.len()).sum::<usize>(),
            bundle.textures.len(),
            bundle.materials.len(),
            vertices,
            indices
        );
        self.asset_report = Some(report.clone());
        self.asset_snapshot = Some(AssetSnapshot {
            name,
            bundle: bundle_bytes,
            textures: owned.into_iter().map(|(_, bytes)| bytes).collect(),
        });
        Ok(report)
    }
}

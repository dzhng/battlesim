//! The effects (`effects/effectPass.ts`): the frame's effect instances
//! (`Presentation::effects`) drawn into the resolved lit world after the
//! world pass and before the fog mask, testing against the world's depth by
//! reading it, and lowering the fog mask's coverage by their own strength.
//! The flipbooks are the browser's sheets, one layer each of a mipmapped
//! sRGB array, premultiplied as the browser's image decode premultiplies
//! them; lit smoke takes the environment's sun, PMREM and cast lights.
use presentation::records::Effect;
use wgpu::*;

use crate::environment::Environment;
use crate::frame::Targets;
use crate::pipelines::Pipelines;

/// The browser's flipbook list: its imports name each sheet, and its
/// `FLIPBOOKS` rows give the layer order.
const FLIPBOOKS_TS: &str =
    include_str!("../../../packages/battle-renderer/src/effects/flipbooks.ts");
const INSTANCE_BYTES: u64 = std::mem::size_of::<Effect>() as u64;

pub struct Effects {
    resources: renderer_core::effects::EffectsResources,
}

/// The sheets' file names in layer order, read from `flipbooks.ts`.
fn sheet_files() -> Vec<String> {
    let imports: Vec<(&str, &str)> = FLIPBOOKS_TS
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("import ")?;
            let (name, from) = rest.split_once(" from ")?;
            let file = from.split("/effects/").nth(1)?.split('?').next()?;
            Some((name.trim(), file))
        })
        .collect();
    let files: Vec<String> = FLIPBOOKS_TS
        .split("{ url: ")
        .skip(1)
        .filter_map(|row| {
            let name = row.split(',').next()?.trim();
            imports
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, f)| f.to_string())
        })
        .collect();
    assert!(!files.is_empty(), "flipbooks.ts lists no sheet");
    files
}

/// A `side`² sheet's RGBA bytes, each colour times its alpha in its encoded
/// value, as `createImageBitmap(…, {premultiplyAlpha: "premultiply"})` gives them.
fn premultiplied(path: &str, side: u32) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{path} is not an image (an LFS pointer?): {e}"))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{path}: {e}"))?;
    if (info.width, info.height) != (side, side) || info.color_type != png::ColorType::Rgba {
        return Err(format!(
            "{path} is {}×{} {:?}, not {side}² RGBA",
            info.width, info.height, info.color_type
        ));
    }
    for px in buf.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
    Ok(buf)
}

/// The next mip: each texel the mean of the 2×2 it covers.
fn halve(level: &[u8], side: usize) -> Vec<u8> {
    let half = side / 2;
    let mut out = vec![0; half * half * 4];
    for y in 0..half {
        for x in 0..half {
            for c in 0..4 {
                let at = |dx: usize, dy: usize| {
                    u32::from(level[((2 * y + dy) * side + 2 * x + dx) * 4 + c])
                };
                let sum = at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1);
                out[(y * half + x) * 4 + c] = ((sum + 2) / 4) as u8;
            }
        }
    }
    out
}

/// Every `sheet`² flipbook into one layer of a mipmapped sRGB 2D array.
fn atlas(
    device: &Device,
    queue: &Queue,
    sheets: &std::path::Path,
    sheet: u32,
) -> Result<TextureView, String> {
    let files = sheet_files();
    let levels = sheet.ilog2() + 1;
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("effect-atlas"),
        size: Extent3d {
            width: sheet,
            height: sheet,
            depth_or_array_layers: files.len() as u32,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, file) in files.iter().enumerate() {
        let mut level = premultiplied(&sheets.join(file).to_string_lossy(), sheet)?;
        for mip in 0..levels {
            let side = sheet >> mip;
            if mip > 0 {
                level = halve(&level, (side * 2) as usize);
            }
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: TextureAspect::All,
                },
                &level,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(side * 4),
                    rows_per_image: None,
                },
                Extent3d {
                    width: side,
                    height: side,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    Ok(texture.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    }))
}

impl Effects {
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        environment: &Environment,
        targets: &Targets,
        sheets: &std::path::Path,
    ) -> Result<Self, String> {
        let atlas = atlas(device, queue, sheets, pipelines.sizes.effect_sheet)?;
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("effects"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });
        let resources = renderer_core::effects::EffectsResources::new(
            device,
            pipelines,
            &[
                environment.camera_buffer.as_entire_binding(),
                BindingResource::TextureView(&targets.depth),
                BindingResource::TextureView(&atlas),
                BindingResource::Sampler(&sampler),
                environment.data.as_entire_binding(),
                BindingResource::TextureView(&environment.pmrem),
                environment.cast_lights.as_entire_binding(),
            ],
            INSTANCE_BYTES,
            1024 * INSTANCE_BYTES,
        );
        Ok(Self { resources })
    }

    /// This frame's instances, in draw order, replacing the last.
    pub fn prepare(&mut self, device: &Device, queue: &Queue, effects: &[Effect]) {
        self.resources
            .prepare(device, queue, bytemuck::cast_slice(effects));
    }

    pub fn resources(&self) -> &renderer_core::effects::EffectsResources {
        &self.resources
    }

    /// Instances drawn this frame.
    pub fn count(&self) -> u32 {
        self.resources.count()
    }
}

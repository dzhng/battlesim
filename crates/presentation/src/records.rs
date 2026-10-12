//! The records a renderer reads (18b), defined once. Each is `#[repr(C)]`,
//! a multiple of 16 bytes, of 4-byte scalars or arrays of them, so the same
//! bytes are a WGSL struct, a vertex instance attribute and a typed array.
//! `records_are_current` renders every layout into
//! `packages/battle-renderer/src/records.gen.ts`; nothing is kept by hand.

/// `u32::MAX`: no row, no id.
pub const NONE: u32 = u32::MAX;

/// What a field holds, for the generated schemas.
pub trait Field {
    /// `"f32"` or `"u32"`.
    const SCALAR: &'static str;
    const COUNT: usize;
}
impl Field for f32 {
    const SCALAR: &'static str = "f32";
    const COUNT: usize = 1;
}
impl Field for u32 {
    const SCALAR: &'static str = "u32";
    const COUNT: usize = 1;
}
impl<const N: usize> Field for [f32; N] {
    const SCALAR: &'static str = "f32";
    const COUNT: usize = N;
}
impl<const N: usize> Field for [u32; N] {
    const SCALAR: &'static str = "u32";
    const COUNT: usize = N;
}

pub struct FieldLayout {
    pub name: &'static str,
    pub offset: usize,
    pub scalar: &'static str,
    pub count: usize,
    pub doc: &'static str,
}

/// How a renderer binds a record: a vertex instance attribute, a storage
/// buffer struct, or plain words it reads on the CPU.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Vertex,
    Storage,
    Plain,
}

pub struct Layout {
    pub name: &'static str,
    pub doc: &'static str,
    pub binding: Binding,
    pub size: usize,
    pub fields: &'static [FieldLayout],
}

macro_rules! records {
    ($($(#[doc = $doc:literal])* $binding:ident $name:ident {
        $($(#[doc = $fdoc:literal])* $field:ident: $ty:ty),* $(,)?
    })*) => {
        $(
            $(#[doc = $doc])*
            #[repr(C)]
            #[derive(Clone, Copy, Debug, PartialEq)]
            pub struct $name { $($(#[doc = $fdoc])* pub $field: $ty),* }
            // SAFETY: `repr(C)`, every field a 4-byte scalar or an array of
            // them, and the size a multiple of 16 (asserted): no padding.
            unsafe impl bytemuck::Zeroable for $name {}
            unsafe impl bytemuck::Pod for $name {}
            impl Default for $name {
                fn default() -> Self {
                    bytemuck::Zeroable::zeroed()
                }
            }
            const _: () = assert!(std::mem::size_of::<$name>() % 16 == 0);
            const _: () = assert!(
                std::mem::size_of::<$name>() == 0 $(+ std::mem::size_of::<$ty>())*
            );
            impl $name {
                pub const LAYOUT: Layout = Layout {
                    name: stringify!($name),
                    doc: concat!($($doc),*),
                    binding: Binding::$binding,
                    size: std::mem::size_of::<$name>(),
                    fields: &[$(FieldLayout {
                        name: stringify!($field),
                        offset: std::mem::offset_of!($name, $field),
                        scalar: <$ty as Field>::SCALAR,
                        count: <$ty as Field>::COUNT,
                        doc: concat!($($fdoc),*),
                    }),*],
                };
            }
        )*
        /// Every record's layout, in declaration order.
        pub const LAYOUTS: &[Layout] = &[$($name::LAYOUT),*];
    };
}

records! {
    /// Where a frame's sets lie, as pointers into the module's memory and
    /// record counts (the WASM binding fills them; native reads the
    /// `Frame` directly).
    Plain FrameHeader {
        /// Presentation time, seconds on the sim clock.
        clock: f32,
        /// The publication drawn.
        tick: u32,
        /// Frames framed so far.
        index: u32,
        /// How far round a surface every cast light reaches past facing it, 0..1.
        light_wrap: f32,
        batches: [u32; 2],
        instances: [u32; 2],
        palettes: [u32; 2],
        effects: [u32; 2],
        lights: [u32; 2],
        picks: [u32; 2],
        emitters: [u32; 2],
        counts: [u32; 2],
    }

    /// Where the world's sets lie (pointer, count), as `FrameHeader`.
    Plain WorldHeader {
        appearances: [u32; 2],
        /// The UTF-8 arena names point into (pointer, bytes).
        strings: [u32; 2],
    }

    /// One drawable an instance may take: an appearance's posed body or
    /// first state, a way it lies dead, or one of its states.
    Plain Appearance {
        /// Byte offset into `strings` of the catalog name the renderer loads.
        name: u32,
        name_len: u32,
        /// The static state drawn (offset, bytes); 0 bytes: the posed body,
        /// or a static bundle's first state.
        state: u32,
        state_len: u32,
        /// The way of lying (`corpse_poses` index) for a corpse; `NONE` otherwise.
        lies: u32,
        /// Joints a palette for it holds (0: rigid).
        skin: u32,
        /// 1 for a drawable of the static world (a building's kit module, a
        /// tree, a prop), whose batches a frame writes only when they change
        /// and names unchanged otherwise; 0 for one the frame poses every time.
        placed: u32,
        /// What draws it: 0 a model or a building's kit module; scenery 1
        /// the forest, 2 a tree line's shrubs, 3 the backdrop, 4 forest-floor
        /// dressing (its seed is its height over the size it fades at), 5 a
        /// felled tree (two records an instance: the tree with its fall's
        /// way, angle and hinge; then its press up-and-down and across,
        /// trunk, landing and crown base), 6 a felled tree's stump
        /// (`Presentation::stump_mesh`); a prop drawn as a model with the
        /// units, 7 fogged face by face, 8 a surface movers stand on, which
        /// takes the ground paint.
        draws: u32,
    }

    /// What one frame draws of one drawable at one tier.
    Plain Batch {
        /// Stable across frames; the renderer keys its buffers by it.
        key: u32,
        /// `World.appearances` row.
        appearance: u32,
        /// Detail tier, 0 finest; 4 is the impostor card.
        tier: u32,
        /// 0 drawn in view; 1 cast into the sun's cascades; 2 a translucent
        /// ghost (no shadow, no depth write).
        pass: u32,
        /// Casters: bit n set if it reaches cascade n.
        cascades: u32,
        /// Changes when the contents change.
        version: u32,
        /// First `Instance`, or `NONE` when unchanged since `version`.
        first: u32,
        count: u32,
    }

    /// One drawn model. Palette indices and the card layer are integral floats.
    Vertex Instance {
        /// x, y, z (m, sim) and yaw (rad, counter-clockwise from +x).
        placement: [f32; 4],
        /// First `Joint` of its palette (0: the identity), the left and right
        /// track scrolls (fractions of a link), and the x-ray or ghost colour
        /// packed as `r·65536 + g·256 + b` bytes.
        data: [f32; 4],
        /// The side's tint (linear rgb) on tint-masked surfaces, and the
        /// impostor atlas layer a card draws (the renderer's; −1 for a mesh).
        tint: [f32; 4],
        /// Per-axis scale in its own frame, and the x-ray or ghost alpha.
        scale: [f32; 4],
    }

    /// One joint's or node's motion, bind to model space: a 3×4 row-major
    /// affine (rows of basis x, basis y, basis z and origin).
    Storage Joint {
        x: [f32; 4],
        y: [f32; 4],
        z: [f32; 4],
    }

    /// One effect instance: four vec4 whose meaning depends on `misc.x`
    /// (0 streak, 1 glow, 2 flipbook, 3 rotor), in draw order.
    Vertex Effect {
        a: [f32; 4],
        b: [f32; 4],
        colour: [f32; 4],
        misc: [f32; 4],
    }

    /// One chosen cast light (at most 24 a frame).
    Storage CastLight {
        /// xyz (m, sim), and 1/radius² (m⁻²).
        position: [f32; 4],
        /// rgb irradiance at its centre; w unused.
        color: [f32; 4],
    }

    /// A box of what this frame draws, for the UI's hit tests.
    Plain Pick {
        unit: u32,
        /// A soldier's id, or `NONE` for the whole unit.
        member: u32,
        /// 1 own, 2 identified enemy, 4 aloft.
        flags: u32,
        pad: u32,
        /// Centre (m, sim), and yaw (rad).
        centre: [f32; 4],
        /// Half extents (m); w unused.
        half: [f32; 4],
    }

    /// A drawn unit as audio hears it.
    Plain Emitter {
        unit: u32,
        member: u32,
        /// The unit type's index in the world's `units.types` (by name), or
        /// `NONE` for a soldier.
        class: u32,
        /// 1 enemy, 2 reversing.
        flags: u32,
        /// Position (m, sim); w unused.
        position: [f32; 4],
        /// Left and right track travel (m), turret yaw (rad), unused.
        motion: [f32; 4],
    }

    /// What the frame's camera sees (sim space).
    Plain View {
        /// Sim → view, column-major.
        view: [f32; 16],
        /// Reverse-Z with an infinite far plane, column-major.
        projection: [f32; 16],
        /// The eye (m); w unused.
        eye: [f32; 4],
        /// Width and height (px), 1/height, pixels per metre at 1 m.
        size: [f32; 4],
        /// The unnormalised ray through the top-left corner, and its change
        /// across the width and down the height (the sky's `Rays`: w unused).
        sky_origin: [f32; 4],
        sky_dx: [f32; 4],
        sky_dy: [f32; 4],
    }

    /// The sky's light for the battle (`SkyLight` in the sky's shaders):
    /// what the sky lookup bakes and the shown sky and its clouds read.
    Plain SkyLight {
        /// Unit vector toward the sun.
        sun_direction: [f32; 3],
        /// 0 clear to 1 overcast, from turbidity.
        overcast: f32,
        /// Max-channel-normalised transmittance toward the sun.
        sun_transmittance: [f32; 3],
        /// `sky.radiance`.
        radiance: f32,
        /// The lower hemisphere's ground bounce, greyed by overcast.
        ground_tint: [f32; 3],
        mie_scatter: f32,
        mie_extinction: f32,
        /// The multiple-scattering floor's share for this sun.
        multiple_scatter: f32,
        /// The low sun's aureole strength.
        aureole: f32,
        /// The overcast dome's brightness for this sun.
        overcast_scale: f32,
        /// The sun disc's radiance through the overcast.
        disc_radiance: f32,
        /// The cloud layer's height over its mass size.
        cloud_reach: f32,
        /// Cloud density's smoothstep edges about the coverage threshold.
        cloud_edge: [f32; 2],
        /// A step across the layer toward the sun.
        cloud_to_sun: [f32; 2],
        /// The coverage threshold, and where the sunward shadow and the
        /// thick core saturate.
        cloud_low: f32,
        cloud_shadow_top: f32,
        cloud_core_top: f32,
        cloud_opacity: f32,
        pad0: [f32; 2],
        /// A sunlit top's radiance.
        cloud_lit: [f32; 3],
        /// The silver lining's radiance.
        cloud_silver: f32,
        /// A shaded base's radiance, before the sky shows through it.
        cloud_base: [f32; 3],
        pad1: f32,
    }

    /// The haze (`Aerial` in every lit shader's environment uniform):
    /// extinction over distance from the camera's ground focus, a range
    /// term, a sunward Mie tint and a low valley mist.
    Plain Aerial {
        /// Per-channel extinction σ (km⁻¹).
        extinction: [f32; 3],
        /// No haze within this distance of the focus (km).
        clear_radius_km: f32,
        /// The sunward Mie tint (linear rgb) and its strength.
        sun_tint: [f32; 3],
        sun_strength: f32,
        /// The valley mist's colour (linear rgb) and how much of it the far air takes.
        mist_color: [f32; 3],
        mist_color_strength: f32,
        overview_from_m: f32,
        range_near_m: f32,
        range_far_m: f32,
        range_power: f32,
        range_strength: f32,
        mist_opacity: f32,
        sun_power: f32,
        pad0: f32,
        /// The mist's height band and distance ramp (m).
        mist_height_m: [f32; 2],
        mist_distance_m: [f32; 2],
        /// Where the horizon's sky takes over from the view's, below and above it.
        horizon_fade: [f32; 2],
        pad1: [f32; 2],
    }

    /// The sun and the shadow fill, as the environment uniform carries them
    /// after its view (`sunDirection` to `fill`).
    Plain Sun {
        /// Unit vector toward the sun; w 0.
        sun_direction: [f32; 4],
        /// The sun's colour (its transmittance, greyed by overcast) times its
        /// intensity, linear rgb; w 0.
        sun_radiance: [f32; 4],
        /// x the renderer's environment map's top mip (0 here); y the share
        /// of the sun a sun-shadowed surface keeps.
        settings: [f32; 4],
        /// Linear rgb multiplier on the sky's environment light; w 0.
        fill: [f32; 4],
    }

    /// The bloom (`Bloom` in post's highpass and composite).
    Plain Bloom {
        /// The first four blurred levels' shares of the composite.
        weights: [f32; 4],
        /// The fifth level's share.
        weight4: f32,
        strength: f32,
        /// Where the highpass starts and where it is full (linear HDR luminance).
        threshold: f32,
        knee: f32,
    }

    /// The grade and exposure (`Grade` in post's final pass).
    Plain Grade {
        saturation: f32,
        contrast: f32,
        split_tone: f32,
        shadow_lift: f32,
        exposure: f32,
        pad: [f32; 3],
        /// rgb; w 1.
        shadow_tint: [f32; 4],
        /// rgb; w 1.
        highlight_tint: [f32; 4],
        /// rgb; w 0.
        lift: [f32; 4],
    }

    /// What unseen looks like (`FogStyleUniform` in the fog mask passes):
    /// a `presentation.fog` style resolved (`fog::FogStyle::look`).
    Plain FogLook {
        /// The night tint at unit luminance.
        tint: [f32; 3],
        dim: f32,
        cool: f32,
        saturation: f32,
        veil: f32,
        line_strength: f32,
        line_spacing_px: f32,
        line_width_px: f32,
        /// Radians.
        line_angle: f32,
        line_floor: f32,
        edge_softness_px: f32,
        pad0: [f32; 3],
        /// Display sRGB.
        rim_color: [f32; 3],
        rim_alpha: f32,
        rim_width_px: f32,
        /// How far the distance passes search for the other side, pixels.
        reach_px: f32,
        pad1: [f32; 2],
    }

    /// A camera as every world, shadow and overlay shader binds it
    /// (`Camera`): the frame's eye, or a sun cascade posed as its light.
    Plain Camera {
        view_proj: [f32; 16],
        inv_view_proj: [f32; 16],
        eye: [f32; 3],
        znear: f32,
        /// The viewport (px).
        width: f32,
        height: f32,
        /// The animation clock (s).
        time: f32,
        pad: f32,
    }

    /// Every shadow receiver's sun cascades (`SunShadow`), from the frame's
    /// fit (`shadow::sun_shadow`).
    Plain SunShadow {
        /// Per cascade (`shadow::CASCADES`), 24 floats: its light's
        /// view-projection (16); depth bias, normal bias (m), PCF radius
        /// (texels), 0; the receiver-depth interval it shades, 0, 0. An
        /// unfitted cascade is the identity over [1, 1].
        cascades: [f32; 96],
        /// The receiver range's capped far, 0, its near, 0.
        control: [f32; 4],
    }

    /// Fog of war's sight for one frame (`FogParams`, bound by the fog
    /// compute and every fogged material): the frame's camera and size, the
    /// sight lights' geometry, the static world sight is cut by, and the
    /// structures that take fog whole (`fog::sight`).
    Plain FogParams {
        inv_view_proj: [f32; 16],
        camera_eye: [f32; 3],
        /// The tallest canopy over the ground (m).
        translucent_lift_m: f32,
        azimuth_bins: u32,
        terrain_azimuth_bins: u32,
        radial_bins: u32,
        tile_px: u32,
        tiles_x: u32,
        tile_eyes_max: u32,
        eye_count: u32,
        /// The foliage grid's cells across and down (0: no foliage).
        foliage_nx: u32,
        foliage_ny: u32,
        height_nx: u32,
        height_ny: u32,
        /// Eyes whose maps this frame rebuilds.
        rebuild_count: u32,
        /// 0: no fog, everything seen.
        enabled: u32,
        probe_count: u32,
        first_bin_m: f32,
        target_height_m: f32,
        foliage_cell_m: f32,
        foliage_full_block: f32,
        min_sight_gap_m: f32,
        pad0: f32,
        /// The playable rectangle as the map's extents give it; outside it
        /// is fogged.
        playable: [f32; 4],
        height_spacing: f32,
        face_probe_m: f32,
        width: f32,
        height: f32,
        step_min_m: f32,
        step_max_m: f32,
        step_fraction: f32,
        roof_reach_m: f32,
        /// The structures that take fog whole: how many, their lookup grid
        /// in the `wholes` texture (cells across and down, cell size,
        /// origin) and where its item lists, boxes and seen flags start.
        whole_count: u32,
        whole_nx: u32,
        whole_ny: u32,
        whole_cell_m: f32,
        whole_origin: [f32; 2],
        whole_items_base: u32,
        whole_boxes_base: u32,
        whole_flags_base: u32,
        pad1: [f32; 3],
    }

    /// One eye as its map was built (`FogEyeRecord`): `position` the map's
    /// origin and `reach` its radius; forward, shape and range the latest
    /// publication's.
    Plain FogEye {
        position: [f32; 3],
        reach: f32,
        forward: f32,
        front: f32,
        side: f32,
        rear: f32,
        range: f32,
        /// The map slot holding this eye's maps.
        slot: u32,
        pad0: u32,
        pad1: u32,
    }

    /// An oriented box that hides what lies behind it (`FogBox`).
    Plain FogBox {
        center: [f32; 2],
        /// Half extents along and across its heading.
        half: [f32; 2],
        yaw: f32,
        top: f32,
        base: f32,
        pad1: f32,
    }

    /// What this frame poses and runs, for the gates.
    Plain Counts {
        soldiers: u32,
        vehicles: u32,
        aircraft: u32,
        corpses: u32,
        fading: u32,
        /// Effects running by kind: tracer, flash, impact, sparks, blast,
        /// puff, flame, trail.
        running: [u32; 8],
        lights: u32,
        /// Effect instances past the capacity.
        dropped: u32,
        /// Soldiers off screen this frame.
        culled: u32,
    }
}

/// `records.gen.ts`: every layout as TypeGPU schemas or word offsets, and the
/// views a renderer reads a frame and a world through.
pub fn typescript() -> String {
    let mut out = String::from(
        "// Generated by `crates/presentation/src/records.rs` (`records_are_current`); do not edit.\n\
         // The records the presentation crate hands a renderer (18b's World and Frame).\n\
         import { d } from \"typegpu\";\n\n\
         export const NONE = 0xffffffff;\n",
    );
    out.push_str(&format!(
        "/** The widest fog rim or soft edge a style may ask for, device pixels (`fog.rs`). */\n\
         export const FOG_EDGE_REACH_PX = {};\n",
        crate::fog::FOG_EDGE_REACH_PX
    ));
    {
        use crate::fog::sight::{FOLIAGE_STEP, WHOLE_BOX_WORDS, WHOLE_TEXTURE_WIDTH};
        out.push_str(&format!(
            "/** Fog's foliage depth steps, the `wholes` texture's row and a structure box's words (`fog/sight.rs`). */\n\
             export const FOLIAGE_STEP = {FOLIAGE_STEP};\n\
             export const WHOLE_TEXTURE_WIDTH = {WHOLE_TEXTURE_WIDTH};\n\
             export const WHOLE_BOX_WORDS = {WHOLE_BOX_WORDS};\n",
        ));
    }
    out.push_str(&format!(
        "/** The sun's cascade slots `SunShadow` holds (`shadow.rs`). */\n\
         export const SUN_CASCADES = {};\n",
        crate::shadow::CASCADES
    ));
    {
        use crate::scars::*;
        out.push_str(&format!(
            "/** The scar cache's directory flags over a tile key + 1, its pool texture\n \
             *  and the normalization table at the pool's start, and `ScarParams`' words\n \
             *  before the biome's look (`scars/mod.rs`). */\n\
             export const SCAR_UNIFORM = {UNIFORM:#x};\n\
             export const SCAR_DENSE = {DENSE:#x};\n\
             export const SCAR_KEY_MASK = {KEY_MASK:#x};\n\
             export const SCAR_POOL_WIDTH = {POOL_WIDTH};\n\
             export const SCAR_POOL_HEIGHT = {POOL_HEIGHT};\n\
             export const SCAR_NORMALIZATION_WORDS = {NORMALIZATION_WORDS};\n\
             export const SCAR_PARAMS_WORDS = {PARAMS_WORDS};\n"
        ));
    }
    for l in LAYOUTS {
        out.push_str(&format!("\n/** {} */\n", l.doc.trim()));
        let word = |s: &str| if s == "f32" { "f32" } else { "u32" };
        match l.binding {
            Binding::Vertex | Binding::Storage => {
                let (kind, vec4) = if l.binding == Binding::Vertex {
                    ("unstruct", "float32x4")
                } else {
                    ("struct", "vec4f")
                };
                out.push_str(&format!("export const {} = d.{kind}({{\n", l.name));
                for f in l.fields {
                    assert!(f.count == 4 && f.scalar == "f32", "{}.{}", l.name, f.name);
                    out.push_str(&format!(
                        "  /** {} */\n  {}: d.{vec4},\n",
                        f.doc.trim(),
                        f.name
                    ));
                }
                out.push_str("});\n");
            }
            Binding::Plain => {
                out.push_str(&format!("export const {} = {{\n", l.name));
                for f in l.fields {
                    out.push_str(&format!(
                        "  /** {} ({}{}) */\n  {}: {},\n",
                        f.doc.trim(),
                        word(f.scalar),
                        if f.count > 1 {
                            format!(" × {}", f.count)
                        } else {
                            String::new()
                        },
                        f.name,
                        f.offset / 4
                    ));
                }
                out.push_str("} as const;\n");
            }
        }
        out.push_str(&format!(
            "export const {}_WORDS = {};\n",
            snake(l.name),
            l.size / 4
        ));
    }
    let views = |name: &str, header: &Layout| {
        let sets: Vec<&FieldLayout> = header
            .fields
            .iter()
            .filter(|f| f.scalar == "u32" && f.count == 2)
            .collect();
        let mut s = format!(
            "\n/** Views over module memory of the {name} whose header is at `ptr`;\n *  valid until the next call into the module. */\nexport function {}Views(memory: ArrayBuffer, ptr: number) {{\n  const header = new Uint32Array(memory, ptr, {});\n  const set = (word: number, words: number) => {{\n    const count = header[word + 1];\n    return {{\n      count,\n      f32: new Float32Array(memory, header[word], count * words),\n      u32: new Uint32Array(memory, header[word], count * words),\n    }};\n  }};\n  return {{\n    header,\n",
            name.to_lowercase(),
            header.size / 4
        );
        for f in sets {
            let word = f.offset / 4;
            s.push_str(&if f.name == "strings" {
                format!(
                    "    strings: new Uint8Array(memory, header[{word}], header[{}]),\n",
                    word + 1
                )
            } else {
                format!(
                    "    {}: set({word}, {}_WORDS),\n",
                    f.name,
                    snake(record_of(f.name))
                )
            });
        }
        s.push_str("  };\n}\n");
        s
    };
    out.push_str(&views("Frame", &FrameHeader::LAYOUT));
    out.push_str(&views("World", &WorldHeader::LAYOUT));
    out
}

/// The record a header's set holds.
fn record_of(set: &str) -> &'static str {
    match set {
        "batches" => "Batch",
        "instances" => "Instance",
        "palettes" => "Joint",
        "effects" => "Effect",
        "lights" => "CastLight",
        "picks" => "Pick",
        "emitters" => "Emitter",
        "counts" => "Counts",
        "appearances" => "Appearance",
        other => panic!("no record for set {other}"),
    }
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_current() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/battle-renderer/src/records.gen.ts"
        );
        let generated = typescript();
        if std::env::var_os("UPDATE_RECORDS").is_some() {
            std::fs::write(path, &generated).unwrap();
        }
        let current = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            current == generated,
            "records.gen.ts is stale: rerun with UPDATE_RECORDS=1"
        );
    }
}

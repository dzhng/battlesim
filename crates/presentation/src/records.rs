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
            #[derive(Clone, Copy, Debug, Default, PartialEq)]
            pub struct $name { $($(#[doc = $fdoc])* pub $field: $ty),* }
            // SAFETY: `repr(C)`, every field a 4-byte scalar or an array of
            // them, and the size a multiple of 16 (asserted): no padding.
            unsafe impl bytemuck::Zeroable for $name {}
            unsafe impl bytemuck::Pod for $name {}
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
        pad: [u32; 2],
    }

    /// What one frame draws of one drawable at one tier.
    Plain Batch {
        /// Stable across frames; the renderer keys its buffers by it.
        key: u32,
        /// `World.appearances` row.
        appearance: u32,
        /// Detail tier, 0 finest; 4 is the impostor card.
        tier: u32,
        /// 0 drawn in view; 2 a translucent ghost (no shadow, no depth write).
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
        /// The unit type's row in the world's kinds, or `NONE` for a soldier.
        class: u32,
        /// 1 enemy, 2 reversing.
        flags: u32,
        /// Position (m, sim); w unused.
        position: [f32; 4],
        /// Left and right track travel (m), turret yaw (rad), unused.
        motion: [f32; 4],
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
        /// Models off screen this frame.
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

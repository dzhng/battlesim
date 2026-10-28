//! Record layout metadata and the schema-generating declaration macro.

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

/// How a renderer binds a record: a vertex instance attribute, a WGSL
/// struct (a uniform, or a storage buffer's element), or plain words it reads
/// on the CPU.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Vertex,
    Struct,
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
            impl<const N: usize> Field for [$name; N] {
                const SCALAR: &'static str = stringify!($name);
                const COUNT: usize = N;
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

pub(super) use records;

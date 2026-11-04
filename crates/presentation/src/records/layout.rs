//! Record layout metadata and the schema-generating declaration macro.

/// What a field holds, for the generated schemas.
pub trait Field {
    /// `"f32"`, `"u32"`, `"i32"`, or a record's name.
    const SCALAR: &'static str;
    const COUNT: usize;
    /// The element's layout for a record or an array of records.
    const RECORD: Option<&'static Layout> = None;
}
impl Field for f32 {
    const SCALAR: &'static str = "f32";
    const COUNT: usize = 1;
}
impl Field for u32 {
    const SCALAR: &'static str = "u32";
    const COUNT: usize = 1;
}
impl Field for i32 {
    const SCALAR: &'static str = "i32";
    const COUNT: usize = 1;
}
// Keep the array impl here: an exported macro's callers cannot implement a
// foreign trait for arrays, even arrays of their own records.
impl<T: Field, const N: usize> Field for [T; N] {
    const SCALAR: &'static str = T::SCALAR;
    const COUNT: usize = N;
    const RECORD: Option<&'static Layout> = T::RECORD;
}

pub struct FieldLayout {
    pub name: &'static str,
    pub offset: usize,
    pub scalar: &'static str,
    pub count: usize,
    pub record: Option<&'static Layout>,
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

/// Declare packed records and their reflection metadata, including nested records.
#[macro_export]
macro_rules! records {
    ($($(#[doc = $doc:literal])* $binding:ident $name:ident {
        $($(#[doc = $fdoc:literal])* $field:ident: $ty:ty),* $(,)?
    })*) => {
        $(
            $(#[doc = $doc])*
            #[repr(C)]
            #[derive(Clone, Copy, Debug, PartialEq)]
            pub struct $name { $($(#[doc = $fdoc])* pub $field: $ty),* }
            // SAFETY: `repr(C)`, every field a 4-byte scalar, packed record,
            // or array of them, and no padding (asserted below).
            unsafe impl $crate::records::bytemuck::Zeroable for $name {}
            unsafe impl $crate::records::bytemuck::Pod for $name {}
            impl ::core::default::Default for $name {
                fn default() -> Self {
                    $crate::records::bytemuck::Zeroable::zeroed()
                }
            }
            impl $crate::records::Field for $name {
                const SCALAR: &'static str = stringify!($name);
                const COUNT: usize = 1;
                const RECORD: ::core::option::Option<&'static $crate::records::Layout> =
                    ::core::option::Option::Some(&Self::LAYOUT);
            }
            const _: () = assert!(::core::mem::size_of::<$name>() % 16 == 0);
            const _: () = assert!(
                ::core::mem::size_of::<$name>() == 0 $(+ ::core::mem::size_of::<$ty>())*
            );
            impl $name {
                pub const LAYOUT: $crate::records::Layout = $crate::records::Layout {
                    name: stringify!($name),
                    doc: concat!($($doc),*),
                    binding: $crate::records::Binding::$binding,
                    size: ::core::mem::size_of::<$name>(),
                    fields: &[$($crate::records::FieldLayout {
                        name: stringify!($field),
                        offset: ::core::mem::offset_of!($name, $field),
                        scalar: <$ty as $crate::records::Field>::SCALAR,
                        count: <$ty as $crate::records::Field>::COUNT,
                        record: <$ty as $crate::records::Field>::RECORD,
                        doc: concat!($($fdoc),*),
                    }),*],
                };
            }
        )*
        /// Every record's layout, in declaration order.
        pub const LAYOUTS: &[$crate::records::Layout] = &[$($name::LAYOUT),*];
    };
}

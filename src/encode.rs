pub(crate) mod color;
pub(crate) mod shape;
pub(crate) mod size;
pub(crate) mod text;
pub(crate) mod theta;
pub(crate) mod x;
pub(crate) mod y;
pub(crate) mod y2;

use self::{
    x::X,
    y::Y,
    y2::Y2,
    theta::Theta,
    color::Color,
    shape::Shape,
    size::Size,
    text::Text,
};

/// Unified application interface for encoding specifications
///
/// This trait provides a common interface for applying different types of encodings
/// to a chart's encoding configuration. Each encoding type (X, Y, Color, etc.)
/// implements this trait to define how it should be added to the global encoding.
///
/// # Type Parameters
/// * `Self` - The encoding type that implements this trait
///
/// # Methods
/// * `apply` - Applies the encoding to the provided `Encoding` container
pub trait IntoEncoding {
    /// Applies the encoding to the provided `Encoding` container
    ///
    /// This method takes ownership of the encoding specification and applies it
    /// to the global encoding configuration. Each specific encoding type (X, Y, Color, etc.)
    /// implements this method to define how it should be stored in the encoding container.
    ///
    /// # Arguments
    /// * `enc` - A mutable reference to the `Encoding` container to which this encoding should be applied
    fn apply(self, enc: &mut Encoding);
}

/// Global encoding container
///
/// The `Encoding` struct serves as a central repository for all visual encoding
/// specifications in a chart. It holds optional references to various encoding
/// types that define how data fields map to visual properties like position,
/// color, size, and shape.
///
/// This struct is typically populated using the `encode` method on chart objects
/// and is used internally during the rendering process to determine how data
/// should be visually represented.
#[derive(Default)]
pub struct Encoding {
    pub(crate) x: Option<X>,             // For both continuous and discrete data
    pub(crate) y: Option<Y>,             // For both continuous and discrete data
    pub(crate) y2: Option<Y2>,           // Useful for mark rule
    pub(crate) theta: Option<Theta>,     // For both continuous and discrete data
    pub(crate) color: Option<Color>,     // For both continuous and discrete data
    pub(crate) shape: Option<Shape>,     // For discrete data
    pub(crate) size: Option<Size>,       // For continuous data
    pub(crate) text: Option<Text>,                // For text marks
}

impl Encoding {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Get all field names that are currently set in the encoding
    pub(crate) fn active_fields(&self) -> Vec<&str> {
        let mut fields = Vec::new();
        if let Some(ref x) = self.x {
            fields.push(x.field.as_str());
        }
        if let Some(ref y) = self.y {
            fields.push(y.field.as_str());
        }
        if let Some(ref y2) = self.y2 {
            fields.push(y2.field.as_str());
        }
        if let Some(ref theta) = self.theta {
            fields.push(theta.field.as_str());
        }
        if let Some(ref color) = self.color {
            fields.push(color.field.as_str());
        }
        if let Some(ref shape) = self.shape {
            fields.push(shape.field.as_str());
        }
        if let Some(ref size) = self.size {
            fields.push(size.field.as_str());
        }
        if let Some(ref text) = self.text {
            fields.push(text.field.as_str());
        }

        fields
    }
}

/* ---------- Single channel implementation ---------- */

impl IntoEncoding for X {
    fn apply(self, enc: &mut Encoding) {
        enc.x = Some(self);
    }
}

impl IntoEncoding for Y {
    fn apply(self, enc: &mut Encoding) {
        enc.y = Some(self);
    }
}

impl IntoEncoding for Y2 {
    fn apply(self, enc: &mut Encoding) {
        enc.y2 = Some(self);
    }
}

impl IntoEncoding for Theta {
    fn apply(self, enc: &mut Encoding) {
        enc.theta = Some(self);
    }
}

impl IntoEncoding for Color {
    fn apply(self, enc: &mut Encoding) {
        enc.color = Some(self);
    }
}

impl IntoEncoding for Shape {
    fn apply(self, enc: &mut Encoding) {
        enc.shape = Some(self);
    }
}

impl IntoEncoding for Size {
    fn apply(self, enc: &mut Encoding) {
        enc.size = Some(self);
    }
}

impl IntoEncoding for Text {
    fn apply(self, enc: &mut Encoding) {
        enc.text = Some(self);
    }
}

/// Macro to implement IntoEncoding trait for tuples of different sizes
///
/// This macro generates implementations of the IntoEncoding trait for tuples
/// containing 1 to 9 elements. Each element in the tuple must implement IntoEncoding.
/// This allows users to pass multiple encodings as a single tuple to the encode method.
///
/// # Parameters
/// * `$($idx:tt $T:ident),+` - A repetition of index-type pairs where:
///   - `$idx` is the tuple index (0, 1, 2, etc.)
///   - `$T` is the type identifier for that position
///
/// # Generated Implementation
/// For each tuple size, this creates an IntoEncoding implementation that:
/// 1. Takes ownership of the tuple
/// 2. Calls apply() on each element in the tuple, passing the same Encoding reference
/// 3. Applies all encodings to the same Encoding container in sequence
macro_rules! impl_tuple_encoding {
    ($($idx:tt $T:ident),+) => {
        impl<$($T: IntoEncoding),+> IntoEncoding for ($($T,)+) {
            #[inline]
            fn apply(self, enc: &mut Encoding) {
                $(
                    self.$idx.apply(enc);
                )+
            }
        }
    };
}

// Update the tuple implementation to support up to 9 elements now
impl_tuple_encoding!(0 T0); // 1
impl_tuple_encoding!(0 T0, 1 T1); // 2
impl_tuple_encoding!(0 T0, 1 T1, 2 T2); // 3
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3); // 4
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4); // 5
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5); // 6
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6); // 7
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7); // 8

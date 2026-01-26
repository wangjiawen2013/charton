pub mod color;
pub mod shape;
pub mod size;
pub mod text;
pub mod theta;
pub mod x;
pub mod y;
pub mod y2;

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
use crate::scale::{Scale, ScaleDomain, Expansion};

/// Represents the various visual aesthetics that can be mapped to data.
/// 
/// By using an enum, we can write generic logic in the rendering engine 
/// to process all channels in a loop rather than writing custom code for 
/// each axis or legend.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Channel {
    X,
    Y,
    Color,
    Shape,
    Size,
}

/// Unified application interface for encoding specifications.
///
/// This trait allows different encoding types (like X, Color, or Size) to be 
/// added to the global `Encoding` container.
pub trait IntoEncoding {
    /// Consumes the specification and applies it to the provided `Encoding` container.
    fn apply(self, enc: &mut Encoding);
}

/// Global encoding container.
///
/// The `Encoding` struct serves as a central repository for all visual encoding
/// specifications in a chart. It holds the "Intent" (user configuration) for 
/// how data fields map to visual properties.
///
/// By using the `Channel` enum, this container can be accessed dynamically 
/// by the rendering engine during the "Resolution" phase.
#[derive(Default)]
pub struct Encoding {
    pub(crate) x: Option<X>,
    pub(crate) y: Option<Y>,
    pub(crate) y2: Option<Y2>,
    pub(crate) theta: Option<Theta>,
    pub(crate) color: Option<Color>,
    pub(crate) shape: Option<Shape>,
    pub(crate) size: Option<Size>,
    pub(crate) text: Option<Text>,
}

impl Encoding {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Returns the data field name associated with a specific visual channel.
    ///
    /// This is used by the `LayeredChart` to discover which columns in the 
    /// dataframe need to be processed for scale training.
    pub fn get_field_by_channel(&self, channel: Channel) -> Option<&str> {
        match channel {
            Channel::X => self.x.as_ref().map(|v| v.field.as_str()),
            Channel::Y => self.y.as_ref().map(|v| v.field.as_str()),
            Channel::Color => self.color.as_ref().map(|v| v.field.as_str()),
            Channel::Shape => self.shape.as_ref().map(|v| v.field.as_str()),
            Channel::Size => self.size.as_ref().map(|v| v.field.as_str()),
        }
    }

    /// Retrieves the user-defined scale type (e.g., Linear, Log, Time) for a channel.
    pub fn get_scale_by_channel(&self, channel: Channel) -> Option<Scale> {
        match channel {
            Channel::X => self.x.as_ref().and_then(|v| v.scale_type.clone()),
            Channel::Y => self.y.as_ref().and_then(|v| v.scale_type.clone()),
            Channel::Color => self.color.as_ref().and_then(|v| v.scale_type.clone()),
            Channel::Shape => self.shape.as_ref().and_then(|v| v.scale_type.clone()),
            Channel::Size => self.size.as_ref().and_then(|v| v.scale_type.clone()),
        }
    }

    /// Retrieves the user-defined domain override for a specific channel.
    pub fn get_domain_by_channel(&self, channel: Channel) -> Option<ScaleDomain> {
        match channel {
            Channel::X => self.x.as_ref().and_then(|v| v.domain.clone()),
            Channel::Y => self.y.as_ref().and_then(|v| v.domain.clone()),
            Channel::Color => self.color.as_ref().and_then(|v| v.domain.clone()),
            Channel::Shape => self.shape.as_ref().and_then(|v| v.domain.clone()),
            Channel::Size => self.size.as_ref().and_then(|v| v.domain.clone()),
        }
    }

    /// Retrieves the expansion (padding) preferences for a channel.
    pub fn get_expand_by_channel(&self, channel: Channel) -> Option<Expansion> {
        match channel {
            Channel::X => self.x.as_ref().and_then(|v| v.expand),
            Channel::Y => self.y.as_ref().and_then(|v| v.expand),
            Channel::Color => self.color.as_ref().and_then(|v| v.expand),
            Channel::Shape => self.shape.as_ref().and_then(|v| v.expand),
            Channel::Size => self.size.as_ref().and_then(|v| v.expand),
        }
    }

    /// Checks if the channel is explicitly configured to include zero in its axis range.
    pub fn get_zero_by_channel(&self, channel: Channel) -> bool {
        match channel {
            Channel::X => self.x.as_ref().and_then(|v| v.zero) == Some(true),
            Channel::Y => self.y.as_ref().and_then(|v| v.zero) == Some(true),
            _ => false,
        }
    }

    /// Returns a list of all data fields currently active in this encoding.
    ///
    /// Useful for debugging or for pruning unused columns from a dataset 
    /// before processing.
    pub(crate) fn active_fields(&self) -> Vec<&str> {
        let mut fields = Vec::new();
        // Check core channels
        let core_channels = [
            Channel::X, Channel::Y, Channel::Color, 
            Channel::Shape, Channel::Size
        ];
        
        for ch in core_channels {
            if let Some(field) = self.get_field_by_channel(ch) {
                fields.push(field);
            }
        }

        // Handle specialty channels not yet in the main Channel enum
        if let Some(ref y2) = self.y2 { fields.push(y2.field.as_str()); }
        if let Some(ref t) = self.theta { fields.push(t.field.as_str()); }
        if let Some(ref txt) = self.text { fields.push(txt.field.as_str()); }

        fields
    }
}

/* ---------- IntoEncoding Implementations ---------- */

impl IntoEncoding for X {
    fn apply(self, enc: &mut Encoding) { enc.x = Some(self); }
}

impl IntoEncoding for Y {
    fn apply(self, enc: &mut Encoding) { enc.y = Some(self); }
}

impl IntoEncoding for Y2 {
    fn apply(self, enc: &mut Encoding) { enc.y2 = Some(self); }
}

impl IntoEncoding for Theta {
    fn apply(self, enc: &mut Encoding) { enc.theta = Some(self); }
}

impl IntoEncoding for Color {
    fn apply(self, enc: &mut Encoding) { enc.color = Some(self); }
}

impl IntoEncoding for Shape {
    fn apply(self, enc: &mut Encoding) { enc.shape = Some(self); }
}

impl IntoEncoding for Size {
    fn apply(self, enc: &mut Encoding) { enc.size = Some(self); }
}

impl IntoEncoding for Text {
    fn apply(self, enc: &mut Encoding) { enc.text = Some(self); }
}

/// Macro to implement IntoEncoding for tuples (e.g., .encode((X::new("a"), Y::new("b"))))
macro_rules! impl_tuple_encoding {
    ($($idx:tt $T:ident),+) => {
        impl<$($T: IntoEncoding),+> IntoEncoding for ($($T,)+) {
            #[inline]
            fn apply(self, enc: &mut Encoding) {
                $( self.$idx.apply(enc); )+
            }
        }
    };
}

impl_tuple_encoding!(0 T0); 
impl_tuple_encoding!(0 T0, 1 T1); 
impl_tuple_encoding!(0 T0, 1 T1, 2 T2); 
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3); 
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4); 
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5); 
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6); 
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7); 
impl_tuple_encoding!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8);
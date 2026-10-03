//! Position adjustments — where marks sit along an axis.
//!
//! # The big picture
//!
//! A chart is built from four independent questions:
//!
//! 1. **What values does the data have?**  → *stats* (see `transform`)
//! 2. **Where does each mark go?**        → *position* (this module)
//! 3. **What shape is drawn?**            → *geometry* (see `mark` + `render`)
//! 4. **How does a value become a pixel?**→ *scales* + *coordinates*
//!
//! Keeping these four apart is what lets a handful of basic parts describe
//! almost any chart. A violin plot, for example, is:
//!
//! ```text
//! density estimate (stat)  +  dodge (position)  +  polygon (geometry)
//! ```
//!
//! # Why position is a data-space idea
//!
//! When several marks share one category (for example three treatments inside
//! the "Placebo" group), something has to decide that they sit *side by side*
//! instead of on top of each other. That decision is about **data**, not about
//! pixels: "move this treatment half a category to the right". Because it is a
//! data-space shift, every geometry can reuse it — points, bars, boxes,
//! error bars and violins all speak the same language.
//!
//! The alternative (and what older code did) is to compute the shift inside
//! each renderer using normalized coordinates. That works, but it forces every
//! renderer to re-implement the same arithmetic and couples it to pixel space.
//!
//! # Units
//!
//! Offsets are expressed in **category steps**: `1.0` is exactly one category
//! slot. That keeps the math independent of the axis, the panel size and the
//! zoom level.
//!
//! ## What is a "category step"?
//!
//! On a categorical axis every category owns one equal band. We measure that
//! band with a unit of our own: the **category step**. The first category sits
//! at position `0.0`, the second at `1.0`, the third at `2.0`, and so on. A
//! shift of `0.5` therefore means "move half a category to the right", no
//! matter how wide the category is on screen.
//!
//! The renderer later turns category steps into pixels with one shared factor,
//! `unit_step_norm = |scale.normalize(1.0) - scale.normalize(0.0)|`, which is
//! how many normalized units one whole step spans. Position code never needs to
//! know that factor, which is why the same layout serves every mark.
//!
//! ## What is a mark's "available width"?
//!
//! When `count` marks share one category they must sit side by side instead of
//! on top of each other. The category gives the group a total band of `span`
//! category steps (for example `0.8`). That band is divided into `count` equal
//! slots with `count - 1` gaps, and each gap is `spacing` times a slot:
//!
//! ```text
//!     span = count * slot + (count - 1) * (slot * spacing)
//!     =>   slot = span / (count + (count - 1) * spacing)
//! ```
//!
//! `item_width` returns the smaller of that slot and the mark's own `max_width`,
//! so a mark that asks to be narrow stays narrow. The i-th mark is then centred
//! at
//!
//! ```text
//!     offset(i) = (i - (count - 1) / 2) * item_width * (1 + spacing)
//! ```
//!
//! Worked example: 3 marks, `span = 0.9`, `spacing = 0.2`, no cap.
//! `slot = 0.9 / (3 + 2 * 0.2) = 0.2647`. The centres are `-0.3176`, `0.0`,
//! `+0.3176`, and the outermost mark's edge reaches `±0.45` — exactly the span.

/// Describes how items that share the same category are arranged.
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum Position {
    /// Every item is placed on the category centre, so they overlap.
    ///
    /// This is the right choice when there is only one item per category
    /// (a single violin, a plain bar) or when categories are split across
    /// facets instead of being placed side by side.
    #[default]
    Identity,

    /// Items are placed side by side inside the category slot.
    ///
    /// `spacing` is the gap between neighbours, measured as a fraction of one
    /// item's width (`0.0` = touching, `0.25` = a quarter-width gap).
    Dodge { spacing: f64 },
}

impl Position {
    /// Returns `true` when items are placed on top of each other.
    pub const fn is_identity(&self) -> bool {
        matches!(self, Self::Identity)
    }

    /// The default side-by-side layout.
    ///
    /// Uses the same 20 % gap between items that the box plot and point marks
    /// use, so a dodged violin group lines up with a dodged box or scatter.
    pub const fn dodge() -> Self {
        Self::Dodge { spacing: 0.2 }
    }

    /// Solves the layout of `count` items sharing one category.
    ///
    /// # Arguments
    /// * `count` — how many items share the category. A value of `0` or `1`
    ///   degenerates to a single centred item.
    /// * `width` — the total width available for the whole group, in category
    ///   steps. `1.0` would fill the category from edge to edge.
    ///
    /// # Returns
    /// A pair `(offsets, slot)`:
    /// * `offsets[i]` is the centre shift of item `i`, in category steps;
    /// * `slot` is the width one item may use, in category steps. Geometry
    ///   (for example the widest point of a violin) should fit inside `slot`.
    ///
    /// # Example
    /// Three items dodged with a 20 % gap over a width of `0.8`:
    /// ```
    /// use charton::position::Position;
    /// let (offsets, slot) = Position::Dodge { spacing: 0.2 }.resolve(3, 0.8);
    /// // The three centres are evenly spread around 0 and each item is
    /// // narrower than the whole band.
    /// assert_eq!(offsets.len(), 3);
    /// assert!(slot < 0.8 / 2.0);
    /// ```
    pub fn resolve(&self, count: usize, width: f64) -> (Vec<f64>, f64) {
        let n = count.max(1);
        match self {
            // A single lane on the centre line.
            Self::Identity | Self::Dodge { .. } if n == 1 => (vec![0.0], width.max(0.0)),

            Self::Identity => {
                // Several items, but they knowingly share the centre line.
                (vec![0.0; count], width.max(0.0))
            }

            Self::Dodge { spacing } => {
                let spacing = spacing.max(0.0);
                let n_f = n as f64;

                // Divide the band into `n` equal slots separated by `n - 1` gaps.
                //   width = n * slot + (n - 1) * gap      with gap = slot * spacing
                //   => slot = width / (n + (n - 1) * spacing)
                let slot = width / (n_f + (n_f - 1.0) * spacing);
                let stride = slot * (1.0 + spacing);

                // Centre the whole row on the category centre.
                let offsets = (0..count)
                    .map(|i| (i as f64 - (n_f - 1.0) / 2.0) * stride)
                    .collect();

                (offsets, slot)
            }
        }
    }

    /// The width one item may use inside a category, in category steps.
    ///
    /// This is the renderer-facing counterpart of [`Position::resolve`]. The
    /// group has `span` category steps to fill, divided into `count` slots with
    /// the configured gaps; `max_width` is the mark's own width cap (pass
    /// `f64::INFINITY` for no cap). It returns the smaller of the two, so a
    /// mark that asks to be narrow stays narrow.
    pub fn item_width(&self, count: f64, span: f64, max_width: f64) -> f64 {
        match self {
            Self::Identity => max_width,
            Self::Dodge { spacing } => {
                let n = count.max(1.0);
                let spacing = spacing.max(0.0);
                let slot = span / (n + (n - 1.0) * spacing);
                max_width.min(slot)
            }
        }
    }

    /// The centre shift of item `index` (0-based) out of `count` items, in
    /// category steps, given the resolved `item_width`.
    ///
    /// This is the single place the "side by side" arithmetic lives, so every
    /// renderer (bar, box, error bar, point layouts) produces identical lanes.
    pub fn offset(&self, index: f64, count: f64, item_width: f64) -> f64 {
        match self {
            Self::Identity => 0.0,
            Self::Dodge { spacing } => {
                let n = count.max(1.0);
                let spacing = spacing.max(0.0);
                (index - (n - 1.0) / 2.0) * item_width * (1.0 + spacing)
            }
        }
    }
}


impl From<&str> for Position {
    /// Lets users write `"identity"` or `"dodge"` where a [`Position`] is expected.
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "dodge" => Self::Dodge { spacing: 0.2 },
            _ => Self::Identity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn identity_keeps_every_item_centred() {
        let (offsets, slot) = Position::Identity.resolve(3, 0.8);
        assert_eq!(offsets, vec![0.0, 0.0, 0.0]);
        assert_eq!(slot, 0.8);
    }

    #[test]
    fn dodge_is_symmetric_and_fits_the_band() {
        let (offsets, slot) = Position::Dodge { spacing: 0.2 }.resolve(3, 0.8);

        // Symmetric around zero.
        assert!((offsets[0] + offsets[2]).abs() < 1e-12);
        assert!(offsets[1].abs() < 1e-12);

        // Every item fits inside the band: outer edge of the first item.
        let left_edge = offsets[0] - slot / 2.0;
        let right_edge = offsets[2] + slot / 2.0;
        assert!(left_edge >= -0.4 - 1e-12);
        assert!(right_edge <= 0.4 + 1e-12);
    }

    #[test]
    fn single_item_is_centred_even_when_dodging() {
        let (offsets, slot) = Position::Dodge { spacing: 0.5 }.resolve(1, 0.6);
        assert_eq!(offsets, vec![0.0]);
        assert_eq!(slot, 0.6);
    }

    /// The renderer helpers must reproduce the same lanes as `resolve` when the
    /// mark width is not capped. This is the guarantee that wiring the
    /// renderers through `Position` cannot move a single mark.
    #[test]
    fn renderer_helpers_match_resolve() {
        let position = Position::Dodge { spacing: 0.2 };
        let (offsets, slot) = position.resolve(3, 0.8);
        let item = position.item_width(3.0, 0.8, f64::INFINITY);
        assert!((item - slot).abs() < 1e-12);
        for (i, expected) in offsets.iter().enumerate() {
            let got = position.offset(i as f64, 3.0, item);
            assert!(
                (got - expected).abs() < 1e-12,
                "lane {i}: {got} != {expected}"
            );
        }
    }

    #[test]
    fn item_width_respects_the_mark_cap() {
        let position = Position::Dodge { spacing: 0.0 };
        // Three items in a span of 0.9 would give 0.3 each, but the mark caps
        // itself at 0.2.
        assert!((position.item_width(3.0, 0.9, 0.2) - 0.2).abs() < 1e-12);
    }
}

use crate::coordinate::{CoordinateTrait, Rect};
use crate::core::aesthetics::GlobalAesthetics;
use crate::theme::Theme;
use std::sync::Arc;

/// `ChartSpec` (Chart Specification) represents the global blueprint of a chart.
///
/// It encapsulates the "static" visual rules that apply to the entire visualization, 
/// regardless of how many individual panels (facets) are rendered. This includes 
/// aesthetic scales (color, shape, etc.) and the visual theme.
pub struct ChartSpec<'a> {
    /// Global aesthetic mappings resolved from data (e.g., color palettes, shape sets).
    pub aesthetics: &'a GlobalAesthetics,

    /// The visual theme defining the look and feel (fonts, grid lines, margins).
    pub theme: &'a Theme,
}

/// `PanelContext` provides the localized rendering environment for a specific 2D area.
///
/// This struct acts as a "toolbox" for `MarkRenderer`, providing access to both 
/// the global `spec` and the local `coord` system.
pub struct PanelContext<'a> {
    /// Reference to the global chart specification.
    /// Named `spec` to avoid confusion with the top-level `Chart` struct.
    pub spec: &'a ChartSpec<'a>,

    /// The coordinate system for this specific panel. 
    pub coord: Arc<dyn CoordinateTrait>,

    /// The physical rectangular area (in pixels) on the canvas for this panel.
    pub panel: Rect,
}

impl<'a> PanelContext<'a> {
    pub fn new(
        spec: &'a ChartSpec<'a>,
        coord: Arc<dyn CoordinateTrait>,
        panel: Rect,
    ) -> Self {
        Self { spec, coord, panel }
    }

    /// Maps normalized data values ([0.0, 1.0]) to absolute screen pixels.
    ///
    /// # Performance Note: #[inline]
    /// We use `#[inline]` here because this method is called inside tight loops 
    /// (e.g., rendering 10,000+ scatter points). Inlining allows the compiler 
    /// to eliminate the function call overhead by embedding the transformation 
    /// logic directly into the caller's loop, significantly boosting performance.
    #[inline]
    pub fn transform(&self, x_norm: f32, y_norm: f32) -> (f32, f32) {
        self.coord.transform(x_norm, y_norm, &self.panel)
    }

    /// Convenience helper for X-axis transformation.
    /// Also inlined to maintain high-throughput rendering.
    #[inline]
    pub fn x_to_px(&self, x_norm: f32) -> f32 {
        self.transform(x_norm, 0.0).0
    }

    /// Convenience helper for Y-axis transformation.
    /// Also inlined to maintain high-throughput rendering.
    #[inline]
    pub fn y_to_px(&self, y_norm: f32) -> f32 {
        self.transform(0.0, y_norm).1
    }

    /// Provides direct access to the global theme.
    pub fn theme(&self) -> &Theme {
        self.spec.theme
    }

    /// Provides direct access to the global aesthetic scales.
    pub fn aesthetics(&self) -> &GlobalAesthetics {
        self.spec.aesthetics
    }
}
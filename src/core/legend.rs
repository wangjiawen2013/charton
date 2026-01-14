/// LegendSpec represents the blueprint for a single legend block.
/// In Grammar of Graphics, a legend can represent multiple visual channels 
/// (e.g., color and shape) if they map to the same data field.
pub struct LegendSpec {
    /// The display title of the legend (usually the field name)
    pub title: String,
    /// The underlying data column name used for this mapping
    pub field: String,
    /// The scale type (e.g., Linear, Log, Discrete) to determine how labels are formatted
    pub scale_type: crate::scale::Scale,
    /// The unique data values (domain) that need to be represented in the legend
    pub domain: crate::scale::ScaleDomain,
    
    // Flags to indicate which visual properties are merged into this legend block
    /// If true, the legend will display color swatches
    pub has_color: bool,
    /// If true, the legend will display geometric shape symbols
    pub has_shape: bool,
    /// If true, the legend will display symbols of varying sizes
    pub has_size: bool,
}
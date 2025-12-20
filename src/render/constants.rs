// Common constants used across renderers
pub(crate) mod render_constants {
    // Multi-column legend constants
    pub(crate) const ITEM_HEIGHT: f64 = 20.0;
    pub(crate) const COLOR_BOX_SIZE: f64 = 15.0;
    pub(crate) const COLOR_BOX_SPACING: f64 = 5.0;
    pub(crate) const COLUMN_SPACING: f64 = 20.0; // Space between legend columns
    pub(crate) const LABEL_PADDING: f64 = 10.0;
    pub(crate) const SPACING: f64 = 15.0; // Space between legend/colorbar and axis
    pub(crate) const MAX_ITEMS_PER_COLUMN: usize = 10; // Maximum items per column before creating a new column
}

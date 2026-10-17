pub mod aesthetics;
pub mod composite;
pub mod context;
pub mod conversion;
pub mod data;
/// Legend packing primitives: one flex-wrap implementation shared by all levels.
pub(crate) mod flow;
/// Font resolution shared by the raster (PNG) and PDF backends.
pub(crate) mod font;
pub mod guide;
pub mod layer;
pub mod layout;
pub mod utils;
